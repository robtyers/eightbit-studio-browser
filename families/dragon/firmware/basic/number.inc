; Original ASCII-to-MBF conversion and numeric comparison. Apache-2.0.
; Decimal input X; FAC result, X advanced. The caller handles a leading sign.
NUMBER_PARSE
        clr     $06
        pshs    y,u
        ; Seen digits, fractional flag, signed decimal scale, exponent sign.
        leas    -5,s
        tfr     s,u
        clra
        clrb
        std     ,u
        std     2,u
        sta     4,u
        jsr     FAC_ZERO
NUMBER_DIGIT
        jsr     DECIMAL_DIGIT
        bhi     NUMBER_NOT_DIGIT
        leax    1,x
        pshs    a
        inc     ,u
        beq     NUMBER_DIGIT_OVERFLOW
        tst     1,u
        beq     NUMBER_INTEGER_DIGIT
        ldd     2,u
        subd    #1
        std     2,u
NUMBER_INTEGER_DIGIT
        pshs    x
        jsr     FP_RUN
        fcb     FP_KARG+2
        fcb     FP_MUL
        fcb     FP_END
        bcs     PARSE_FAILURE_16
        ldb     2,s
        jsr     ARG_DIGIT
        jsr     F_ADD
        bcs     PARSE_FAILURE_16
        puls    x
        leas    1,s
        bra     NUMBER_DIGIT
NUMBER_DIGIT_OVERFLOW
        leas    1,s
PARSE_FAILURE_15
        jmp     NUMBER_ERROR
PARSE_FAILURE_16
        jmp     NUMBER_MATH_ERROR
NUMBER_NOT_DIGIT
        adda    #'0
        cmpa    #'.
        bne     NUMBER_EXPONENT
        tst     1,u
        bne     NUMBER_EXPONENT
        inc     1,u
        leax    1,x
        bra     NUMBER_DIGIT
NUMBER_EXPONENT
        tst     ,u
        beq     PARSE_FAILURE_15
        anda    #$df
        cmpa    #'E
        bne     NUMBER_SCALE_SETUP
NUMBER_EXP_START
        jsr     EX_NEXT_SPACE
        cmpa    #'-
        beq     NUMBER_EXP_NEGATIVE
        cmpa    #$c4
        beq     NUMBER_EXP_NEGATIVE
        cmpa    #'+
        beq     NUMBER_EXP_SIGN_END
        cmpa    #$c3
        bne     NUMBER_EXP_DIGIT_START
        bra     NUMBER_EXP_SIGN_END
NUMBER_EXP_NEGATIVE
        inc     4,u
NUMBER_EXP_SIGN_END
        leax    1,x
NUMBER_EXP_DIGIT_START
        ; Reuse the bounded native word scanner without changing a caller's
        ; pending line-number workspace. Only data, never parser code, is shared.
        ldd     LINE_NUMBER
        pshs    d
        jsr     PARSE_LINE_NUMBER
        puls    y
        sty     LINE_NUMBER
        bcs     NUMBER_ERROR
        cmpd    #255
        bhi     NUMBER_ERROR
        tst     4,u
        beq     NUMBER_HAVE_EXPONENT
        jsr     NEGATE_D
NUMBER_HAVE_EXPONENT
        addd    2,u
        std     2,u
NUMBER_SCALE_SETUP
NUMBER_SCALE
        tst     FAC
        beq     NUMBER_COMPLETE
        ldd     2,u
        beq     NUMBER_COMPLETE
        bmi     NUMBER_SCALE_DOWN
        cmpd    #128
        bhi     NUMBER_ERROR
        jsr     FP_RUN
        fcb     FP_KARG+2
        fcb     FP_MUL
        fcb     FP_END
        bcs     NUMBER_ERROR
        ldd     2,u
        subd    #1
        bra     NUMBER_SAVE_SCALE
