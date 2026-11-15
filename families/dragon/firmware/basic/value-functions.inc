; Original guest conversions and non-parenthesised/device functions.
EX_INKEY
        pshs    x
        jsr     $bbe5
        beq     EX_INKEY_EMPTY
        tfr     a,b
        jsr     STRING_BYTE
        puls    x,pc
EX_INKEY_EMPTY
        ldu     #0
        clra
        clr     STR_OWNER
        jsr     STRING_VALUE
        puls    x,pc
EX_VARPTR
        jsr     EX_OPEN
        jsr     VAR_FIND_NATIVE
        jsr     MULTI_CLOSE
        tfr     y,d
        jmp     UNSIGNED_TO_FAC


EX_USR
        jsr     USR_INDEX
        pshs    b
        jsr     EX_PAREN
        tst     $06
        jsr     NATIVE_CHECK_NE
        puls    b
        ldy     USR_TAB
        ldy     b,y
        pshs    x
        ldx     #FAC
        jsr     ,y
        puls    x,pc
VALUE_HEX
        jsr     EX_TO_ADDRESS
        pshs    x,y,u
        leas    -4,s
        leax    4,s
        ldu     #0
; Emit from the low nibble backwards: zero naturally produces one digit.
HEX_DIGIT
        pshs    d
        andb    #15
        addb    #'0
        cmpb    #'9
        bls     HEX_STORE
        addb    #7
HEX_STORE
        stb     ,-x
        puls    d
        lsra
        rorb
        lsra
        rorb
        lsra
        rorb
        lsra
        rorb
        leau    1,u
        cmpd    #0
        bne     HEX_DIGIT
        tfr     u,d
        bsr     VALUE_COPY
        leas    4,s
        puls    x,y,u,pc

VALUE_STR
        pshs    x,y,u
        leas    -32,s
        sts     FORMAT_TARGET
        lda     #32
        sta     FORMAT_LENGTH
        inc     FORMAT_BUFFER
        jsr     PRINT_NUMBER
        clr     FORMAT_BUFFER
        clra
        ldb     #31
        subb    FORMAT_LENGTH
        leax    ,s
        bsr     VALUE_COPY
        leas    32,s
        puls    x,y,u,pc
; D count, X stack source. Allocation may collect; stack source remains stable.
VALUE_COPY
        pshs    d
        clr     $06
        jsr     STRING_ALLOC
        bcs     VALUE_COPY_ERROR
        pshs    u
        jsr     STRING_COPY
        puls    u
        puls    d
        tfr     b,a
        jmp     STRING_NEW_VALUE
PARSE_FAILURE_22
        jmp     EX_ERROR
VALUE_COPY_ERROR
        leas    2,s
        rts
VALUE_POS
        jsr     FAC_TO_INT
        bcs     PARSE_FAILURE_22
        cmpd    #-2
        bne     VALUE_POS_SCREEN
        ldb     $9c
        bra     VALUE_POS_RESULT
VALUE_POS_SCREEN
        ldb     $6c
VALUE_POS_RESULT
NATIVE_TAIL_14
        clra
        jmp     INT_TO_FAC
VALUE_JOY
        jsr     EX_TO_ADDRESS
        cmpd    #3
        lbhi    NATIVE_RANGE_ERROR
        pshs    b
        jsr     JOYIN
        puls    b
        ldx     #POTVAL
        ldb     b,x
        bra     NATIVE_TAIL_14

VALUE_EOF
        jsr     FAC_TO_INT
        bcs     PARSE_FAILURE_22
        cmpd    #-1
        bne     PARSE_FAILURE_22
        clra
        ldb     $70
        beq     VALUE_EOF_RESULT
        ldd     #-1
VALUE_EOF_RESULT
        jmp     INT_TO_FAC
EX_BASE_NUMBER
        pshs    y
        ldy     #0
        jsr     EX_NEXT_SPACE
        jsr     UPCASE
        ldb     #3
        cmpa    #'H
        beq     BASE_HEXADECIMAL
        cmpa    #'O
        beq     BASE_PREFIX
        ; The lawful bare-& form seeds its first decimal digit before octal
        ; accumulation. Explicit O and all following digits enforce 0..7.
        suba    #'0
        cmpa    #9
        bhi     BASE_BEGIN
        tfr     a,b
        clra
        tfr     d,y
        ldb     #3
        bra     BASE_PREFIX
BASE_HEXADECIMAL
        ldb     #4
BASE_PREFIX
        leax    1,x
BASE_BEGIN
        pshs    b
BASE_DIGIT
        jsr     EX_UPCASE
        suba    #'0
        blo     BASE_END
        cmpa    #9
        bls     BASE_NUMERAL
        suba    #7
        cmpa    #10
        blo     BASE_END
        cmpa    #16
        bhs     BASE_END
        ldb     ,s
        cmpb    #4
        bne     BASE_END
        bra     BASE_HAVE_DIGIT
BASE_NUMERAL
        ldb     ,s
        cmpb    #4
        beq     BASE_HAVE_DIGIT
        cmpa    #8
        jsr     NATIVE_CHECK_HS
BASE_HAVE_DIGIT
        pshs    a
        ldb     1,s
        stb     TEMP
        tfr     y,d
BASE_SHIFT
        lslb
        rola
        jsr     ST_CHECK
        dec     TEMP
        bne     BASE_SHIFT
        addb    ,s+
        adca    #0
        jsr     ST_CHECK
        tfr     d,y
        leax    1,x
        bra     BASE_DIGIT
BASE_END
        leas    1,s
        tfr     y,d
        puls    y
        jmp     UNSIGNED_TO_FAC

; Optional adjacent USR digit, shared by calls and DEF USR.
USR_INDEX
        ldb     ,x
        subb    #'0
        cmpb    #9
        bhi     USR_INDEX_NONE
        leax    1,x
        lslb
        rts
USR_INDEX_NONE
        clrb
        rts

; Double the 40 integer digits, adding the input carry. DAA is the native
; packed-decimal correction; loads, stores and DEC retain each byte's carry.
FORMAT_DOUBLE
        leax    20,u
        ldb     #20
FORMAT_DOUBLE_BYTE
        lda     ,-x
        adca    ,x
        daa
        sta     ,x
        decb
        bne     FORMAT_DOUBLE_BYTE
        rts
; The caller's byte 1 tracks the next nibble; the return adds two stack bytes.
FORMAT_BCD_DIGIT
        lda     ,x
        com     3,s
        beq     FORMAT_BCD_LOW
        lsra
        lsra
        lsra
        lsra
        rts
FORMAT_BCD_LOW
        leax    1,x
        anda    #15
        rts

