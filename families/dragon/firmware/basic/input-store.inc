INPUT_STORE
        clr     STR_OWNER
        ldy     3,u
        jsr     VALUE_STORE
        jsr     ST_CHECK
INPUT_STORED
        ldx     1,u
        tst     ,u
        bne     NATIVE_SHARED12_INPUT_STORE_INC_0
        jsr     EX_OPTIONAL_COMMA
        bne     NATIVE_SHARED12_INPUT_STORE_INC_0
        pshs    x
        ldx     5,u
        lda     ,x+
        stx     5,u
        puls    x
        cmpa    #',
        beq     INPUT_NEXT_VARIABLE
        lbra    INPUT_VARIABLE
INPUT_NEXT_VARIABLE
        jsr     INPUT_FIND_VARIABLE
        jmp     INPUT_VALUE
NATIVE_SHARED12_INPUT_STORE_INC_0
        jmp     INPUT_SUCCESS

