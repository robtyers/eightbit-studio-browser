; Original guest conversions and non-parenthesised/device functions.
EX_INKEY
        jsr     $bbe5
        beq     EX_INKEY_EMPTY
        tfr     a,b
        jsr     STRING_BYTE
        rts
EX_INKEY_EMPTY
        ldu     #0
        clra
        jsr     STRING_TEMP_VALUE
        rts
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
        puls    b
        ldy     USR_TAB
        ldy     b,y
        pshs    x
        ldx     #FAC
        tst     $06
        beq     EX_USR_CALL
        ldx     STR_DESC
        ; USR consumes its argument before entering machine code. The released
        ; descriptor bytes remain the public X/$52 argument representation.
        jsr     STRING_RELEASE
        clr     STR_PTR
        clr     STR_PTR+1
EX_USR_CALL
        jsr     ,y
        jsr     EX_NUMBER_CHECK
        puls    x,pc
VALUE_HEX
        jsr     EX_TO_ADDRESS
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
        rts

VALUE_STR
        ; Both consumers receive the same completed numeric span. Allocate
        ; STR$ directly from it while the formatter's native frame remains live.
        pshs    d,x,y,u
        ldy     #VALUE_COPY
        ldb     #UF_PLUS
        jmp     FORMAT_CONSUMER_ENTRY
; D count, X stack source. Allocation may collect; stack source remains stable.
VALUE_COPY
        pshs    d
        clr     $06
        jsr     STRING_ALLOC
        pshs    u
        jsr     STRING_COPY
        puls    u
        puls    d
        tfr     b,a
        jmp     STRING_NEW_VALUE
VALUE_POS
        ldb     DEVNUM
        pshs    b
        jsr     DATA_DEVICE_FAC
        tstb
        beq     VALUE_POS_SCREEN
        incb
        beq     VALUE_POS_CASSETTE
        ldb     $9c
        bra     VALUE_POS_RESULT
VALUE_POS_SCREEN
        ldb     CURSOR+1
        andb    #31
VALUE_POS_RESULT
        stb     $6c
        puls    a
        sta     DEVNUM
NATIVE_TAIL_14
        clra
        jmp     INT_TO_FAC
; A cassette position is zero once output is open. The public mode byte
; distinguishes only closed and input here; other nonzero modes are accepted.
VALUE_POS_CASSETTE
        lda     $78
        lbeq    DATA_NOT_OPEN
        deca
        lbeq    DATA_MODE_ERROR
        clrb
        bra     VALUE_POS_RESULT
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

VALUE_EOF equ DATA_EOF

EX_BASE_NUMBER
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

