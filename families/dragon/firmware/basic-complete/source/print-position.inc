; Cursor positioning is validated before the published screen pointer changes.
PRINT_AT
        leax    1,x
        jsr     EVAL_ADDRESS
        cmpd    #511
        lbhi    NATIVE_RANGE_ERROR
        pshs    d
        jsr     MULTI_COMMA
        puls    d
        addd    #$0400
        std     CURSOR
        ; Retain the nested success call's stack depth for USING's preflight,
        ; but position has consumed the sole prefix and requires a first item.
        jsr     EX_SPACE
        jsr     PRINT_FIRST
        jmp     CMD_PRINT_SUCCESS
