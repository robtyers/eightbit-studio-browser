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


; The documented hook can add four to its saved return address to suppress
; this Break check. Explicit opcode data keeps the gate four bytes after packing.
EXEC_NEXT_HOOK
        stx     TXT_PTR
        jsr     $019a
        fcb     $bd
        fdb     EXEC_NEXT_BREAK
        nop
        ldx     TXT_PTR
        rts
EXEC_NEXT_BREAK
        bsr     BREAK_POLL
        bne     EXEC_NEXT_BREAK_RETURN
        leas    4,s
        jsr     CMD_END_VALIDATED
        bsr     BREAK_MESSAGE
        jmp     EXEC_COMPLETE
EXEC_NEXT_BREAK_RETURN
        rts
