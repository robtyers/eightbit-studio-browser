LOCAL_FAILURE_5
        jmp     CMD_UNSUPPORTED
CMD_RESTORE
        jsr     CMD_REQUIRE_END
        ; Native tail continues into the following block.
; Original DATA/READ/RESTORE cursor. Public $31/$33 retain the last DATA line
; and its delimiter; the next linked-line header is an implementation detail.
DATA_RESET
        ldd     TXTTAB
        std     DATA_NEXT
        subd    #1
        std     $33
        rts
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
        jsr     EX_OPTIONAL_COMMA
        bne     DATA_SKIP_END
        bra     CMD_READ
DATA_FETCH
        pshs    x,y,u
        ; The published reset cursor is TXTTAB-1; no duplicate active flag.
        ldx     $33
        leax    1,x
        cmpx    TXTTAB
        beq     DATA_NEXT_LINE
        lda     -1,x
        cmpa    #',
        beq     DATA_ITEM
        tsta
        bne     DATA_SCAN_START
DATA_NEXT_LINE
        ldy     DATA_NEXT
        ldd     ,y
        beq     DATA_EXHAUSTED
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
        clra
        puls    x,y,u,pc
DATA_EXHAUSTED
        ldb     #6
        stb     ERROR_CODE
DATA_ERROR
        coma
        puls    x,y,u,pc

; Parse one typed CSV value at X, returning its delimiter in X. DATA, keyboard
; input and cassette data can share it without borrowing each other's cursor.

; One transient mode selects DATA (zero), keyboard (negative), or file
; (positive) fields. Keyboard accepts empty numeric fields; file mode keeps
; its whole-record delimiter, whitespace and failure rules. Destination
; registers are retained in every mode.
DATA_KEYBOARD_VALUE
        ldb     #-1
        bra     DATA_VALUE_MODE
DATA_FILE_VALUE
        ldb     #1
        bra     DATA_VALUE_MODE
DATA_VALUE
        clrb
DATA_VALUE_MODE
        pshs    b,y,u
        jsr     EX_SPACE
        tst     $06
        bne     DATA_STRING
        tsta
        bne     DATA_SIGNED_NUMBER
        tst     ,s
        beq     DATA_VALUE_ERROR
DATA_ZERO_NUMBER
        jsr     FAC_ZERO
        bra     DATA_DELIMITER
DATA_SIGNED_NUMBER
        tst     ,s
        bpl     DATA_SIGN_VALUE
        cmpa    #',
        beq     DATA_ZERO_NUMBER
        cmpa    #':
        beq     DATA_ZERO_NUMBER
DATA_SIGN_VALUE
        jsr     ASCII_SIGN
DATA_NUMBER
        jsr     NUMBER_SIGNED
        bcs     DATA_VALUE_ERROR
        bra     DATA_DELIMITER
DATA_STRING
        cmpa    #'"
        bne     DATA_UNQUOTED
        ; String construction returns success or unwinds its native owner.
        jsr     STRING_LITERAL
        bra     DATA_DELIMITER
DATA_UNQUOTED
        tfr     x,u
        clrb
DATA_STRING_BYTE
        lda     ,x
        beq     DATA_TRIM
        tst     ,s
        bgt     DATA_STRING_MORE
        cmpa    #',
        beq     DATA_TRIM
        cmpa    #':
        beq     DATA_TRIM
DATA_STRING_MORE
        leax    1,x
        incb
        bne     DATA_STRING_BYTE
        ; Native tail continues into the following block.
DATA_VALUE_ERROR
        tst     ,s
        lbgt     DATA_BAD_VALUE
        coma
        puls    b,y,u,pc
DATA_TRIM
        pshs    x
        tst     2,s
        bgt     DATA_STRING_VALUE
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
        puls    x
DATA_DELIMITER
        tst     ,s
        bgt     DATA_VALUE_FOUND
        jsr     EX_SPACE
        tsta
        beq     DATA_VALUE_FOUND
        cmpa    #':
        beq     DATA_VALUE_FOUND
        cmpa    #',
        bne     DATA_VALUE_ERROR

DATA_VALUE_FOUND
        clra
        puls    b,y,u,pc
