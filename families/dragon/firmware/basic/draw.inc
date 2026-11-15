; Original guest DRAW and shared bounded macro strings. Apache-2.0.
CMD_DRAW
        ldy     #DRAW_COMMAND
MACRO_EVALUATE
        jsr     EVAL_NATIVE
        jsr     EX_STRING_CHECK
        jsr     CMD_REQUIRE_END
        clrb
        bra    MACRO_STRING
NATIVE_SHARED12_DRAW_INC_0
        jmp     NATIVE_FAIL
; Copy each bounded string to the guest stack so nested variable evaluation
; cannot invalidate the active text through string collection. No heap reserve.
MACRO_STRING
        pshs    b,x,y,u
        jsr     STACK_STRING
        bcs     NATIVE_SHARED12_DRAW_INC_0
        leax    -154,s
        cmpx    STREND
        bls     NATIVE_SHARED12_DRAW_INC_0
        tfr     y,d
        leas    -26,s
        leax    14,s
        std     -14,x
        ldd     5,u
        std     -12,x
        ldb     2,u
        tfr     x,u
        leax    26,s
        stb     -10,u
MACRO_NEXT
        jsr     EX_SPACE
        cmpx    -14,u
        beq    MACRO_END
        leax    1,x
        cmpa    #';
        beq    MACRO_NEXT
        jsr     [-12,u]
        bra    MACRO_NEXT
MACRO_END
        ldb     -10,u
        lds     -14,u
        leas    4,s
        puls    x,y,u,pc
MACRO_VARIABLE
        pshs    u
        jsr     EX_VARIABLE
        puls    u
        lda     #';
        jmp     EX_EXPECT
MACRO_NUMBER
        jsr     EX_SPACE
        cmpa    #'+
        beq    MACRO_RANGE_ERROR
        cmpa    #'-
        beq    MACRO_RANGE_ERROR
        cmpa    #';
        beq    MACRO_RANGE_ERROR
        cmpa    #'=
        beq    MACRO_NUMBER_VARIABLE
        jsr     DECIMAL_DIGIT
        bhi    MACRO_DEFAULT
        jsr     PARSE_LINE_NUMBER
        bcs     NATIVE_SHARED12_DRAW_INC_0
        tsta
        bne    MACRO_RANGE_ERROR
        rts
MACRO_NUMBER_VARIABLE
        leax    1,x
        bsr    MACRO_VARIABLE
        jmp     MULTI_BYTE
MACRO_DEFAULT
        ldb     #1
        coma
        rts
MACRO_SIGNED
        jsr     EX_SPACE
        jsr     ASCII_SIGN
        negb
        pshs    b
        bsr    MACRO_NUMBER
        bcs    MACRO_RANGE_ERROR
        clra
        lbra    DRAW_SCALE_RESTORE
MACRO_RANGE_ERROR
        jmp     NATIVE_RANGE_ERROR
DRAW_COMMAND
        cmpa    #'B
        beq    DRAW_BLANK
        cmpa    #'N
        beq    DRAW_STILL
        cmpa    #'X
        beq    DRAW_NESTED
        cmpa    #'M
        lbeq    DRAW_MOVE
        ldy     #DRAW_LETTERS
        ldb     #10
DRAW_FIND
        cmpa    b,y
        beq    DRAW_ARGUMENT
        decb
        bpl    DRAW_FIND
        jmp     NATIVE_FAIL
DRAW_ARGUMENT
        pshs    b
        bsr    MACRO_NUMBER
        puls    a
        cmpa    #8
        blo    DRAW_DIRECTION
        beq    DRAW_SCALE
        cmpa    #9
        beq    DRAW_ANGLE
        jsr     GFX_COLOUR
        stb     $b2
        rts
DRAW_SCALE
        cmpb    #62
        bhi    MACRO_RANGE_ERROR
        stb     $e9
        rts
DRAW_ANGLE
        cmpb    #3
        bhi    MACRO_RANGE_ERROR
        stb     $e8
        rts
DRAW_BLANK
        lda     #1
        bra    DRAW_FLAG
DRAW_STILL
        lda     #2
DRAW_FLAG
        ora     -10,u
        sta     -10,u
        rts
DRAW_NESTED
        jsr     MACRO_VARIABLE
        jsr     EX_STRING_CHECK
        ldy     -12,u
        ldb     -10,u
        jsr     MACRO_STRING
        stb     -10,u
        rts
