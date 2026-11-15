; Original string values and heap allocation. Apache-2.0.
; Legacy records are [length,0,pointer-hi,pointer-lo,0]. FAC string pointers
; use $52 and the published length byte $56; $4F also carries a local copy.
; X addresses the common five-byte value; $06 selects its legacy interpretation.
VALUE_LOAD
        tst     $06
        bne     STRING_LOAD
        jmp     FAC_LOAD
STRING_VALUE
        sta     FAC
        sta     STR_LEN
        stu     STR_PTR
        lda     #$ff
        sta     $06
        clrb
        rts
STRING_NEW_VALUE
        bsr     STRING_VALUE
        lda     #1
        sta     STR_OWNER
        rts

STRING_LITERAL
        leax    1,x
        tfr     x,u
        clrb
STRING_LITERAL_SCAN
        lda     ,x
        beq     STRING_LITERAL_END
        leax    1,x
        cmpa    #'"
        beq     STRING_LITERAL_END
        incb
        bne     STRING_LITERAL_SCAN
        lbra    EX_ERROR
STRING_LITERAL_END
        tfr     b,a
        bsr     STRING_VALUE
        lda     #2
        sta     STR_OWNER
        cmpu    TXTTAB
        blo     STRING_LITERAL_COPY
        cmpu    VARTAB
        blo     STRING_LITERAL_OK
STRING_LITERAL_COPY
        pshs    x
        ldx     STR_PTR
        clra
        ldb     FAC
        jsr     VALUE_COPY
        puls    x,pc

STRING_LITERAL_OK
        clra
        rts
STRING_LOAD
        lda     ,x
        ldu     2,x
        bsr     STRING_VALUE
        clr     STR_OWNER
        cmpu    TXTTAB
        blo     STRING_LOAD_END
        cmpu    VARTAB
        bhs     STRING_LOAD_END
        lda     #2
        sta     STR_OWNER
STRING_LOAD_END
        clra
        rts
STRING_ALLOC
        pshs    d,x
        ldd     FRETOP
        subd    ,s
        bcs     STRING_ALLOC_ERROR
        cmpd    STACKTOP
        blo     STRING_ALLOC_COLLECT
STRING_ALLOC_READY
        std     FRETOP
        tfr     d,u
        leau    1,u
        puls    d,x,pc
STRING_ALLOC_ERROR
        coma
        puls    d,x,pc
STRING_ALLOC_COLLECT
        jsr     STRING_COLLECT_ACTIVE
        bcs     STRING_ALLOC_ERROR
        ldd     FRETOP
        subd    ,s
        bcs     STRING_ALLOC_ERROR
        cmpd    STACKTOP
        blo     STRING_ALLOC_ERROR
        bra     STRING_ALLOC_READY
STRING_COPY
        tstb
        beq     STRING_COPY_END
STRING_COPY_BYTE
        lda     ,x+
        sta     ,u+
        decb
        bne     STRING_COPY_BYTE
STRING_COPY_END
        rts
STRING_STORE
        pshs    x,y,u
        tst     FAC
        beq     STRING_STORE_EMPTY
        tst     STR_OWNER
        bne     STRING_STORE_READY
        clra
        ldb     FAC
        bsr     STRING_ALLOC
        bcs     STRING_STORE_FAIL
        ldx     STR_PTR
        stu     STR_PTR
        bsr     STRING_COPY
        bra     STRING_STORE_READY
STRING_STORE_EMPTY
        clra
        clrb
        std     STR_PTR
STRING_STORE_READY
        lda     FAC
        sta     ,y
        clr     1,y
        ldd     STR_PTR
        std     2,y
        clr     4,y
STRING_STORE_FAIL
        puls    x,y,u,pc
STRING_CONCAT
        lda     FAC
        adda    ARG
        bcs     STRING_TOO_LONG
        pshs    a
        clra
        ldb     ,s
        inc     STR_BOTH
        bsr     STRING_ALLOC
        lda     #0
        sta     STR_BOTH
        bcs     STRING_CONCAT_FAIL
        pshs    u
        ldx     STR_PTR
        ldb     FAC
        bsr     STRING_COPY
        ldx     ARG+3
        ldb     ARG
        bsr     STRING_COPY
        puls    u
        puls    a
        jmp     STRING_NEW_VALUE
STRING_CONCAT_FAIL
        leas    1,s
STRING_TOO_LONG
        coma
        rts
STRING_COMPARE
        pshs    b,x,y
        ldx     STR_PTR
        ldy     ARG+3
        ldb     FAC
        cmpb    ARG
        bls     STRING_COMPARE_LOOP
        ldb     ARG
STRING_COMPARE_LOOP
        jsr     COMPARE_BYTES
        bne     STRING_COMPARE_RESULT
STRING_COMPARE_LENGTH
        lda     FAC
        cmpa    ARG
        bne     STRING_COMPARE_RESULT
        clra
        bra     STRING_COMPARE_END
STRING_COMPARE_RESULT
        lda     #0
        sbca    #0
        ora     #1
STRING_COMPARE_END
        clr     $06
        tsta
        puls    b,x,y,pc
LOCAL_FAILURE_4
        jmp     EX_ERROR
STRING_PRINT
        pshs    d,x
        ldx     STR_PTR
        ldb     FAC
        beq     STRING_PRINT_END
STRING_PRINT_BYTE
        lda     ,x+
        jsr     CHROUT
        decb
        bne     STRING_PRINT_BYTE
STRING_PRINT_END
        puls    d,x,pc
STRING_LENGTH
        tst     $06
        beq     LOCAL_FAILURE_4
        clra
        ldb     FAC
        jmp     INT_TO_FAC
STRING_ASC
        tst     $06
        beq     LOCAL_FAILURE_4
        tst     FAC
        beq     LOCAL_FAILURE_4
        ldx     STR_PTR
        clra
        ldb     ,x
        jmp     INT_TO_FAC
STRING_CHR
        jsr     EX_TO_ADDRESS
        tsta
        bne     LOCAL_FAILURE_4
STRING_BYTE
        pshs    b
        ldd     #1
        leax    ,s
        jsr     VALUE_COPY
        puls    b,pc

STRING_VAL
        tst     $06
        beq     LOCAL_FAILURE_4
        pshs    u
        jsr     STACK_STRING
        bcs     STRING_VAL_ERROR
        jsr     EX_SPACE
        cmpa    #'&
        beq     STRING_VAL_RADIX
        jsr     ASCII_SIGN
STRING_VAL_SIGNED
        jsr     EX_SPACE
        cmpa    #'.
        beq     STRING_VAL_NUMBER
        cmpa    #'0
        blo     STRING_VAL_ZERO
        cmpa    #'9
        bhi     STRING_VAL_ZERO
STRING_VAL_NUMBER
        jsr     NUMBER_SIGNED
        bcs     STRING_VAL_ERROR
        bra     STRING_VAL_RETURN

STRING_VAL_RADIX
        jsr     EX_BASE_NUMBER
        bra     STRING_VAL_RETURN
STRING_VAL_ZERO
        jsr     EX_FALSE
        bra     STRING_VAL_RETURN
STRING_VAL_ERROR
        coma
STRING_VAL_RETURN
        tfr     u,s
        leas    2,s
        puls    u,pc

