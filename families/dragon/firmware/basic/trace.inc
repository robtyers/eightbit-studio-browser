; Original guest trace mode. Apache-2.0.
CMD_TRON
        lda     #1
        bra     TRACE_SET
CMD_TROFF
        clra
TRACE_SET
        pshs    a
        jsr     CMD_REQUIRE_END
        puls    a
        sta     $af
        rts
TRACE_LINE
        tst     $af
        beq     TRACE_DONE
        pshs    d
        lda     #'[
        jsr     CHROUT
        ldd     CURLINE
        jsr     PRINT_UNSIGNED
        lda     #']
        jsr     CHROUT
        puls    d
TRACE_DONE
        rts

