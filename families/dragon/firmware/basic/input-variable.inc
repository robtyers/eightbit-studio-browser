INPUT_FIND_VARIABLE
        jsr     VAR_FIND_NATIVE
        stx     1,u
        sty     3,u
        lda     $06
        sta     7,u
        rts
INPUT_VARIABLE
        bsr     INPUT_FIND_VARIABLE
        tst     ,u
        beq     INPUT_PROMPT
        tst     $06
        lbeq    NATIVE_FAIL
        jsr     CMD_REQUIRE_END
INPUT_PROMPT
        tst     ,u
        bne     INPUT_LINE_REPLY
        lda     #'?
        jsr     CHROUT
        lda     #32
        jsr     CHROUT
INPUT_LINE_REPLY
        leax    -255,u
        jsr     READLINE_INTO
        jsr     ST_CHECK
        stx     5,u
        jmp     INPUT_VALUE

