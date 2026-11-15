; Original DATA/READ/RESTORE cursor. Public $31/$33 retain the last DATA line
; and its delimiter; the next linked-line header is an implementation detail.
DATA_RESET
        ldd     TXTTAB
        std     DATA_NEXT
        subd    #1
        std     $33
        clr     DATA_ACTIVE
        rts
LOCAL_FAILURE_5
        jmp     CMD_UNSUPPORTED
CMD_RESTORE
        jsr     CMD_REQUIRE_END
        bra     DATA_RESET
CMD_DATA
        clrb
DATA_SKIP
        lda     ,x
        beq     DATA_SKIP_END
        jsr     TEXT_CONTEXT
        bcs     DATA_SKIP_BYTE
        cmpa    #':
        beq     DATA_SKIP_END
DATA_SKIP_BYTE
        leax    1,x
        bra     DATA_SKIP
DATA_SKIP_END
        clra
        rts
CMD_READ
        jsr     VAR_FIND_NATIVE
        bsr     DATA_FETCH
        bcs     LOCAL_FAILURE_5
        jsr     VALUE_STORE
        bcs     LOCAL_FAILURE_5
        jsr     EX_OPTIONAL_COMMA
        bne     DATA_SKIP_END
        bra     CMD_READ
DATA_FETCH
        pshs    x,y,u
        tst     DATA_ACTIVE
        beq     DATA_NEXT_LINE
        ldx     $33
        lda     ,x
        leax    1,x
        cmpa    #',
        beq     DATA_ITEM
        tsta
        bne     DATA_SCAN_START
DATA_NEXT_LINE
        ldy     DATA_NEXT
        ldd     ,y
        beq    DATA_ERROR
        std     DATA_NEXT
        ldd     2,y
        std     DATA_LINE
        leax    4,y
DATA_SCAN_START
        clrb
DATA_SCAN
        lda     ,x+
        beq     DATA_NEXT_LINE
        jsr     TEXT_CONTEXT
        bcs     DATA_SCAN

DATA_SCAN_STATEMENT
        cmpa    #$86
        bne     DATA_SCAN
        ldd     DATA_LINE
        std     $31
DATA_ITEM
        bsr     DATA_VALUE
        bcs     DATA_ERROR
DATA_FOUND
        stx     $33
        lda     #1
        sta     DATA_ACTIVE
        clra
        puls    x,y,u,pc
DATA_ERROR
        coma
        puls    x,y,u,pc

; Parse one typed CSV value at X, returning its delimiter in X. DATA, keyboard
; input and cassette data can share it without borrowing each other's cursor.

DATA_VALUE
        pshs    y,u
        jsr     EX_SPACE
        tst     $06
        bne     DATA_STRING
        jsr     ASCII_SIGN
DATA_NUMBER
        jsr     NUMBER_SIGNED
        bcs     DATA_VALUE_ERROR
        bra     DATA_DELIMITER

DATA_STRING
        cmpa    #'"
        bne     DATA_UNQUOTED
        jsr     STRING_LITERAL
        bcs     DATA_VALUE_ERROR
        bra     DATA_DELIMITER
DATA_UNQUOTED
        tfr     x,u
        clrb
DATA_STRING_BYTE
        lda     ,x
        beq     DATA_TRIM
        cmpa    #',
        beq     DATA_TRIM
        cmpa    #':
        beq     DATA_TRIM
        leax    1,x
        incb
        bne     DATA_STRING_BYTE
        bra     DATA_VALUE_ERROR
DATA_TRIM
        pshs    x
DATA_TRAILING_SPACE
        tstb
        beq     DATA_STRING_VALUE
        lda     -1,x
        cmpa    #32
        bne     DATA_STRING_VALUE
        leax    -1,x
        decb
        bra     DATA_TRAILING_SPACE
DATA_STRING_VALUE
        tfr     b,a
        jsr     STRING_VALUE
        lda     #2
        sta     STR_OWNER
        puls    x
DATA_DELIMITER
        jsr     EX_SPACE
        tsta
        beq     DATA_VALUE_FOUND
        cmpa    #':
        beq     DATA_VALUE_FOUND
        cmpa    #',
        bne     DATA_VALUE_ERROR
DATA_VALUE_FOUND
        clra
        puls    y,u,pc
DATA_VALUE_ERROR
        coma
        puls    y,u,pc

