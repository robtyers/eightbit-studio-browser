; Original statement-boundary Break handling. Apache-2.0.

BREAK_POLL
        ldb     $ff02
        lda     #$fb
        sta     $ff02
        lda     $ff00
        stb     $ff02
        bita    #64
        bne     BREAK_POLL_END
        lda     KEYOLD+2
        anda    #$bf
        sta     KEYOLD+2
        clra
BREAK_POLL_END
        rts
BREAK_MESSAGE
        pshs    x
        ldx     #BREAK_TEXT
        jsr     PUTS
        jsr     ERROR_LOCATION
        puls    x,pc

BREAK_TEXT
        fcc     "BREAK"
        fcb     0

