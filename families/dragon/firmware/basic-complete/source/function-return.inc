FN_EVALUATE
        ldx     7,u
        jsr     EX_ONE
        bcs     FN_RESTORE_PARAMETER
        tst     $06
        bne     FN_RESULT_ERROR
        jsr     CMD_END_CARRY
        bra     FN_RESTORE_PARAMETER
FN_RESULT_ERROR
        coma
FN_RESTORE_PARAMETER
        ldy     5,u
        tfr     u,x
        ldb     #5
        jsr     COPY_BYTES
        ldx     9,u
        leas    13,s
        rts
