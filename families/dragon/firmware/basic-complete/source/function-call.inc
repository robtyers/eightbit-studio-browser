; Original lexical parameter binding, with restoration on success and failure.
EX_FN
        leax    1,x
        jsr     FN_NAME
        jsr     EX_NUMBER_CHECK
        pshs    y
        jsr     EX_PAREN
        jsr     EX_NUMBER_CHECK
        puls    y
        ldd     ,y
        cmpd    TXTTAB
        blo     PARSE_FAILURE_5
        cmpd    VARTAB
        bhs     PARSE_FAILURE_5
        ldd     2,y
        cmpd    VARTAB
        blo     PARSE_FAILURE_5
        cmpd    ARYTAB
        bhs     PARSE_FAILURE_5
        leas    -13,s
        tfr     s,u
        stx     9,u
        ldd     ,y
        std     7,u
        ldd     2,y
        std     5,u
        tfr     d,x
        tfr     u,y
        ldb     #5
        jsr     COPY_BYTES
        ldx     5,u
        jsr     FAC_STORE
        jmp     FN_EVALUATE
PARSE_FAILURE_5
        jmp     NATIVE_UNDEFINED_FN_ERROR
