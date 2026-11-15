; Original interactive input. Reply storage is a bounded guest stack buffer,
; so READLINE cannot overwrite direct tokens or public interpreter workspace.
CMD_INPUT
        clra
        bra     INPUT_BEGIN
CMD_LINE_INPUT
        leax    1,x
        lda     #1
INPUT_BEGIN
        leau    -392,s
        cmpu    STREND
        lbls    NATIVE_FAIL
INPUT_STACK_READY
        leas    -264,s
        leau    256,s
        sta     ,u
        jsr     EX_SPACE
        cmpa    #'"
        lbne     INPUT_VARIABLE
        pshs    u
        jsr     STRING_LITERAL
        puls    u
        jsr     ST_CHECK
        jsr     STRING_PRINT
        lda     #';
        jsr     EX_EXPECT
        jmp     INPUT_VARIABLE

