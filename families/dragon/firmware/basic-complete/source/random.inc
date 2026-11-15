; Original native base-256 generator. The recurrence coefficients and five-byte seed
; layout are inferred from approved black-box numeric-state observations.
; No reference instructions or implementation code are inputs.
; Positive arguments select 1..INT(n); zero/negative arguments return a fraction.
; The enclosing expression owns Y/U, including the string-type error call.
VALUE_RND
        leas    -14,s
        jsr     FP_RUN
        fcb     FP_STORE+8,FP_END
        tst     FSIGN
        bpl     RND_NEXT
        ldd     FMANT
        std     $0116
        ldd     FMANT+2
        std     $0118
RND_NEXT
        bsr     RND_LOAD_SEED
        ldd     #$40e6
        std     AMANT
        ldd     #$4dab
        std     AMANT+2
        tfr     s,u
        jsr     F_PRODUCT
        ldd     4,s
        std     $ab
        ldd     6,s
        std     $ad
        ; Black-box public numeric facts: the return guard is product bits
        ; 40..47, independently of the low-32-bit seed recurrence.
        lda     2,s
        sta     FGUARD
        ldd     $ad
        addd    #$658b
        std     $0118
        ldd     $ab
        adcb    #$b0
        adca    #5
        std     $0116
        lda     #128
        sta     $0115
RND_FRACTION
        bsr     RND_LOAD_SEED
        lda     #128
        sta     FAC
        clr     FSIGN
        jsr     F_NORMALISE
        tst     13,s
        bmi     RND_DONE
        tst     8,s
        beq     RND_DONE
        ; Preserve the random fraction in ARG while truncating the original
        ; positive bound. A nonzero bound below one still produces integer one.
        jsr     FP_RUN
        fcb     FP_SWAP,FP_LOAD+8,FP_FIX,FP_SWAP
        fcb     FP_MUL,FP_FIX,FP_KARG+FPC_ONE,FP_ADD,FP_END
        bcs     RND_FAIL
RND_DONE
        leas    14,s
        jmp     NATIVE_TAIL_9

RND_LOAD_SEED
        ldd     $0116
        std     FMANT
        ldd     $0118
        std     FMANT+2
        rts
RND_FAIL
        leas    14,s
        jmp     NATIVE_TAIL_6

