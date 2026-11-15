; Original guest trace mode. Apache-2.0.
CMD_TRON
        ldb     #1
        bra     TRACE_SET
CMD_TROFF
        clrb
TRACE_SET
        ; The delimiter guard preserves B; the statement boundary owns D.
        jsr     CMD_REQUIRE_END
        stb     $af
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
