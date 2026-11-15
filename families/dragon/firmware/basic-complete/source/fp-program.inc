; Original compact arithmetic microprograms, interpreted entirely by the 6809.
; Slots address the caller's current guest stack frame. The constant dictionary
; shares original numeric facts; all arithmetic and decoding stay on the guest.
FP_RUN
        puls    y
        pshs    x,u
        leau    4,s
FP_NEXT
        ldb     ,y+
        beq     FP_SUCCESS
FP_SAVE_CONTINUATION
        pshs    y,u
        cmpb    #FP_ADD
        bhs     FP_OPERATION
        ; Slots share a six-bit offset; the two high bits select load/store/ARG.
        decb
        tfr     b,a
        andb    #63
        leax    b,u
        lsla
        bcs     FP_ARGUMENT
        bmi     FP_WRITE_SLOT
FP_LOAD_VALUE
        jsr     FAC_UNPACKED_LOAD
        bra     FP_COPY_DONE
FP_OPERATION
        cmpb    #FP_KARG
        bhs     FP_DICTIONARY
        subb    #FP_ADD
        ldx     #FP_OPERATIONS
        ; Native tail continues into the following block.
FP_DISPATCH
        lslb
        jsr     [b,x]
FP_RESTORE
        puls    y,u
        bcc     FP_NEXT
; All micro-operations are now one byte; failures scan to the terminator.
FP_SKIP
        ldb     ,y+
        bne     FP_SKIP
FP_ERROR
        comb
        bra     FP_DONE

FP_DICTIONARY
        pshs    b
        subb    #FP_KARG
        andb    #15
        lda     #6
        mul
        ldx     #FP_CONSTANTS
        leax    d,x
        puls    b
        cmpb    #FP_KLOAD
        blo     FP_ARGUMENT
        bra     FP_LOAD_VALUE
FP_WRITE_SLOT
        jsr     FAC_UNPACKED_STORE
        bra     FP_COPY_DONE
FP_ARGUMENT
        jsr     ARG_FROM_ROM
FP_COPY_DONE
        puls    y,u
        bra     FP_NEXT
FP_SUCCESS
        lsrb
FP_DONE
        puls    x,u
        jmp     ,y
FP_OPERATIONS
        fdb     F_ADD,F_SUB,F_MUL,F_DIV,F_SWAP
FP_FUNCTIONS
        fdb     F_FIX_VALUE,F_LOG,F_EXP,F_SIN,F_COS,FP_COMPARE_VALUE
; Comparisons return a relation in A, never an arithmetic failure carry.
FP_COMPARE_VALUE
        jsr     F_COMPARE
        andcc   #$fe
        rts
