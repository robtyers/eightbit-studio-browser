; Original framed blocks: 55 3C type length payload sum 55.
TAPE_BLOCK_OUT
        pshs    cc,d,x,u
        orcc    #$50
        clr     $81
        ; Reads may cross $7FFF or wrap $FFFF; the byte count bounds them.
        lda     #$55
        jsr     TAPE_BYTE_OUT
        lda     #$3c
        jsr     TAPE_BYTE_OUT
        lda     $7c
        sta     $80
        jsr     TAPE_BYTE_OUT
        lda     $7d
        adda    $80
        sta     $80
        lda     $7d
        jsr     TAPE_BYTE_OUT
        ldx     $7e
        ldb     $7d
        beq     TAPE_OUTPUT_SUM
TAPE_OUTPUT_DATA
        lda     ,x+
        pshs    a
        adda    $80
        sta     $80
        puls    a
        jsr     TAPE_BYTE_OUT
        ; Preserve the measured inter-byte budget while using the byte count.
        pshs    d
        puls    d
        decb
        bne     TAPE_OUTPUT_DATA
TAPE_OUTPUT_SUM
        lda     $80
        jsr     TAPE_BYTE_OUT
        lda     #$55
        jsr     TAPE_BYTE_OUT
        ; A pending frame IRQ must not stretch a last bit between bytes.
        ; Restore interrupt masks only after the complete framed block.
        puls    cc,d,x,u
        tst     $81
        andcc   #$fe
        rts

