; Original multi-argument string functions. Arguments remain registered roots
; while later expressions allocate or collect the descending legacy heap.
EX_MULTI_STRING
        pshs    y,u
        leas    -12,s
        tfr     s,u
        sta     7,u
        lda     #1
        sta     8,u
        ldd     STR_ROOTS
        std     ,u
        clr     2,u
        stu     STR_ROOTS
        jsr     EX_OPEN
        bsr     NATIVE_ISLAND_STRING_FUNCTIONS_INC_0
        lda     7,u
        cmpa    #$a0
        lbeq    MULTI_REPEAT
        cmpa    #$9d
        bne     MULTI_FIRST_STRING
        tst     $06
        bne     MULTI_FIRST_STRING
        bsr     NATIVE_ISLAND_STRING_FUNCTIONS_INC_1
        tstb
        lbeq    NATIVE_FAIL
        stb     8,u
        jsr     MULTI_COMMA
        bsr     NATIVE_ISLAND_STRING_FUNCTIONS_INC_0
MULTI_FIRST_STRING
        tst     $06
        beq     PARSE_FAILURE_19
        lda     FAC
        sta     2,u
        ldd     STR_PTR
        std     4,u
        jsr     MULTI_COMMA
        bsr     NATIVE_ISLAND_STRING_FUNCTIONS_INC_0
        lda     7,u
        cmpa    #$9d
        lbeq    MULTI_SEARCH
        bsr     NATIVE_ISLAND_STRING_FUNCTIONS_INC_1
        stb     9,u
        lda     7,u
        cmpa    #$98
        bne     MULTI_SUBSTRING
        tstb
        beq     PARSE_FAILURE_19
        stb     8,u
        lda     #255
        sta     9,u
        jsr     EX_OPTIONAL_COMMA
        bne     MULTI_SUBSTRING
        bsr     NATIVE_ISLAND_STRING_FUNCTIONS_INC_0
        bsr     NATIVE_ISLAND_STRING_FUNCTIONS_INC_1
        stb     9,u
MULTI_SUBSTRING
        jsr     MULTI_CLOSE
        clrb
        lda     7,u
        cmpa    #$97
        bne     MULTI_LEFT_OR_MID
        ldb     2,u
        subb    9,u
        bhs     MULTI_SLICE
        clrb
        bra     MULTI_SLICE
NATIVE_ISLAND_STRING_FUNCTIONS_INC_0
        jmp     EX_BODY
PARSE_FAILURE_19
        jmp     MULTI_ERROR
NATIVE_ISLAND_STRING_FUNCTIONS_INC_1
        jmp     MULTI_BYTE
MULTI_LEFT_OR_MID
        cmpa    #$98
        bne     MULTI_SLICE
        ldb     8,u
        decb
MULTI_SLICE
        cmpb    2,u
        blo     MULTI_SLICE_LENGTH
        clra
        bra     MULTI_SLICE_READY
MULTI_SLICE_LENGTH
        lda     2,u
        pshs    b
        suba    ,s+
        cmpa    9,u
        bls     MULTI_SLICE_READY
        lda     9,u
MULTI_SLICE_READY
        pshs    a
        clra
        addd    4,u
        tfr     d,u
        puls    a
        jsr     STRING_VALUE
        tfr     s,u
        clr     STR_OWNER
        jmp     MULTI_SUCCESS
MULTI_REPEAT
        bsr     NATIVE_ISLAND_STRING_FUNCTIONS_INC_1
        stb     9,u
        jsr     MULTI_COMMA
        bsr     NATIVE_ISLAND_STRING_FUNCTIONS_INC_0
        tst     $06
        beq     MULTI_REPEAT_NUMBER
        tst     FAC
        beq     PARSE_FAILURE_19
        ldy     STR_PTR
        ldb     ,y
        bra     MULTI_REPEAT_BYTE
MULTI_REPEAT_NUMBER
        bsr     NATIVE_ISLAND_STRING_FUNCTIONS_INC_1
MULTI_REPEAT_BYTE
        stb     8,u
        jsr     MULTI_CLOSE
        pshs    x,u
        clra
        ldb     9,u
        jsr     STRING_ALLOC
        puls    x,y
        bcs     MULTI_REPEAT_ERROR
        tfr     u,d
        std     STR_PTR
        ldb     9,y
        lda     8,y
        tstb
        beq     MULTI_REPEAT_DONE
MULTI_REPEAT_FILL
        sta     ,u+
        decb
        bne     MULTI_REPEAT_FILL
MULTI_REPEAT_DONE
        lda     9,y
        ldu     STR_PTR
        jsr     STRING_NEW_VALUE
        tfr     s,u
        bra     MULTI_SUCCESS
MULTI_REPEAT_ERROR
        tfr     y,u
        bra     MULTI_ERROR
MULTI_SEARCH
        tst     $06
        beq     MULTI_ERROR
        jsr     MULTI_CLOSE
        pshs    x
        ldb     8,u
        decb
        cmpb    2,u
        bhi     MULTI_NOT_FOUND
        ldx     4,u
        clra
        leax    d,x
MULTI_SEARCH_POSITION
        lda     2,u
        suba    8,u
        inca
        cmpa    FAC
        blo     MULTI_NOT_FOUND
        ldy     STR_PTR
        ldb     FAC
        beq     MULTI_FOUND
        pshs    x
MULTI_SEARCH_BYTE
        jsr     COMPARE_BYTES
        bne     MULTI_SEARCH_NEXT
        puls    x
MULTI_FOUND
        clra
        ldb     8,u
        bra     MULTI_SEARCH_RESULT
MULTI_SEARCH_NEXT
        puls    x
        leax    1,x
        inc     8,u
        bne     MULTI_SEARCH_POSITION
MULTI_NOT_FOUND
        clra
        clrb
MULTI_SEARCH_RESULT
        jsr     INT_TO_FAC
        puls    x
MULTI_SUCCESS
        clra
        bra     MULTI_RETURN
MULTI_ERROR
        jmp     NATIVE_FAIL
MULTI_RETURN
        ldd     ,u
        std     STR_ROOTS
        leas    12,s
        puls    y,u,pc

MULTI_BYTE
        jsr     EX_TO_ADDRESS
        tsta
        lbne    NATIVE_RANGE_ERROR
        rts

