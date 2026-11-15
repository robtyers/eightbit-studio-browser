; Original decimal presentation for packed BASIC numbers. Apache-2.0.
; Nine significant decimal digits; guest arithmetic, no host conversion.
; LIST, tracing and line diagnostics use the same exact digit conversion,
; omitting only PRINT's surrounding spaces. Their FAC is not a live value.
PRINT_UNSIGNED
        pshs    d,x,y,u
        ; Feed the integer directly to BCD at exponent 160. Normalising it
        ; first would require up to 31 subsequent decimal halvings.
        std     FMANT+2
        clra
        clrb
        std     FMANT
        sta     FSIGN
        lda     #160
        sta     FAC
        bra     FORMAT_ENTRY
PRINT_NUMBER
        pshs    d,x,y,u
        ldb     #1
FORMAT_ENTRY
        pshs    b
        leax    -83,s
        cmpx    STREND
        lbls    NATIVE_FAIL
        ; Ten display digits plus an exact fixed-point decimal accumulator:
        ; 40 integer and 50 fractional digits, two digits per byte. Repeated
        ; halving truncates only below 10^-50, beyond every tenth significant
        ; digit of a normalised five-byte number (minimum 2^-128).
        leas    -59,s
        clr     ,s
        clr     1,s
        tst     59,s
        beq     FORMAT_VALUE
        lda     #32
        tst     FSIGN
        bpl     FORMAT_SIGN
        lda     #'-
FORMAT_SIGN
        jsr     FORMAT_OUT
FORMAT_VALUE
        tst     FAC
        beq     FORMAT_ZERO
        ldd     FMANT
        bne     FORMAT_CONVERT
        ldd     FMANT+2
        bne     FORMAT_CONVERT
FORMAT_ZERO
        lda     #'0
        jsr     FORMAT_OUT
        jmp     FORMAT_FINISH
FORMAT_CONVERT
        leau    14,s
        tfr     u,x
        ldb     #45
FORMAT_CLEAR_BCD
        clr     ,x+
        decb
        bne     FORMAT_CLEAR_BCD
        lda     #32
        sta     2,s
FORMAT_BINARY_BIT
        lsl     FMANT+3
        rol     FMANT+2
        rol     FMANT+1
        rol     FMANT
        jsr     FORMAT_DOUBLE
        dec     2,s
        bne     FORMAT_BINARY_BIT
        ldb     FAC
        subb    #160
        bcs     FORMAT_SCALE_DOWN
        stb     2,s
        beq     FORMAT_EXTRACT
FORMAT_SCALE_UP
        clra
        jsr     FORMAT_DOUBLE
        dec     2,s
        bne     FORMAT_SCALE_UP
        bra     FORMAT_EXTRACT
FORMAT_SCALE_DOWN
        negb
        stb     2,s
FORMAT_HALVE
        tfr     u,x
        ldb     #45
        clra
FORMAT_HALF_BYTE
        lda     ,x
        rora
        pshs    cc
        bpl     FORMAT_HALF_TENS
        suba    #$30
FORMAT_HALF_TENS
        bita    #8
        beq     FORMAT_HALF_STORE
        suba    #3
FORMAT_HALF_STORE
        sta     ,x+
        puls    cc
        decb
        bne     FORMAT_HALF_BYTE
        dec     2,s
        bne     FORMAT_HALVE
FORMAT_EXTRACT
        tfr     u,x
        leay    4,s
        lda     #40
        sta     ,s
FORMAT_LEADING_DIGIT
        jsr     FORMAT_BCD_DIGIT
        dec     ,s
        tsta
        beq     FORMAT_LEADING_DIGIT
        ldb     #10
FORMAT_DIGIT
        adda    #'0
        sta     ,y+
        decb
        beq     FORMAT_DIGITS_READY
        jsr     FORMAT_BCD_DIGIT
        bra     FORMAT_DIGIT
FORMAT_DIGITS_READY
        clr     1,s
        lda     13,s
        cmpa    #'5
        blo     FORMAT_TRIM
        leax    13,s
        ldb     #9
FORMAT_ROUND
        inc     ,-x
        lda     ,x
        cmpa    #':
        blo     FORMAT_TRIM
        lda     #'0
        sta     ,x
        decb
        bne     FORMAT_ROUND
FORMAT_ROUND_CARRY
        lda     #'1
        sta     ,x
        inc     ,s
FORMAT_TRIM
        leax    13,s
        ldb     #9
FORMAT_TRIM_NEXT
        cmpb    #1
        beq     FORMAT_LAYOUT
        lda     ,-x
        cmpa    #'0
        bne     FORMAT_LAYOUT
        decb
        bra     FORMAT_TRIM_NEXT
FORMAT_LAYOUT
        stb     2,s
        leax    4,s
        lda     ,s
        cmpa    #9
        bge     FORMAT_SCIENTIFIC
        cmpa    #-2
        blt     FORMAT_SCIENTIFIC
        inca
        bra     FORMAT_POINT
FORMAT_SCIENTIFIC
        inc     1,s
        lda     #1
FORMAT_POINT
        sta     3,s
        bgt     FORMAT_DIGITS
        lda     #'.
        bsr     FORMAT_OUT
FORMAT_LEADING_ZERO
        tst     3,s
        beq     FORMAT_FRACTION_READY
        lda     #'0
        bsr     FORMAT_OUT
        inc     3,s
        bra     FORMAT_LEADING_ZERO
FORMAT_FRACTION_READY
        dec     3,s
FORMAT_DIGITS
        tst     3,s
        bne     FORMAT_CHARACTER
        lda     #'.
        bsr     FORMAT_OUT
FORMAT_CHARACTER
        lda     #'0
        tst     2,s
        beq     FORMAT_CHARACTER_OUT
        lda     ,x+
        dec     2,s
FORMAT_CHARACTER_OUT
        bsr     FORMAT_OUT
        dec     3,s
        tst     2,s
        bne     FORMAT_DIGITS
        tst     3,s
        bgt     FORMAT_DIGITS
        tst     1,s
        beq     FORMAT_FINISH
FORMAT_EXPONENT
        lda     #'E
        bsr     FORMAT_OUT
        lda     #'+
        tst     ,s
        bpl     FORMAT_EXP_SIGN
        lda     #'-
FORMAT_EXP_SIGN
        bsr     FORMAT_OUT
        ldb     ,s
        bpl     FORMAT_EXP_ABS
        negb
FORMAT_EXP_ABS
        lda     #'0
FORMAT_EXP_TENS
        cmpb    #10
        blo     FORMAT_EXP_OUT
        subb    #10
        inca
        bra     FORMAT_EXP_TENS
FORMAT_EXP_OUT
        bsr     FORMAT_OUT
        tfr     b,a
        adda    #'0
        bsr     FORMAT_OUT
FORMAT_FINISH
        tst     59,s
        beq     FORMAT_RETURN
        lda     #32
        bsr     FORMAT_OUT
FORMAT_RETURN
        leas    60,s
        puls    d,x,y,u,pc

FORMAT_OUT
        jmp     CHROUT


; A is the current raw ASCII character. B reports a consumed minus sign;
; X advances only over a sign. DATA and VAL retain their own number rules.
ASCII_SIGN
        clrb
        cmpa    #'+
        beq     ASCII_SIGN_NEXT
        cmpa    #'-
        bne     ASCII_SIGN_END
ASCII_MINUS
        incb
ASCII_SIGN_NEXT
        leax    1,x
ASCII_SIGN_END
        rts


