; Shared original statement guards and text clearing. Apache-2.0.
CMD_REM
        lda     ,x+
        bne     CMD_REM
        leax    -1,x
        clra
        rts
CMD_CLS equ CLS
CMD_END_CARRY
        jsr     EX_END
        beq     CMD_END_VALID
        coma
        rts
CMD_END_VALID
        clra
        rts
CMD_REQUIRE_END
        bsr     CMD_END_CARRY
        jmp     ST_CHECK

