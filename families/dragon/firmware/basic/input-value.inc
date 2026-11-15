INPUT_VALUE
        tst     ,u
        beq     INPUT_CSV
        pshs    u
        leau    -255,u
        lda     LINELEN
        jsr     STRING_VALUE
        puls    u
        jmp     INPUT_STORE
INPUT_CSV
        ldx     5,u
        jsr     DATA_VALUE
        bcs     INPUT_RETRY
        lda     ,x
        cmpa    #':
        beq     INPUT_RETRY
        stx     5,u
        lbra    INPUT_STORE
INPUT_RETRY
        ldx     #INPUT_REDO
        jsr     PUTS
        bra     INPUT_RESTORE_TYPE
INPUT_REDO
        fcc     "?REDO"
        fcb     13,0

