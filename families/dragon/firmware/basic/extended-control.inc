; Original computed branches and machine-code function definitions.
CMD_ON
        jsr     GFX_BYTE
        pshs    b
        jsr     EX_SPACE
        cmpa    #$81
        bne     PARSE_FAILURE_4
        jsr     EX_NEXT_SPACE
        cmpa    #$bc
        beq     ON_KIND
        cmpa    #$bd
        bne     PARSE_FAILURE_4
ON_KIND
        pshs    a
        leax    1,x
ON_TARGET
        jsr     EX_SPACE
        pshs    x
        jsr     PARSE_LINE_NUMBER
        puls    y
        bsr     ST_CHECK
        dec     1,s
        beq     ON_SELECTED
        bsr     EX_OPTIONAL_COMMA
        bne     ON_NOT_SELECTED
        bra     ON_TARGET
ON_SELECTED
        ldb     ,s
        leas    2,s
        cmpb    #$bd
        bne     ON_GOTO
        leax    -1,y
        jmp     CMD_GOSUB
ON_GOTO
        bra     CTL_GO_TARGET_FROM_NUMBER
ON_NOT_SELECTED
        leas    2,s
        clra
        rts
CTL_GO_TARGET_FROM_NUMBER
        jsr     CTL_FIND_LINE
        bcs     PARSE_FAILURE_4
        jmp     CTL_GO_TARGET
PARSE_FAILURE_4
        jmp     CMD_UNSUPPORTED
CMD_DEF
        jsr     EX_SPACE
        cmpa    #$be
        lbeq    CMD_DEF_FN
        cmpa    #$ff
        bne     PARSE_FAILURE_4
        lda     1,x
        cmpa    #$a1
        bne     PARSE_FAILURE_4
        leax    2,x
        jsr     USR_INDEX
        pshs    b
        bsr     EX_EQUALS
        jsr     EX_ADDRESS
        pshs    d
        jsr     CMD_REQUIRE_END
        ldu     USR_TAB
        ldb     2,s
        leau    b,u
        puls    d
        leas    1,s
        std     ,u
        rts

