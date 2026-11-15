; Original shared power/factorial recurrence for EXP, LOG, SIN and ATN.
; Caller slots: multiplier6, sum12, power18, temporary24. A first divisor,
; B low six bits: exclusive limit. Bit7 selects the divided-power recurrence;
; bit6 selects divisor n*(n+1) and a step of two. Plain powers also step by two.
; The extra four bytes below the caller frame are our return and counter pair.
F_SERIES
        pshs    d
SERIES_TERM
        bsr     NATIVE_ISLAND_SERIES_INC_0
        fcb     FP_LOAD+22,FP_ARG+10,FP_MUL,FP_STORE+22,FP_STORE+28,FP_END
        bcs     SERIES_END
        ldb     ,s
        lda     1,s
        bita    #$40
        beq     SERIES_LINEAR_DIVISOR
        tfr     b,a
        incb
        mul
        bra     SERIES_DIVISOR
NATIVE_ISLAND_SERIES_INC_0
        jmp     FP_RUN
SERIES_LINEAR_DIVISOR
        clra
SERIES_DIVISOR
        jsr     INT_TO_FAC
        bsr     NATIVE_ISLAND_SERIES_INC_0
        fcb     FP_ARG+28,FP_SWAP,FP_DIV,FP_END
        bcs     SERIES_END
        tst     1,s
        bpl     SERIES_SUM
        bsr     NATIVE_ISLAND_SERIES_INC_0
        fcb     FP_STORE+22,FP_END
SERIES_SUM
        bsr     NATIVE_ISLAND_SERIES_INC_0
        fcb     FP_ARG+16,FP_ADD,FP_STORE+16,FP_END
        bcs     SERIES_END
        inc     ,s
        lda     1,s
        bita    #$80
        beq     SERIES_STEP_TWO
        bita    #$40
        beq     SERIES_LIMIT
SERIES_STEP_TWO
        inc     ,s
SERIES_LIMIT
        anda    #$3f
        cmpa    ,s
        bhi     SERIES_TERM
SERIES_END
        leas    2,s
        rts

