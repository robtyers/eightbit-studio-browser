; Original bounded real-number routines. Apache-2.0.
; Series/Newton iterations use only the guest 32-bit-significand arithmetic.
FAC_UNPACKED_STORE
        ldy     #FAC
        exg     x,y
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

NUMBER_NEG_PI
        fcb     130,$c9,$0f,$da,$a2,$c9
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
        fcb     FP_KARG+0
        fcb     FP_SUB
        fcb     FP_END
F_INT_DONE
        leas    6,s
        rts

F_EXP
        pshs    y,u
        leas    -32,s
        bsr     NATIVE_ISLAND_FUNCTIONS_INC_1
        fcb     FP_STORE+0,FP_KARG+3,FP_DIV,FP_END
        bcs     F_EXP_INPUT_RANGE
        jsr     FAC_TO_INT
        bcs     F_EXP_INPUT_RANGE
        std     30,s
        jsr     INT_TO_FAC
        bsr     NATIVE_ISLAND_FUNCTIONS_INC_1
        fcb     FP_KARG+3,FP_MUL,FP_SWAP,FP_LOAD+0,FP_SUB
        fcb     FP_STORE+6,FP_KLOAD+0,FP_STORE+12,FP_STORE+18,FP_END
        bcs     F_EXP_INPUT_RANGE
        ldd     #$0195
        jsr     F_SERIES
        bcs     F_EXP_FAIL
        clra
        ldb     FAC
        addd    30,s
        jsr     F_EXPONENT
        bcs     F_EXP_FAIL
        tst     FAC
        beq     F_EXP_RETURN_ZERO
        jsr     F_FINISH
F_EXP_END
        leas    32,s
        puls    y,u,pc
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

F_LOG
        tst     FAC
        beq     LOCAL_FAILURE_18
        tst     FSIGN
        bmi     LOCAL_FAILURE_18
        pshs    y,u
        leas    -34,s
        lda     FAC
        suba    #129
        tfr     a,b
        sex
        std     32,s
        lda     #129
        sta     FAC
        jsr     F_UNIT_RATIO
        jsr     F_SERIES_SEED
        ldd     #$0321
        jsr     F_SERIES
        bsr     NATIVE_ISLAND_FUNCTIONS_INC_1
        fcb     FP_KARG+1
        fcb     FP_MUL
        fcb     FP_STORE+24,FP_END
        ldd     32,s
        jsr     INT_TO_FAC
        bsr     NATIVE_ISLAND_FUNCTIONS_INC_1
        fcb     FP_KARG+3
        fcb     FP_MUL
        fcb     FP_ARG+24
        fcb     FP_ADD
        fcb     FP_END
        leas    34,s
        puls    y,u,pc

F_SQR
        tst     FAC
        lbeq    FAC_ZERO
        tst     FSIGN
        lbmi    F_RANGE
        pshs    y,u
        leas    -13,s
        bsr     NATIVE_ISLAND_FUNCTIONS_INC_1
        fcb     FP_STORE+0,FP_END
        lda     FAC
        suba    #129
        asra
        adda    #129
        sta     FAC
        lda     #$80
        sta     FMANT
        clr     FMANT+1
        clr     FMANT+2
        clr     FMANT+3
        clr     FSIGN
        lda     #8
        sta     12,s
F_SQR_ITERATION
        jsr     FP_RUN
        fcb     FP_STORE+6
        fcb     FP_ARG+0
        fcb     FP_SWAP
        fcb     FP_DIV
        fcb     FP_ARG+6
        fcb     FP_ADD
        fcb     FP_KARG+4
        fcb     FP_MUL
        fcb     FP_END
        dec     12,s
        bne     F_SQR_ITERATION
        leas    13,s
        puls    y,u,pc
LOCAL_FAILURE_19
        bra     F_SIN_ERROR
F_COS
        bsr     NATIVE_ISLAND_FUNCTIONS_INC_0
        fcb     FP_KARG+7
        fcb     FP_ADD
        fcb     FP_END
        bcs     F_TRIG_ERROR
        bra     F_SIN