NUMBER_SCALE_DOWN
        cmpd    #-128
        ble     NUMBER_UNDERFLOW
        jsr     FP_RUN
        fcb     FP_KARG+2
        fcb     FP_DIV
        fcb     FP_END
        bcs     NUMBER_ERROR
        ldd     2,u
        addd    #1
NUMBER_SAVE_SCALE
        std     2,u
        bra     NUMBER_SCALE
NUMBER_UNDERFLOW
        jsr     FAC_ZERO
NUMBER_COMPLETE
        leas    5,s
NATIVE_TAIL_9
        clra
        puls    y,u,pc
NUMBER_MATH_ERROR
        puls    x
        leas    1,s
NUMBER_ERROR
        leas    5,s
NATIVE_TAIL_6
        coma
        puls    y,u,pc
; Signed comparison, FAC versus ARG. A = -1/0/+1, Z indicates equality.
F_COMPARE
        tst     FAC
        beq     F_COMPARE_ZERO_LEFT
        tst     ARG
        beq     F_COMPARE_SIGN
        lda     FSIGN
        eora    ASIGN
        bmi     F_COMPARE_SIGN
        ldx     #FAC
        ldy     #ARG
        ldb     #5
F_COMPARE_MANTISSA
        jsr     COMPARE_BYTES
        bne     F_COMPARE_MAGNITUDE
F_COMPARE_EQUAL
        clra
        rts
F_COMPARE_MAGNITUDE
        ; Borrow gives a zero/FF mask; a negative magnitude reverses it.
        lda     #0
        sbca    #0
        tst     FSIGN
        bpl     F_COMPARE_RESULT
        coma
F_COMPARE_RESULT
        ora     #1
        rts
F_COMPARE_ZERO_LEFT
        tst     ARG
        beq     F_COMPARE_EQUAL
        lda     ASIGN
        coma
        bra     F_COMPARE_SIGN_BYTE
F_COMPARE_SIGN
        lda     FSIGN
F_COMPARE_SIGN_BYTE
        asla
        lda     #0
        sbca    #0
        bra     F_COMPARE_RESULT

; B is one decimal digit. FAC and the token cursor remain intact.
ARG_DIGIT
        clra
        tstb
        beq     ARG_DIGIT_STORE
        lda     #136
ARG_DIGIT_NORMALISE
        deca
        lslb
        bpl     ARG_DIGIT_NORMALISE
ARG_DIGIT_STORE
        std     ARG
        andb    #$7f
        stb     ASIGN
        clr     AMANT+1
        clr     AMANT+2
        clr     AMANT+3
        rts

; Shared native digit scanner. Spaces are insignificant inside Dragon numbers.
; A is 0..9 on LS; X stops at the digit or first other non-space byte.
DECIMAL_DIGIT
        jsr     EX_SPACE
        suba    #'0
        cmpa    #9
        rts

; Contiguous six-byte constants avoid a second ROM pointer table.
FP_CONSTANTS
NUMBER_ONE
        fcb     129,$80,0,0,0,0
NUMBER_TWO
        fcb     130,$80,0,0,0,0
NUMBER_TEN
        fcb     132,$a0,0,0,0,$20
NUMBER_LN2
        fcb     128,$b1,$72,$17,$f8,$31
NUMBER_HALF
        fcb     128,$80,0,0,0,0
NUMBER_PI
        fcb     130,$c9,$0f,$da,$a2,$49
NUMBER_TWO_PI
        fcb     131,$c9,$0f,$da,$a2,$49
NUMBER_HALF_PI
        fcb     129,$c9,$0f,$da,$a2,$49
NUMBER_QUARTER_PI
        fcb     128,$c9,$0f,$da,$a2,$49

; B is the already parsed ASCII minus flag. Carry remains the decimal parser's
; status; a signed zero retains the same sign byte as DATA and VAL previously.
NUMBER_SIGNED
        pshs    b
        jsr     NUMBER_PARSE
        puls    b
        bcs     NUMBER_SIGNED_END
        tstb
        beq     NUMBER_SIGNED_END
        jmp     F_NEGATE
NUMBER_SIGNED_END
        rts

