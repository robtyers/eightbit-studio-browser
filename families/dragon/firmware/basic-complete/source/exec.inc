; Original native machine-code entry and legacy stack hand-off. Apache-2.0.
CMD_EXEC
        jsr     EX_END
        beq     CMD_EXEC_DEFAULT
        jsr     EVAL_ADDRESS
        tfr     d,y
        jsr     CMD_REQUIRE_END
        bra     CMD_EXEC_CALL
CMD_EXEC_DEFAULT
        ldy     $9d
CMD_EXEC_CALL
        stx     TXT_PTR
        lda     #1
        sta     EXEC_MODE
        ; Machine code sees the legacy control stack immediately after its
        ; return slot. Remove the temporary statement boundary before JMP.
        lds     STATEMENT_SP
        ldd     2,s
        std     STATEMENT_SP
        ldb     5,s
        jsr     STRING_TEMP_REWIND
        leas    12,s
        jmp     ,y
