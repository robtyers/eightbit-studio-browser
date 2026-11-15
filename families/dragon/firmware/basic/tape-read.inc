; Original bounded edge sampling. SYNC leaves the input at a rising edge.
; BIT_IN returns the next cycle's bit in C; $81 reports a missing signal.
TAPE_SYNC
        pshs    d,x
        jsr     MOTORON
        clr     $81
        ldx     #0
TAPE_SYNC_LOW
        leax    -1,x
        beq     TAPE_SYNC_ERROR
        lda     $ff20
        bita    #1
        bne     TAPE_SYNC_LOW
TAPE_SYNC_HIGH
        leax    -1,x
        beq     TAPE_SYNC_ERROR
        lda     $ff20
        bita    #1
        beq     TAPE_SYNC_HIGH
        andcc   #$fe
        puls    d,x,pc
TAPE_SYNC_ERROR
        lda     #3
        sta     $81
        orcc    #1
        puls    d,x,pc
TAPE_BIT_IN
        pshs    cc,d,y
        orcc    #$50
        ldy     #0
        clr     $81
TAPE_BIT_HIGH
        leay    1,y
        beq     TAPE_BIT_TIMEOUT
        lda     $ff20
        bita    #1
        bne     TAPE_BIT_HIGH
TAPE_BIT_LOW
        leay    1,y
        beq     TAPE_BIT_TIMEOUT
        lda     $ff20
        bita    #1
        beq     TAPE_BIT_LOW
        ldb     #0
        cmpy    #28
        bhs     TAPE_BIT_RESULT
        incb
        bra     TAPE_BIT_RESULT
TAPE_BIT_TIMEOUT
        ldb     #3
        stb     $81
        ldb     #1
TAPE_BIT_RESULT
        lda     ,s
        anda    #$fe
        pshs    b
        ora     ,s+
        sta     ,s
        puls    cc,d,y,pc
TAPE_BYTE_IN
        pshs    b
        clra
        pshs    a
        ldb     #8
TAPE_READ_BIT
        jsr     TAPE_BIT_IN
        ror     ,s
        tst     $81
        bne     TAPE_READ_BYTE_END
        decb
        bne     TAPE_READ_BIT
TAPE_READ_BYTE_END
        puls    a,b
        tst     $81
        beq     TAPE_READ_BYTE_OK
        orcc    #1
        rts
TAPE_READ_BYTE_OK
        andcc   #$fe
        rts

