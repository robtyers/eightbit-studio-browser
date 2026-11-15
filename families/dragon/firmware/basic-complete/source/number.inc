; Original ASCII-to-MBF conversion and numeric comparison. Apache-2.0.
; Decimal input X; FAC result, X advanced. The caller handles a leading sign.
; Y/U are private workspace; expression and DATA callers own their saved pair.
; VAL restores its text scope from its fixed stack frame after this call.
NUMBER_PARSE
        clr     $06
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
        fcb     FP_KARG+FPC_TEN
        fcb     FP_MUL
        fcb     FP_END
        bcs     PARSE_FAILURE_16
NUMBER_DIGIT_PREPARE
        ; Retain the rounded running value in ARG. The existing integer
        ; converter supplies the small positive digit in FAC; addition is
        ; commutative, so no second integer normaliser is necessary.
        jsr     F_SWAP
        ldb     2,s
        clra
        ; FP_RUN has just supplied positive NUMBER_TEN in ARG. Swapping
        ; leaves its positive sign in FAC; NUMBER_PARSE already owns type zero.
        ; The positive integer entry therefore needs neither repeated reset.
        jsr     INT_POSITIVE
NUMBER_DIGIT_READY
        jsr     F_ADD
        bcs     PARSE_FAILURE_16
        puls    x
        leas    1,s
        bra     NUMBER_DIGIT
NUMBER_DIGIT_OVERFLOW
        leas    1,s
PARSE_FAILURE_15
        bra     NUMBER_ERROR
PARSE_FAILURE_16
        ; Native tail continues into the following block.
NUMBER_MATH_ERROR
        puls    x
        leas    1,s
NUMBER_ERROR
        leas    5,s
NATIVE_TAIL_6
        coma
        rts
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
        tst     FAC
        beq     NUMBER_COMPLETE
        ldd     2,u
        cmpd    #-128
        ble     NUMBER_UNDERFLOW
        cmpd    #127
        bgt     NUMBER_ERROR
        jsr     SCALE_DECIMAL
        bcs     NUMBER_ERROR
        bra     NUMBER_COMPLETE

NUMBER_UNDERFLOW
        jsr     FAC_ZERO
NUMBER_COMPLETE
        leas    5,s
NATIVE_TAIL_9
        clra
        rts
; Signed comparison, FAC versus ARG. A = -1/0/+1, Z indicates equality.
F_COMPARE
        tst     FAC
        beq     F_COMPARE_ZERO_LEFT
        tst     ARG
        beq     F_COMPARE_SIGN
        lda     FSIGN
        eora    ASIGN
        bmi     F_COMPARE_SIGN
F_COMPARE_MANTISSA
        jsr     F_COMPARE_UNSIGNED
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

; Shared magnitude ordering for comparison and arithmetic. Both inputs are
; nonzero; their exponent followed by the complete significand is a lexical
; unsigned key. Sign bytes are outside the five-byte spans.
F_COMPARE_UNSIGNED
        ldx     #FAC
        ldy     #ARG
        ldb     #5
        jmp     COMPARE_BYTES

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
NUMBER_TEN
        fcb     132,$a0,0,0,0,$20
NUMBER_LN2
        fcb     128,$b1,$72,$17,$f8,$31
NUMBER_HALF
        fcb     128,$80,0,0,0,0
NUMBER_TWO_PI
        fcb     131,$c9,$0f,$da,$a2,$49
NUMBER_HALF_PI
        fcb     129,$c9,$0f,$da,$a2,$49

NUMBER_LOG2_E
        fcb     $81,$b8,$aa,$3b,$29,$38

; Independently rounded mathematical sqrt(1/2), compact arithmetic dictionary.
NUMBER_SQRT_HALF
        fcb     $80,$b5,$04,$f3,$34,$35

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

; Scale FAC by signed B decimal powers. Only transient guest-stack data is
; used; the common FP operations preserve the exact rounding at each step.
SCALE_DECIMAL
        pshs    b
SCALE_DECIMAL_NEXT
        tst     FAC
        beq     SCALE_DECIMAL_DONE
        tst     ,s
        beq     SCALE_DECIMAL_DONE
        bmi     SCALE_DECIMAL_DOWN
        jsr     FP_RUN
        fcb     FP_KARG+FPC_TEN,FP_MUL,FP_END
        bcs     SCALE_DECIMAL_RETURN
        dec     ,s
        bra     SCALE_DECIMAL_NEXT
SCALE_DECIMAL_DOWN
        jsr     FP_RUN
        fcb     FP_KARG+FPC_TEN,FP_DIV,FP_END
        bcs     SCALE_DECIMAL_RETURN
        inc     ,s
        bra     SCALE_DECIMAL_NEXT
SCALE_DECIMAL_DONE
        clra
SCALE_DECIMAL_RETURN
        puls    b,pc
