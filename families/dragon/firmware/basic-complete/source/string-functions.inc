; Original multi-argument string functions. Pending arguments retain public
; descriptor identities across later allocation, collection and array movement.
EX_MULTI_STRING
        leas    -10,s
        tfr     s,u
        sta     7,u
        ldb     #1
        tsta
        bmi     MULTI_START_READY
        ldb     TEMP_FREE+1
MULTI_START_READY
        stb     8,u
        jsr     EX_OPEN
        bsr     NATIVE_ISLAND_STRING_FUNCTIONS_INC_0
        lda     7,u
        bpl     MULTI_SLICE_FIRST
        cmpa    #$a0
        lbeq    MULTI_REPEAT
        cmpa    #$9d
        bne     MULTI_SLICE_FIRST
        tst     $06
        bne     MULTI_FIRST_STRING
        bsr     NATIVE_ISLAND_STRING_FUNCTIONS_INC_1
        tstb
        lbeq    NATIVE_RANGE_ERROR
        stb     8,u
        jsr     MULTI_COMMA
        bsr     NATIVE_ISLAND_STRING_FUNCTIONS_INC_0
        bra     MULTI_FIRST_STRING
MULTI_SLICE_FIRST
        ; Slice syntax selects its comma before checking the first value's
        ; type. INSTR deliberately retains its distinct argument ordering.
        jsr     EX_COMMA
        jsr     NATIVE_CHECK_NE
MULTI_FIRST_STRING
        jsr     MULTI_CAPTURE_ARGUMENT
        lda     7,u
        lbpl    MULTI_REBOUND_SLICE
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
        jsr     VALUE_CAPTURED_REFERENCE
        jsr     MULTI_CLOSE
        clrb
        lda     7,u
        cmpa    #$97
        bne     MULTI_LEFT_OR_MID
        ldb     ,y
        subb    9,u
        bhs     MULTI_SLICE
        clrb
        bra     MULTI_SLICE
NATIVE_ISLAND_STRING_FUNCTIONS_INC_0
        jmp     EX_BODY
PARSE_FAILURE_19
        jmp     NATIVE_RANGE_ERROR
NATIVE_ISLAND_STRING_FUNCTIONS_INC_1
        ; Native tail continues into the following block.

MULTI_BYTE
        jsr     EX_TO_ADDRESS
        tsta
        lbne    NATIVE_RANGE_ERROR
        rts
MULTI_LEFT_OR_MID
        cmpa    #$98
        bne     MULTI_SLICE
        ldb     8,u
        decb
MULTI_SLICE
        cmpb    ,y
        blo     MULTI_SLICE_LENGTH
        clra
        bra     MULTI_SLICE_READY
MULTI_SLICE_LENGTH
        lda     ,y
        pshs    b
        suba    ,s+
        cmpa    9,u
        bls     MULTI_SLICE_READY
        lda     9,u
MULTI_SLICE_READY
        sta     9,u
        stb     8,u
        bra     MULTI_ALLOCATE

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
        jsr     STRING_RELEASE
        bra     MULTI_REPEAT_BYTE
MULTI_REPEAT_NUMBER
        bsr     NATIVE_ISLAND_STRING_FUNCTIONS_INC_1
MULTI_REPEAT_BYTE
        stb     8,u
        clr     $06
        jsr     MULTI_CLOSE
MULTI_ALLOCATE
        ; One result allocation owns both fill continuations. The descriptor
        ; is refreshed only after collection and only for a slice source.
        tfr     u,y
        clra
        ldb     9,u
        jsr     STRING_ALLOC
        lda     7,y
        cmpa    #$a0
        bne     MULTI_COPY_SLICE
        pshs    u
        lda     8,y
        tstb
        beq     MULTI_REPEAT_DONE
MULTI_REPEAT_FILL
        sta     ,u+
        decb
        bne     MULTI_REPEAT_FILL
MULTI_REPEAT_DONE
        puls    u
MULTI_NEW_VALUE
        lda     9,y
        jsr     STRING_NEW_VALUE
        bra     MULTI_SUCCESS
MULTI_COPY_SLICE
        pshs    x,y,u
        tfr     y,u
        jsr     VALUE_CAPTURED_REFERENCE
        clra
        ldb     8,u
        addd    2,y
        tfr     d,x
        ldb     9,u
        ldu     4,s
        jsr     STRING_COPY
        puls    x,y,u
        ldb     3,y
        jsr     STRING_TEMP_REWIND
        bra     MULTI_NEW_VALUE

MULTI_SEARCH
        jsr     VALUE_CAPTURED_REFERENCE
        jsr     EX_STRING_CHECK
        jsr     MULTI_CLOSE
        pshs    x
        ldb     8,u
        decb
        cmpb    ,y
        bhi     MULTI_NOT_FOUND
        lda     ,y
        sta     9,u
        ldx     2,y
        clra
        leax    d,x
MULTI_SEARCH_POSITION
        lda     9,u
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
        ; Both search operands are consumed by this numeric result. Slice and
        ; repeat paths already replace their inputs with one result descriptor.
        ldb     3,u
        jsr     STRING_TEMP_REWIND
        puls    x
MULTI_SUCCESS
        clra
MULTI_RETURN
        leas    10,s
        rts

; Native and replacement slice functions retain the same first descriptor
; while evaluating the next argument. The caller owns the U capture frame.
MULTI_CAPTURE_ARGUMENT
        jsr     EX_STRING_CHECK
        jsr     VALUE_CAPTURE
        jsr     MULTI_COMMA
        jmp     EX_BODY

; Native and replacement slices share argument ownership and grammar. The
; public callback still owns its count/descriptor bytes below our frame.
MULTI_REBOUND_SLICE
        jsr     PUBLIC_FAC_TO_INT
        tsta
        lbne    NATIVE_RANGE_ERROR
        stb     9,u
        ; The accepted byte is nonnegative; publish the legacy sign byte,
        ; not unused low sign bits retained by private floating arithmetic.
        clr     FSIGN
        stx     TXT_PTR
        jsr     VALUE_CAPTURED_REFERENCE
        pshs    y
        lda     7,u
        ldb     9,u
        pshs    b
        ldx     $0128
        leax    a,x
        tfr     a,b
        ldu     #0
        jsr     [,x]
        ; A conforming replacement consumed its own three argument bytes.
        ldb     8,s
        jsr     STRING_TEMP_REWIND
        ldx     TXT_PTR
        jsr     EX_NUMBER_CHECK
        bra     MULTI_RETURN
