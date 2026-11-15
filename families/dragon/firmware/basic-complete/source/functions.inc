; Original bounded real-number routines. Apache-2.0.
; Polynomial evaluation uses only guest 32-bit-significand arithmetic.
; Private algorithm calls borrow Y/U; FP_RUN or the expression owner retains
; its live continuation. Fixed public entries and callback adapters are separate.
FAC_UNPACKED_STORE
        tfr     x,y
        ldx     #FAC
        bsr     FAC_UNPACKED_COPY
        exg     x,y
        rts
ARG_FROM_ROM
        ldy     #ARG
        bra     FAC_UNPACKED_COPY
FAC_UNPACKED_LOAD
        ldy     #FAC
FAC_UNPACKED_COPY
        ldb     #6
COPY_BYTES
FAC_UNPACKED_READ
        lda     ,x+
        sta     ,y+
        decb
        bne     FAC_UNPACKED_READ
        rts

; Truncate whole fractional bytes first, then mask the final partial byte.
; FAC and ARG retain their existing packed/unpacked 32-bit ABI.
F_FIX_VALUE
        lda     FAC
        cmpa    #129
        lblo    FAC_ZERO
        suba    #160
        lbhs    F_FINISH
        ldx     #FMANT+4
        ldb     #$ff
F_FIX_BYTES
        adda    #8
        bpl     F_FIX_PARTIAL
        clr     ,-x
        bra     F_FIX_BYTES
F_FIX_PARTIAL
        suba    #8
F_FIX_MASK
        lslb
        inca
        bne     F_FIX_MASK
        andb    ,-x
        stb     ,x
        jmp     F_FINISH

F_INT_VALUE
        leas    -6,s
        bsr     NATIVE_ISLAND_FUNCTIONS_INC_1
        fcb     FP_STORE+0,FP_END
        bsr     F_FIX_VALUE
        tst     5,s
        bpl     F_INT_DONE
        leax    ,s
        bsr     ARG_FROM_ROM
        jsr     F_COMPARE
        beq     F_INT_DONE
        bsr     NATIVE_ISLAND_FUNCTIONS_INC_1
        fcb     FP_KARG+FPC_ONE
        fcb     FP_SUB
        fcb     FP_END
F_INT_DONE
        leas    6,s
        rts

F_EXP
        ; The observed EXP range rejects either sign at exponent 255.
        lda     FAC
        inca
        beq     LOCAL_FAILURE_18
        leas    -8,s
        jsr     FP_RUN
        fcb     FP_KARG+FPC_LOG2_E,FP_MUL,FP_STORE+0,FP_END
        bcs     F_EXP_FAIL
        jsr     F_INT_VALUE
        jsr     FAC_TO_INT
        bcs     F_EXP_INPUT_RANGE
        std     6,s
        jsr     FP_RUN
        fcb     FP_SWAP,FP_LOAD+0,FP_SUB,FP_END
        bcs     F_EXP_FAIL
        ldx     #EXP_POLYNOMIAL
        jsr     F_POLYNOMIAL
        bcs     F_EXP_FAIL
        clra
        ldb     FAC
        addd    6,s
        jsr     F_EXPONENT
        bcs     F_EXP_FAIL
        ldb     FAC
        cmpb    #2
        blo     F_EXP_RETURN_ZERO
        jsr     F_FINISH
F_EXP_END
        leas    8,s
        rts
NATIVE_ISLAND_FUNCTIONS_INC_1
        jmp     FP_RUN
LOCAL_FAILURE_18
        coma
        rts
F_EXP_INPUT_RANGE
        tst     5,s
        bpl     F_EXP_FAIL
F_EXP_RETURN_ZERO
        jsr     FAC_ZERO
        bra     F_EXP_END
F_EXP_FAIL
        coma
        bra     F_EXP_END

; X selects an authored degree >= 1 and descending packed coefficients.
; FAC is the argument/result; X advances beyond the table, U is preserved.
; The argument and counter occupy seven transient bytes, with no new RAM state.
F_POLYNOMIAL
        leas    -7,s
        jsr     FP_RUN
        fcb     FP_STORE+0,FP_END
        ldb     ,x+
        stb     6,s
        jsr     FAC_LOAD
F_POLYNOMIAL_LOOP
        jsr     FP_RUN
        fcb     FP_ARG+0,FP_MUL,FP_SWAP,FP_END
        bcs     F_POLYNOMIAL_END
        jsr     FAC_LOAD
        jsr     FP_RUN
        fcb     FP_ADD,FP_END
        bcs     F_POLYNOMIAL_END
        dec     6,s
        bne     F_POLYNOMIAL_LOOP
F_POLYNOMIAL_END
        leas    7,s
        rts
; Mathematical relative minimax of 2^x on [0,1], degree seven.
; Independently generated from mathematical functions; no reference fitting.
EXP_POLYNOMIAL
        fcb     7
        fcb     $71,$34,$58,$3e,$55
        fcb     $74,$16,$7e,$b3,$1c
        fcb     $77,$2f,$ee,$e3,$85
        fcb     $7a,$1d,$84,$1c,$2a
        fcb     $7c,$63,$59,$58,$0a
        fcb     $7e,$75,$fd,$e7,$c6
        fcb     $80,$31,$72,$18,$10
        fcb     $81,$00,$00,$00,$00