F_TAN
        leas    -12,s
        bsr     NATIVE_ISLAND_FUNCTIONS_INC_0
        fcb     FP_STORE+0,FP_SIN,FP_STORE+6,FP_LOAD+0
        fcb     FP_COS,FP_ARG+6,FP_SWAP,FP_DIV,FP_END
        leas    12,s
        rts
F_TRIG_ERROR
        coma
        rts
F_SIN
        pshs    y,u
        leas    -32,s
        bsr     NATIVE_ISLAND_FUNCTIONS_INC_0
        fcb     FP_STORE+0,FP_KARG+6,FP_DIV,FP_FIX
        fcb     FP_KARG+6,FP_MUL,FP_SWAP,FP_LOAD+0,FP_SUB
        fcb     FP_KARG+5,FP_COMPARE,FP_END
        bcs     LOCAL_FAILURE_19
        cmpa    #1
        bne     F_SIN_LOWER
        bsr     NATIVE_ISLAND_FUNCTIONS_INC_0
        fcb     FP_KARG+6
        fcb     FP_SUB
        fcb     FP_END
F_SIN_LOWER
        ldx     #NUMBER_NEG_PI
        jsr     ARG_FROM_ROM
        jsr     F_COMPARE
        cmpa    #$ff
        bne     F_SIN_REDUCED
        bsr     NATIVE_ISLAND_FUNCTIONS_INC_0
        fcb     FP_KARG+6
        fcb     FP_ADD
        fcb     FP_END
F_SIN_REDUCED
        jsr     F_SERIES_SEED
        lda     11,s
        eora    #$80
        sta     11,s
        ldd     #$02d6
        jsr     F_SERIES
        bcs     F_SIN_ERROR
        leas    32,s
        puls    y,u,pc
NATIVE_ISLAND_FUNCTIONS_INC_0
        jmp     FP_RUN
F_SIN_ERROR
        leas    32,s
        coma
        puls    y,u,pc
F_ATN
        pshs    y,u
        leas    -33,s
        lda     FSIGN
        anda    #$80
        sta     32,s
        jsr     EX_ABS
        bsr     NATIVE_ISLAND_FUNCTIONS_INC_0
        fcb     FP_KARG+0,FP_COMPARE,FP_END
        cmpa    #1
        bne     F_ATN_SMALL
        inc     32,s
        bsr     NATIVE_ISLAND_FUNCTIONS_INC_0
        fcb     FP_KARG+0
        fcb     FP_SWAP
        fcb     FP_DIV
        fcb     FP_END
F_ATN_SMALL
        bsr     NATIVE_ISLAND_FUNCTIONS_INC_0
        fcb     FP_KARG+4,FP_COMPARE,FP_END
        cmpa    #1
        bne     F_ATN_REDUCED
        lda     32,s
        ora     #2
        sta     32,s
        jsr     F_UNIT_RATIO
F_ATN_REDUCED
        jsr     F_SERIES_SEED
        lda     11,s
        eora    #$80
        sta     11,s
        ldd     #$032b
        jsr     F_SERIES
        lda     32,s
        bita    #2
        beq     F_ATN_RECIPROCAL
        bsr     NATIVE_ISLAND_FUNCTIONS_INC_0
        fcb     FP_KARG+8
        fcb     FP_ADD
        fcb     FP_END
F_ATN_RECIPROCAL
        lda     32,s
        bita    #1
        beq     F_ATN_SIGN
        jsr     FP_RUN
        fcb     FP_SWAP
        fcb     FP_KLOAD+7
        fcb     FP_SUB
        fcb     FP_END
F_ATN_SIGN
        tst     32,s
        bpl     F_ATN_END
        jsr     F_NEGATE
F_ATN_END
        jsr     F_FINISH
        leas    33,s
        puls    y,u,pc



; Shared arithmetic streams include the native helper's return word in slots.
F_UNIT_RATIO
        jsr     FP_RUN
        fcb     FP_STORE+2,FP_KARG+0,FP_ADD,FP_STORE+26
        fcb     FP_LOAD+2,FP_KARG+0,FP_SUB,FP_ARG+26,FP_DIV,FP_END
        rts
F_SERIES_SEED
        jsr     FP_RUN
        fcb     FP_STORE+2,FP_STORE+14,FP_STORE+20
        fcb     FP_ARG+2,FP_MUL,FP_STORE+8,FP_END
        rts

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

