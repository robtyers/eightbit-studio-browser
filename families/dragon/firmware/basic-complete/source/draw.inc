; Original guest DRAW and shared bounded macro strings. Apache-2.0.
CMD_DRAW
        ldy     #DRAW_COMMAND
MACRO_EVALUATE
        jsr     EVAL_STRING
        clrb
        ; Native tail continues into the following block.
; Active macro contents remain authoritative. The payload/end are latched at
; entry: later descriptor reassignment does not retarget the running macro.
; A transient linked endpoint bounds the shared native expression scanner.
MACRO_STRING
        pshs    b,x,y,u
        ; Retain the existing conservative collision preflight, including the
        ; bounded source extent, before publishing an active endpoint context.
        tfr     s,d
        subd    #161
        subb    FAC
        sbca    #0
        cmpd    STREND
        lbls    NATIVE_SHARED12_DRAW_INC_0
        leas    -30,s
        leau    16,s
        sty     -12,u
        lda     14,u
        sta     -10,u
        ldd     STR_ROOTS
        std     -16,u
        leay    -16,u
        sty     STR_ROOTS
        ldx     STR_PTR
        clra
        ldb     FAC
        leay    d,x
        sty     -14,u
MACRO_NEXT
        bsr     MACRO_SPACE
        cmpx    -14,u
        beq    MACRO_END
        leax    1,x
        bsr     MACRO_SYNC
        cmpa    #';
        beq    MACRO_NEXT
        jsr     [-12,u]
        bra    MACRO_NEXT
NATIVE_SHARED12_DRAW_INC_0
        jmp     NATIVE_FAIL
MACRO_END
        clra
        clrb
        std     $d9
        inca
        sta     $d8
        ldb     -10,u
        ldx     -16,u
        stx     STR_ROOTS
        leas    15,u
        puls    x,y,u,pc
MACRO_VARIABLE
        pshs    x,u
        ; The published continuation is selected before variable evaluation.
        ; Its first semicolon is structural even inside a quoted subexpression.
MACRO_VARIABLE_SCAN
        cmpx    -14,u
        beq     MACRO_RANGE_ERROR
        lda     ,x+
        cmpa    #';
        bne     MACRO_VARIABLE_SCAN
        bsr     MACRO_SYNC
        puls    x
        jsr     EX_VARIABLE
        puls    u
        ; Machine code may explicitly replace the public continuation/count.
        ldx     $d9
        clra
        ldb     $d8
        leay    d,x
        sty     -14,u
        rts
MACRO_NUMBER
        bsr     MACRO_SPACE
        cmpa    #'+
        beq    MACRO_BAD_CHAR
        cmpa    #'-
        beq    MACRO_BAD_CHAR
        cmpa    #';
        beq    MACRO_BAD_CHAR
        cmpa    #'=
        beq    MACRO_NUMBER_VARIABLE
        jsr     DECIMAL_DIGIT
        bhi    MACRO_DEFAULT
        jsr     PARSE_LINE_NUMBER
        bsr     MACRO_SYNC
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
        bsr     MACRO_SPACE
        jsr     ASCII_SIGN
        negb
        pshs    b
        bsr    MACRO_NUMBER
        bcs    MACRO_RANGE_ERROR
        clra
        lbra    DRAW_SCALE_RESTORE
; The active cursor is the actual payload address. Preserve parser flags and
; result while publishing the same remaining count and current source cursor.
MACRO_SPACE
        jsr     EX_SPACE
MACRO_SYNC
        pshs    cc,d
        stx     $d9
        ldd     -14,u
        subd    $d9
        stb     $d8
        puls    cc,d,pc
MACRO_EXPECT
        pshs    a
        bsr     MACRO_SPACE
        cmpa    ,s+
        beq     MACRO_SKIP
MACRO_BAD_CHAR
        tsta
        beq     MACRO_RANGE_ERROR
        leax    1,x
MACRO_RANGE_ERROR
        bsr     MACRO_SYNC
        jmp     NATIVE_RANGE_ERROR
MACRO_SKIP
        leax    1,x
        bra     MACRO_SYNC
DRAW_COMMAND
        jsr     BYTE_EQUAL_DISPATCH
        fcb     4
        fcb     $42
        fdb     DRAW_BLANK
        fcb     $4e
        fdb     DRAW_STILL
        fcb     $58
        fdb     DRAW_NESTED
        fcb     $4d
        fdb     DRAW_MOVE
        ldy     #DRAW_LETTERS
        ldb     #10
DRAW_FIND
        cmpa    b,y
        beq    DRAW_ARGUMENT
        decb
        bpl    DRAW_FIND
        bra     MACRO_RANGE_ERROR
DRAW_ARGUMENT
        pshs    b
        lbsr    MACRO_NUMBER
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
        ; Native tail continues into the following block.
DRAW_RELATIVE
        ; Signed scale truncates magnitude before restoring sign. Quarter-turn
        ; permutations commute with it, so retain the raw pair until each axis
        ; is scaled, translated and clipped once below.
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
        jsr     MACRO_SPACE
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
        lda     #',
        jsr     MACRO_EXPECT
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
        ; Native tail continues into the following block.
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
DRAW_TRANSLATE
        ldd     -4,u
        bsr     DRAW_SCALE_COMPONENT
        addd    $c7
        ldy     #255
        bsr    DRAW_CLIP
        std     -4,u
        ldd     -2,u
        bsr     DRAW_SCALE_COMPONENT
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