F_LOG
        tst     FAC
        beq     LOCAL_FAILURE_18
        tst     FSIGN
        bmi     LOCAL_FAILURE_18
        leas    -8,s
        lda     FAC
        suba    #128
        tfr     a,b
        sex
        std     6,s
        lda     #128
        sta     FAC
        ; The reciprocal form shares one denominator and leaves no numerator
        ; capture: z = 1 - sqrt(2)/(m + sqrt(1/2)), m in [.5,1).
        jsr     FP_RUN
        fcb     FP_KARG+FPC_SQRT_HALF,FP_ADD,FP_SWAP,FP_KLOAD+FPC_SQRT_HALF,FP_END
        bcs     F_LOG_END
        inc     FAC
        jsr     FP_RUN
        fcb     FP_DIV,FP_KARG+FPC_ONE,FP_SWAP,FP_SUB,FP_STORE+0
        fcb     FP_ARG+0,FP_MUL,FP_END
        bcs     F_LOG_END
        ldx     #LOG_POLYNOMIAL
        jsr     F_POLYNOMIAL
        bcs     F_LOG_END
        jsr     FP_RUN
        fcb     FP_ARG+0,FP_MUL,FP_KARG+FPC_HALF,FP_SUB,FP_STORE+0,FP_END
        bcs     F_LOG_END
        ldd     6,s
        jsr     INT_TO_FAC
        jsr     FP_RUN
        fcb     FP_ARG+0,FP_ADD,FP_KARG+FPC_LN2,FP_MUL,FP_END
F_LOG_END
        leas    8,s
        rts
; Mathematical absolute-minimax odd log2 kernel on the centred interval.
; The degree-three polynomial takes z^2; its result is multiplied by z.
LOG_POLYNOMIAL
        fcb     3
        fcb     $7f,$5e,$56,$cb,$74
        fcb     $80,$13,$9b,$0b,$65
        fcb     $80,$76,$38,$93,$16
        fcb     $82,$38,$aa,$3b,$20

F_SQR
        tst     FAC
        lbeq    FAC_ZERO
        tst     FSIGN
        lbmi    F_RANGE
        ; Shared real-function composition retains the observed SQR rounding.
        jsr     FP_RUN
        fcb     FP_LOG,FP_KARG+FPC_HALF,FP_MUL,FP_EXP,FP_END
        rts

LOCAL_FAILURE_19
F_SIN_ERROR
        leas    8,s
        coma
        rts
F_COS
        bsr     NATIVE_ISLAND_FUNCTIONS_INC_0
        fcb     FP_KARG+FPC_HALF_PI,FP_ADD,FP_END
        bcs     F_TRIG_ERROR
F_SIN
        ; Fractional turns retain the guest's finite phase precision. Floor
        ; gives a nonnegative turn even for negative, sub-unit arguments.
        leas    -8,s
        clr     6,s
        bsr     NATIVE_ISLAND_FUNCTIONS_INC_0
        fcb     FP_KARG+FPC_TWO_PI,FP_DIV,FP_STORE+0,FP_END
        bcs     F_SIN_ERROR
        jsr     F_INT_VALUE
        bsr     NATIVE_ISLAND_FUNCTIONS_INC_0
        fcb     FP_SWAP,FP_LOAD+0,FP_SUB
        fcb     FP_KARG+FPC_HALF,FP_COMPARE,FP_END
        bcs     F_SIN_ERROR
        cmpa    #1
        bne     F_SIN_QUARTER
        bsr     NATIVE_ISLAND_FUNCTIONS_INC_0
        fcb     FP_KARG+FPC_ONE,FP_SWAP,FP_SUB,FP_END
        dec     6,s
F_SIN_QUARTER
        bsr     NATIVE_ISLAND_FUNCTIONS_INC_0
        fcb     FP_KARG+FPC_HALF,FP_END
        dec     ARG
        jsr     F_SUB
        lda     FSIGN
        coma
        anda    #$80
        sta     7,s
        jsr     EX_ABS
        bsr     NATIVE_ISLAND_FUNCTIONS_INC_0
        fcb     FP_KARG+FPC_HALF,FP_END
        dec     ARG
        jsr     F_SWAP
        jsr     F_SUB
        tst     FAC
        beq     F_SIN_NORMALISED
        inc     FAC
        inc     FAC
F_SIN_NORMALISED
        ; The odd kernel's final multiply leaves its original input in ARG.
        ; TAN consumes that retained quarter angle without a second copy.
        jsr     F_SINE_KERNEL
        bcs     F_SIN_ERROR
        tst     6,s
        bpl     F_SIN_POSITIVE
        jsr     F_NEGATE
F_SIN_POSITIVE
        ; The private TAN caller consumes the same normalised quarter angle;
        ; ARG's sign supplies the cosine quadrant, avoiding a second reduction.
        lda     7,s
        sta     ASIGN
        leas    8,s
        rts