DRAW_LETTERS
        fcc     "UERFDGLHSAC"
DRAW_DIRECTION
        pshs    a
        anda    #1
        sta     -9,u
        clra
        std     0,u
        ldb     ,s
        bsr     DRAW_COMPONENT
        std     -4,u
        puls    b
        subb    #2
        bsr     DRAW_COMPONENT
        std     -2,u
        bra     DRAW_RELATIVE
; Compass components: 0/4 have zero X, 1..3 positive, 5..7 negative.
DRAW_COMPONENT
        bitb    #3
        beq    DRAW_ZERO
        lsrb
        lsrb
        lsrb
        ldd     0,u
        bcc    DRAW_COMPONENT_END
        jmp     NEGATE_D
DRAW_ZERO
        clra
        clrb
DRAW_COMPONENT_END
        rts
DRAW_MOVE
        sta     -9,u
        jsr     EX_SPACE
        clr     2,u
        cmpa    #'+
        beq    DRAW_MOVE_RELATIVE
        cmpa    #'-
        bne    DRAW_MOVE_FIRST
DRAW_MOVE_RELATIVE
        inc     2,u
DRAW_MOVE_FIRST
        jsr     MACRO_SIGNED
        std     -4,u
        jsr     MULTI_COMMA
        jsr     MACRO_SIGNED
        std     -2,u
        tst     2,u
        bne    DRAW_RELATIVE
        ; Absolute coordinates clamp unsigned; a negative second coordinate
        ; therefore clips at the upper bound, matching the observed interface.
        ldd     -2,u
        cmpd    #191
        bls    DRAW_ABSOLUTE_Y
        ldd     #191
DRAW_ABSOLUTE_Y
NATIVE_TAIL_10
        std     -2,u
        bra    DRAW_RENDER
DRAW_RELATIVE
        ldd     -4,u
        bsr    DRAW_SCALE_COMPONENT
        std     -4,u
        ldd     -2,u
        bsr    DRAW_SCALE_COMPONENT
        std     -2,u
        ldb     $e8
        stb     2,u
DRAW_ROTATE
        tst     2,u
        beq    DRAW_TRANSLATE
        ldd     -4,u
        pshs    d
        ldd     -2,u
        jsr     NEGATE_D
        std     -4,u
        puls    d
        std     -2,u
        dec     2,u
        bra    DRAW_ROTATE
DRAW_TRANSLATE
        ldd     -4,u
        addd    $c7
        ldy     #255
        bsr    DRAW_CLIP
        std     -4,u
        ldd     -2,u
        addd    $c9
        ldy     #191
        bsr    DRAW_CLIP
        bra     NATIVE_TAIL_10

DRAW_SCALE_COMPONENT
        pshs    a
        tsta
        bpl    DRAW_SCALE_MAGNITUDE
        jsr     NEGATE_D
DRAW_SCALE_MAGNITUDE
        lda     $e9
        bne    DRAW_SCALE_MULTIPLY
        lda     #4
DRAW_SCALE_MULTIPLY
        mul
        lsra
        rorb
        lsra
        rorb
DRAW_SCALE_RESTORE
        tst     ,s+
        bpl    DRAW_SCALE_END
        jsr     NEGATE_D
DRAW_SCALE_END
        rts
DRAW_CLIP
        tsta
        lbmi    DRAW_ZERO
        pshs    y
        cmpd    ,s++
        bls    DRAW_SCALE_END
        tfr     y,d
        rts
DRAW_RENDER
        ldd     $c7
        std     -8,u
        ldd     $c9
        std     -6,u
        lda     -10,u
        bita    #1
        bne    DRAW_BLANK_CURSOR
        ldb     $b2
        stb     $b4
        jsr     GFX_LINE
        lda     -9,u
        bra     DRAW_CURSOR
DRAW_BLANK_CURSOR
        clra
DRAW_CURSOR
        ; Axial/blank moves retain the previous physical cursor; rendered
        ; diagonals and M commands leave the parsed target in that workspace.
        ldy     #$c7
        tsta
        beq     DRAW_WORKSPACE
        leay    -4,u
DRAW_WORKSPACE
        ldd     ,y
        std     $c3
        ldd     2,y
        std     $c5
        jsr     GFX_ADDRESS
        lda     -10,u
        bita    #2
        bne    DRAW_DONE
        ldd     -4,u
        std     $c7
        ldd     -2,u
        std     $c9
DRAW_DONE
        clr     -10,u
        rts

