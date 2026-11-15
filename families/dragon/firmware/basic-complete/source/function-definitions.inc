; Original FN definitions. Observed record: body pointer, parameter value
; pointer, zero; the first significant name byte has its high bit set.
CMD_DEF_FN
        ldd     CURLINE
        cmpd    #$ffff
        lbeq    DATA_DIRECT_ERROR
        leax    1,x
        jsr     FN_NAME
        tst     $06
        bne     PARSE_FAILURE_6
        pshs    y
        jsr     EX_OPEN
        jsr     VAR_FIND_NATIVE
        jsr     EX_NUMBER_CHECK
        cmpy    ARYTAB
        bhs     NATIVE_ISLAND_FUNCTION_DEFINITIONS_INC_0
        jsr     MULTI_CLOSE
        pshs    y
        jsr     EX_EQUALS
        jsr     EX_SPACE
        tsta
        beq     NATIVE_ISLAND_FUNCTION_DEFINITIONS_INC_0
        cmpa    #':
        beq     NATIVE_ISLAND_FUNCTION_DEFINITIONS_INC_0
        puls    y,u
        stx     ,u
        sty     2,u
        clr     4,u
        lbra     CMD_DATA
NATIVE_ISLAND_FUNCTION_DEFINITIONS_INC_0
PARSE_FAILURE_6
        lbra     NATIVE_FAIL