F_TAN
        leas    -7,s
        bsr     NATIVE_ISLAND_FUNCTIONS_INC_0
        fcb     FP_SIN,FP_STORE+0,FP_END
        bcs     F_TAN_END
        ; Retain the first error and complete zero before any complementary
        ; evaluation or division, including phases with no fractional bits.
        tst     FAC
        beq     F_TAN_END
        lda     ASIGN
        sta     6,s
        jsr     F_SWAP
        jsr     EX_ABS
        bsr     NATIVE_ISLAND_FUNCTIONS_INC_0
        fcb     FP_KARG+FPC_ONE,FP_SWAP,FP_SUB,FP_END
        jsr     F_SINE_KERNEL
        bcs     F_TAN_END
        tst     6,s
        bpl     F_TAN_DIVIDE
        jsr     F_NEGATE
F_TAN_DIVIDE
        bsr     NATIVE_ISLAND_FUNCTIONS_INC_0
        fcb     FP_ARG+0,FP_SWAP,FP_DIV,FP_END
F_TAN_END
        leas    7,s
        rts
F_TRIG_ERROR
        coma
        rts
NATIVE_ISLAND_FUNCTIONS_INC_0
        jmp     FP_RUN
; Odd relative-minimax sine on the normalised quarter turn [0,1]. The
; coefficient generator uses mathematical sine only, never reference outputs.
F_SINE_KERNEL
        ldx     #SINE_POLYNOMIAL
F_ODD_POLYNOMIAL
        leas    -6,s
        jsr     FP_RUN
        fcb     FP_STORE+0,FP_ARG+0,FP_MUL,FP_END
        bcs     F_SINE_KERNEL_END
        jsr     F_POLYNOMIAL
        bcs     F_SINE_KERNEL_END
        jsr     FP_RUN
        fcb     FP_ARG+0,FP_MUL,FP_END
F_SINE_KERNEL_END
        leas    6,s
        rts
SINE_POLYNOMIAL
        fcb     5
        fcb     $6e,$e6,$1a,$2d,$2e
        fcb     $74,$28,$07,$fb,$f9
        fcb     $79,$99,$68,$89,$01
        fcb     $7d,$23,$35,$df,$e1
        fcb     $80,$a5,$5d,$e7,$28
        fcb     $81,$49,$0f,$da,$a2
F_ATN
        ; Reciprocal/sign reduction is sufficient on the polynomial's [0,1]
        ; domain; no secondary ratio or pending power-series sum is retained.
        leas    -1,s
        lda     FSIGN
        anda    #$80
        sta     ,s
        jsr     EX_ABS
        jsr     FP_RUN
        fcb     FP_KARG+FPC_ONE,FP_COMPARE,FP_END
        cmpa    #1
        bne     F_ATN_REDUCED
        inc     ,s
        jsr     FP_RUN
        fcb     FP_KARG+FPC_ONE,FP_SWAP,FP_DIV,FP_END
        bcs     F_ATN_END
F_ATN_REDUCED
        ldx     #ATN_POLYNOMIAL
        jsr     F_ODD_POLYNOMIAL
        bcs     F_ATN_END
        lda     ,s
        bita    #1
        beq     F_ATN_SIGN
        jsr     FP_RUN
        fcb     FP_SWAP,FP_KLOAD+FPC_HALF_PI,FP_SUB,FP_END
        bcs     F_ATN_END
F_ATN_SIGN
        tst     ,s
        bpl     F_ATN_FINISH
        jsr     F_NEGATE
F_ATN_FINISH
        jsr     F_FINISH
F_ATN_END
        leas    1,s
        rts
; Mathematical odd relative-minimax ATN on [0,1], degree eleven in x*x.
ATN_POLYNOMIAL
        fcb     11
        fcb     $76,$b3,$83,$be,$07
        fcb     $79,$1e,$f4,$a7,$19
        fcb     $7b,$83,$fc,$b0,$25
        fcb     $7c,$0c,$1f,$67,$d8
        fcb     $7c,$de,$53,$cb,$cc
        fcb     $7d,$14,$64,$70,$4f
        fcb     $7d,$b7,$ea,$51,$7b
        fcb     $7d,$63,$30,$88,$7e
        fcb     $7e,$92,$44,$99,$3a
        fcb     $7e,$4c,$cc,$91,$c7
        fcb     $7f,$aa,$aa,$aa,$13
        fcb     $81,$00,$00,$00,$00

; Unsigned lexical comparison of B bytes at X/Y. Equal spans, including empty
; ones, return Z with carry clear; a mismatch retains the byte comparison flags.
; All callers own the bounded spans and choose their own sign/length semantics.
COMPARE_BYTES
        cmpb    #0
        beq     COMPARE_BYTES_END
COMPARE_BYTES_NEXT
        lda     ,x+
        cmpa    ,y+
        bne     COMPARE_BYTES_END
        decb
        bne     COMPARE_BYTES_NEXT
COMPARE_BYTES_END
        rts
