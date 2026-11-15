; Cursor positioning is validated before the published screen pointer changes.
PRINT_AT
        leax    1,x
        jsr     EVAL_ADDRESS
        cmpd    #511
        bhi     PRINT_AT_ERROR
        pshs    d
        jsr     MULTI_COMMA
        puls    d
        addd    #$0400
        std     CURSOR
        andb    #31
        stb     $6c
        jmp     CMD_PRINT
PRINT_AT_ERROR
        coma
        rts

