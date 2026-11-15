; Original native CIRCLE, derived from mathematical and synthetic pixel facts.
; The 64-segment polygon truncates the radius component before aspect scaling.
; Each endpoint clamps before line drawing; reversed arcs wrap through zero.
; U biases a 36-byte stack frame: centre -20/-18, radius -16, ratio -14,
; sign -12, parameter -11, line frame -8..11, cursor 12, angle/end 14/15.
CMD_CIRCLE
        leas    -36,s
        leau    20,s
        jsr     GFX_PAIR
        jsr     GFX_ADDRESS
        ldd     $bd
        std     -20,u
        ldd     $bf
        std     -18,u
        jsr     MULTI_COMMA
        jsr     CIRCLE_POSITIVE
        jsr     FAC_TO_ADDRESS
        jsr     ST_CHECK
        pshs    d
        lda     $b6
        cmpa    #4
        puls    d
        bhs     CIRCLE_RADIUS_READY
        lsra
        rorb
CIRCLE_RADIUS_READY
        std     -16,u
        ldd     #256
        std     -14,u
        ldd     #64
        std     14,u
        clr     -11,u
        ldb     $b2
        stb     $b4
CIRCLE_PARAMETER
        jsr     EX_OPTIONAL_COMMA
        bne     CIRCLE_PARSED
        inc     -11,u
        lda     -11,u
        cmpa    #4
        bhi     CIRCLE_ERROR
        jsr     EX_SPACE
        cmpa    #',
        beq     CIRCLE_PARAMETER
        jsr     EX_END_CHAR
        beq     CIRCLE_PARSED
        jsr     CIRCLE_POSITIVE
        lda     -11,u
        cmpa    #1
        bne     CIRCLE_REAL_PARAMETER
        bsr     MULTI_BYTE
        jsr     GFX_COLOUR_ZERO
        stb     $b4
        bra     CIRCLE_PARAMETER
CIRCLE_REAL_PARAMETER
        cmpa    #2
        beq     CIRCLE_RATIO
        ; The observed arc range is checked after 1/64-turn truncation.
        lda     #6
        bsr     CIRCLE_SCALED_WORD
        cmpd    #64
        bhi     CIRCLE_ERROR
        lda     -11,u
        cmpa    #3
        beq     CIRCLE_START
        stb     15,u
        bra     CIRCLE_PARAMETER
CIRCLE_START
        stb     14,u
        bra     CIRCLE_PARAMETER
CIRCLE_RATIO
        lda     #8
        bsr     CIRCLE_SCALED_WORD
        std     -14,u
        bra     CIRCLE_PARAMETER
CIRCLE_ERROR
        jmp     NATIVE_RANGE_ERROR
CIRCLE_SCALED_WORD
        tst     FAC
        beq     CIRCLE_WORD
        adda    FAC
        bcs     CIRCLE_ERROR
        sta     FAC
CIRCLE_WORD
        jsr     FAC_TO_ADDRESS
        jmp     ST_CHECK
CIRCLE_PARSED
        jsr     CMD_REQUIRE_END
        stx     12,u
        ldb     15,u
        cmpb    14,u
        bhi     CIRCLE_END_READY
        addb    #64
        stb     15,u
CIRCLE_END_READY
        bsr     CIRCLE_POINT
        ldd     -4,u
        std     -8,u
        ldd     -2,u
        std     -6,u
CIRCLE_DRAW
        jsr     GFX_PHYSICAL_LINE
        ldb     14,u
        cmpb    15,u
        beq     CIRCLE_DONE
        inc     14,u
        bsr     CIRCLE_POINT
        bra     CIRCLE_DRAW
CIRCLE_DONE
        ; The equal endpoint comparison already cleared carry.
        ldx     12,u
        leas    36,s
        rts
CIRCLE_POINT
        ldb     14,u
        addb    #16
        bsr     CIRCLE_COMPONENT
        bsr     CIRCLE_SIGN
        addd    -20,u
        ldy     #255
        pshs    d
        lda     $b6
        cmpa    #4
        puls    d
        bhs     CIRCLE_X_LIMIT
        leay    -128,y
CIRCLE_X_LIMIT
        jsr     DRAW_CLIP
        std     -4,u
        ldb     14,u
        bsr     CIRCLE_COMPONENT
        ldx     -14,u
        jsr     PRODUCT_16
        ; Divide the complete unsigned product by 256, saturating only values
        ; already beyond every visible offset. No signed overflow can wrap in.
        cmpy    #1
        bhi     CIRCLE_Y_OUTSIDE
        stb     -11,u
        pshs    a
        tfr     y,d
        tfr     b,a
        ldb     ,s+
        pshs    a
        lda     $b6
        bita    #2
        puls    a
        beq     CIRCLE_Y_SIGN
        lsl     -11,u
        rolb
        rola
        bra     CIRCLE_Y_SIGN
CIRCLE_Y_OUTSIDE
        ldd     #512
CIRCLE_Y_SIGN
        bsr     CIRCLE_SIGN
        addd    -18,u
        ldy     #191
        pshs    d
        lda     $b6
        cmpa    #2
        puls    d
        bhs     CIRCLE_Y_LIMIT
        leay    -96,y
CIRCLE_Y_LIMIT
        jsr     DRAW_CLIP
        std     -2,u
        rts
CIRCLE_SIGN
        cmpd    #512
        blo     CIRCLE_SIGNED
        ldd     #512
CIRCLE_SIGNED
        tst     -12,u
        bpl     CIRCLE_SIGN_END
        jsr     NEGATE_D
CIRCLE_SIGN_END
        rts
; B is a 1/64-turn angle. Return the unsigned, truncated radius component.
CIRCLE_COMPONENT
        tfr     b,a
        lsla
        lsla
        sta     -12,u
        andb    #31
        cmpb    #16
        bls     CIRCLE_QUADRANT
        negb
        addb    #32
CIRCLE_QUADRANT
        cmpb    #16
        beq     CIRCLE_CARDINAL
        lslb
        ldx     #CIRCLE_QUARTER
        ldx     b,x
        ldd     -16,u
        bsr     PRODUCT_16
        tfr     y,d
        rts
CIRCLE_CARDINAL
        ldd     -16,u
        rts
CIRCLE_QUARTER
        ; floor(65536*sin(n*pi/32)), n=0..15; the cardinal is exact.
        fdb     0,6423,12785,19024,25079,30893,36409,41575
        fdb     46340,50660,54491,57797,60547,62714,64276,65220
CIRCLE_POSITIVE
        jsr     EVAL_NUMBER
        tst     FSIGN
        bmi     NATIVE_RANGE_ERROR
        rts
; Unsigned D*X -> Y:D, preserving X/U. Sixteen bounded native steps retain
; the full 32-bit product; no floating-point or persistent scratch is involved.
PRODUCT_16
        pshs    d,x
        ldy     #0
        ldx     #16
PRODUCT_16_BIT
        ldb     1,s
        lsrb
        tfr     y,d
        bcc     PRODUCT_16_SHIFT
        addd    2,s
PRODUCT_16_SHIFT
        rora
        rorb
        ror     ,s
        ror     1,s
        tfr     d,y
        leax    -1,x
        bne     PRODUCT_16_BIT
        puls    d,x,pc

