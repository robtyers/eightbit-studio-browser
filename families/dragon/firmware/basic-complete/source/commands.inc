; Original guest statement execution. Apache-2.0.
EXECUTE_DIRECT
        lda     #1
        sta     EXEC_ORIGIN
        lds     STACKTOP
        leas    1,s
        sts     EXEC_SAVED
; The executor owns U between statements. Native boundaries preserve it;
; control-stack/machine-code returns explicitly restore this private base.
EXEC_START
        jsr     STRING_TEMP_RESET
        ldu     #$03e8
        clra
        clrb
        tst     CONT_VALID-$03e8,u
        beq     EXEC_NEW_CONTROL
        lds     CONT_SP-$03e8,u
        sts     CTL_SP-$03e8,u
        bra     EXEC_CONTROL_READY
EXEC_NEW_CONTROL
        pshs    d
        sts     CTL_BASE-$03e8,u
        sts     CTL_SP-$03e8,u
EXEC_CONTROL_READY
        clr     FORMAT_BUFFER-$03e8,u
        std     CTL_RESET-$03e8,u
        clr     EXEC_MODE-$03e8,u
        jsr     DIRECT_LINE
EXEC_LOOP
EXEC_TOKEN
        bsr     NATIVE_SHARED12_COMMANDS_INC_0
        stx     TXT_PTR
        stx     $2d
        tsta
        beq     EXEC_LINE_END
        cmpa    #':
        beq     EXEC_SEPARATOR
        jsr     STATEMENT
EXEC_AFTER_STATEMENT
        pshs    cc
        jsr     STRING_TEMP_RESET
        puls    cc
        ldu     #$03e8
        ldb     EXEC_MODE-$03e8,u
        decb
        bne     EXEC_CHECK_ERROR
        clr     EXEC_MODE-$03e8,u
EXEC_CHECK_ERROR
        ldx     TXT_PTR
        bcs     EXEC_ERROR
        ; A failed command can request NEW/CLEAR recovery. Observe its carry
        ; before CLR and control-stack replacement clear that condition flag.
        tst     CTL_RESET-$03e8,u
        beq     EXEC_CHECK_MODE
        clr     CTL_RESET-$03e8,u
        lds     CTL_BASE-$03e8,u
        clra
        clrb
        std     ,s
        sts     CTL_SP-$03e8,u
EXEC_CHECK_MODE
        tst     EXEC_MODE-$03e8,u
        bmi     EXEC_COMPLETE
        beq     EXEC_CHECK_SEPARATOR
        clr     EXEC_MODE-$03e8,u
        bra     EXEC_LOOP
EXEC_CHECK_SEPARATOR
        jsr     EXEC_NEXT_HOOK
        bsr     NATIVE_SHARED12_COMMANDS_INC_0
        tsta
        beq     EXEC_LINE_END
        cmpa    #':
        beq     EXEC_SEPARATOR
        cmpa    #$84
        bne     EXEC_ERROR
        jsr     CMD_REM
        bra     EXEC_CHECK_SEPARATOR
NATIVE_SHARED12_COMMANDS_INC_0
        jmp     EX_SPACE
EXEC_SEPARATOR
        leax    1,x
        bra     EXEC_LOOP
EXEC_LINE_END
        ldx     EXEC_NEXT-$03e8,u
        beq     EXEC_COMPLETE
        ldd     ,x
        beq     EXEC_PROGRAM_FINISHED
        std     EXEC_NEXT-$03e8,u
        ldd     2,x
        std     CURLINE
        leax    4,x
        jsr     TRACE_LINE
        bra     EXEC_LOOP
EXEC_PROGRAM_FINISHED
        clr     CONT_VALID-$03e8,u
        ldd     CTL_BASE-$03e8,u
        std     CTL_SP-$03e8,u
EXEC_COMPLETE
        clr     EXEC_MODE-$03e8,u
        jsr     DIRECT_LINE
        tst     EXEC_ORIGIN-$03e8,u
        beq     EXEC_RETURN_CALLER
        lds     CTL_SP-$03e8,u
        jmp     DIRECT
EXEC_RETURN_CALLER
        lds     EXEC_SAVED-$03e8,u
        puls    y,u,pc
EXEC_ERROR
        ldu     #$03e8
        lds     EXEC_SAVED-$03e8,u
        clr     CONT_VALID-$03e8,u
        clr     EXEC_MODE-$03e8,u
        coma
        tst     EXEC_ORIGIN-$03e8,u
        lbne     DIRECT_ERROR
        puls    y,u,pc

EXECUTE_STATEMENT
        pshs    d,y,u
        jsr     NATIVE_INLINE
        jsr     STATEMENT_DISPATCH
        pshs    cc
        ldb     8,s
        jsr     STRING_TEMP_REWIND
        puls    cc
        stx     TXT_PTR
        rts
STATEMENT_FAIL equ NATIVE_FAIL
STATEMENT_DISPATCH
        ldb     #2
        stb     ERROR_CODE
        bsr     NATIVE_SHARED12_COMMANDS_INC_0
        cmpa    #$ff
        beq     CMD_LET
        cmpa    #$80
        blo     CMD_LET
        cmpa    #$ce
        bhs     STATEMENT_EXTENSION
        cmpa    #$ba
        lbhi    NATIVE_FAIL
        leax    1,x
        suba    #$80
        lsla
        ldu     $0123
        ; Copying the public table does not turn unchanged native entries into
        ; extension callbacks with a different X/U calling convention.
        pshs    x
        ldx     #STATEMENT_TABLE
        ldx     a,x
        cmpx    a,u
        puls    x
        bne     CALL_REBOUND_TABLE
        jmp     [a,u]
CALL_REBOUND_TABLE
        tfr     a,b
        stx     TXT_PTR
        leax    a,u
        ldu     TXT_PTR
        lda     ,u
        jsr     [,x]
        ldx     TXT_PTR
        clra
        rts
STATEMENT_EXTENSION
        stx     TXT_PTR
        jsr     [$012d]
        ldx     TXT_PTR
        clra
        rts
CMD_UNSUPPORTED equ NATIVE_FAIL
CMD_LET
        lbsr     NATIVE_SHARED12_COMMANDS_INC_0
        cmpa    #$ff
        bne     CMD_LET_VARIABLE
        ldb     1,x
        cmpb    #$9e
        beq     CMD_TIMER
        jmp     CMD_MID
CMD_LET_VARIABLE
        jsr     VAR_FIND_NATIVE
        jsr     VARIABLE_REFERENCE
        pshs    d
        lda     $06
        pshs    a
        jsr     EX_EQUALS
        jsr     EVAL_NATIVE
        lda     $06
        cmpa    ,s+
        lbne     NATIVE_TYPE_ERROR
        tfr     s,u
        jsr     VALUE_REFERENCE
        leas    2,s
CMD_LET_ADDRESS_READY
        jmp     VALUE_STORE
CMD_TIMER
        leax    2,x
        jsr     EX_EQUALS
        jsr     EVAL_ADDRESS
        pshs    d
        bsr     NATIVE_ISLAND_COMMANDS_INC_1
        puls    d
        clr     TIMER
        std     TIMER+1
        rts

CMD_STOP
        jsr     CMD_REQUIRE_END
        jsr     BREAK_MESSAGE
        bra     CMD_END_VALIDATED
; Retain the builder name as an alias to the authoritative END lifecycle.
CMD_END equ DATA_END
CMD_END_VALIDATED
        ldd     EXEC_NEXT
        beq     CMD_END_MODE
        ldd     CURLINE
        std     $29
        stx     $2f
        ldd     EXEC_NEXT
        std     CONT_NEXT
        ldd     CTL_SP
        std     CONT_SP
        lda     #1
        sta     CONT_VALID
CMD_END_MODE
        lda     #$ff
        sta     EXEC_MODE
        clra
        rts
CMD_NEW
        bsr     NATIVE_ISLAND_COMMANDS_INC_1
NEW_PROGRAM
        ldy     TXTTAB
        clr     ,y
        clr     1,y
        leay    2,y
        sty     VARTAB
        bra     CMD_CLEAR_VARIABLES
CMD_CLEAR
        jsr     EX_END
        beq     CMD_CLEAR_VARIABLES
        jsr     EVAL_ADDRESS
        pshs    d
        jsr     EX_OPTIONAL_COMMA
        bne     CMD_CLEAR_CURRENT_LIMIT
        jsr     EVAL_ADDRESS
        tsta
        lbmi    NATIVE_MEMORY_ERROR
        subd    #1
        lbcs    NATIVE_MEMORY_ERROR
        bra     CMD_CLEAR_VALIDATE_LIMIT
NATIVE_ISLAND_COMMANDS_INC_1
        jmp     CMD_REQUIRE_END
CMD_CLEAR_CURRENT_LIMIT
        ldd     MEMTOP
CMD_CLEAR_VALIDATE_LIMIT
        pshs    d
        bsr     NATIVE_ISLAND_COMMANDS_INC_1
        ldd     ,s
        subd    2,s
        lbcs    NATIVE_MEMORY_ERROR
        pshs    d
        ldd     VARTAB
        addd    #128
        cmpd    ,s++
        lbhs    NATIVE_MEMORY_ERROR
        ldd     ,s
        std     MEMTOP
        subd    2,s
        std     STACKTOP
        leas    4,s

CMD_CLEAR_VARIABLES
        jsr     STRING_TEMP_RESET
        ldd     VARTAB
        std     ARYTAB
        std     STREND
        ldd     MEMTOP
        std     FRETOP
        clr     CONT_VALID
        clra
        clrb
        std     $2f
        std     $31
        jsr     DATA_RESET
        tst     EXEC_ORIGIN
        beq     CMD_CLEAR_CONTROL
        ldd     STACKTOP
        addd    #1
        std     EXEC_SAVED
        subd    #2
        std     CTL_BASE
CMD_CLEAR_CONTROL
        lda     #1
        sta     CTL_RESET
        clra
        rts
CMD_RUN
        jsr     EX_SPACE
        stx     TXT_PTR
        ; Recreate EX_SPACE flags exactly after publishing the hook cursor.
        cmpa    #32
        jsr     $0194
        ldx     TXT_PTR
        jsr     GFX_RUNTIME_RESET
        jsr     EX_END
        beq     CMD_RUN_ALL
CMD_RUN_LINE
        jsr     PARSE_LINE_NUMBER_OR_ZERO
        jsr     ST_CHECK
        jsr     CTL_FIND_LINE
        bcs     LOCAL_FAILURE_2
        ; Variable/data reset leaves the selected programme record in Y.
        bsr     CMD_CLEAR_VARIABLES
        jmp     CTL_GO_TARGET
CMD_RUN_ALL
        jsr     VALIDATE_TEXT
        bcs     LOCAL_FAILURE_2
        bsr     CMD_CLEAR_VARIABLES
        jsr     DIRECT_LINE
        ldd     TXTTAB
        std     EXEC_NEXT
        ldx     TXTTAB
        leax    -1,x
        rts
LOCAL_FAILURE_2
        jmp     CMD_UNSUPPORTED
CMD_GO
        bsr     NATIVE_ISLAND_COMMANDS_INC_2
        cmpa    #$bd
        lbeq     CMD_GOSUB
        cmpa    #$bc
        bne     LOCAL_FAILURE_2
        jsr     EX_NEXT_SPACE
NATIVE_TAIL_3
        jsr     ST_LINE_NUMBER
        jmp     CTL_GO_TARGET_FROM_NUMBER

CMD_IF
        jsr     EVAL_NUMBER
        bsr     NATIVE_ISLAND_COMMANDS_INC_2
        cmpa    #$bf
        bne     LOCAL_FAILURE_2
        leax    1,x
        tst     FAC
        beq     CMD_IF_FALSE
        bsr     NATIVE_ISLAND_COMMANDS_INC_2
        cmpa    #'0
        blo     CMD_IF_STATEMENT
        cmpa    #'9
        bhi     CMD_IF_STATEMENT
        bra     NATIVE_TAIL_3
CMD_IF_STATEMENT
        lda     #2
        sta     EXEC_MODE
        clra
        rts
LOCAL_FAILURE_25
        clra
        rts
CMD_IF_FALSE
        clrb
        ldu     #0
CMD_IF_SKIP
        lda     ,x
        beq     LOCAL_FAILURE_25
        leax    1,x
        jsr     TEXT_CONTEXT
        bcs     CMD_IF_SKIP
CMD_IF_SKIP_STATEMENT
        cmpa    #$85
        bne     CMD_IF_ELSE
        leau    1,u
        bra     CMD_IF_SKIP
CMD_IF_ELSE
        cmpa    #$84
        bne     CMD_IF_SKIP
        leay    ,u
        beq     CMD_IF_STATEMENT
        leau    -1,u
        bra     CMD_IF_SKIP
NATIVE_ISLAND_COMMANDS_INC_2
        jmp     EX_SPACE




; Typed numeric expression with the token cursor preserved across conversion.
EX_ADDRESS
        jsr     EX_BODY
        bra     EX_TO_ADDRESS
EVAL_NEXT_ADDRESS
        jsr     MULTI_COMMA
EVAL_ADDRESS
        jsr     EVAL_NATIVE
EX_TO_ADDRESS
        jsr     EX_NUMBER_CHECK
        jsr     FAC_TO_ADDRESS
        lbcs     NATIVE_RANGE_ERROR
EVAL_ADDRESS_END
        rts
CMD_POKE
        bsr     EVAL_ADDRESS
        pshs    d
        jsr     GFX_NEXT_BYTE
        jsr     CMD_REQUIRE_END
        puls    y
        stb     ,y
        rts
; A completed PRINT returns to screen output. Language failures bypass this
; success tail and retain the enclosing native frame's original device.
CMD_PRINT
        jsr     PRINT_NATIVE
CMD_PRINT_SUCCESS
        clr     DEVNUM
        rts
PRINT_NATIVE
        bsr     NATIVE_ISLAND_COMMANDS_INC_2
        jsr     BYTE_EQUAL_DISPATCH
        fcb     2
        fcb     $23
        fdb     PRINT_DEVICE
        fcb     $40
        fdb     PRINT_AT
PRINT_BODY
        jsr     EX_END_CHAR
        beq     CMD_PRINT_NEWLINE
; Only the initial item accepts USING. Separators and completed values resume
; the ordinary item list without reopening the device, position or format.
PRINT_FIRST
        cmpa    #$cd
        lbeq    PRINT_USING
CMD_PRINT_NUMBER
        jsr     BYTE_EQUAL_DISPATCH
        fcb     3
        fcb     $bb
        fdb     PRINT_TAB
        fcb     $3b
        fdb     CMD_PRINT_MORE
        fcb     $2c
        fdb     CMD_PRINT_ZONE
        jsr     EVAL_NATIVE
        tst     $06
        bne     CMD_PRINT_VALUE_STRING
        ; The cached column is measured before PRINT's trailing number blank.
        jsr     PRINT_DATA_NUMBER
        jsr     PRINT_COLUMN
        ldb     DEVNUM
        incb
        beq     CMD_PRINT_FILE_RECORD
        ; A comma immediately after a number uses its assumed trailing-blank
        ; column, even if a callback discards that blank. A separator or TAB
        ; starts a fresh column read, and the public cache remains pre-blank.
        inca
        pshs    a
        lda     #32
        jsr     CHROUT
        jsr     EX_SPACE
        cmpa    #',
        puls    a
        beq     CMD_PRINT_ZONE_KNOWN
        ; Native tail continues into the following block.
CMD_PRINT_END
        bsr     NATIVE_ISLAND_COMMANDS_INC_2
        jsr     EX_END_CHAR
        bne     CMD_PRINT_NUMBER
        ldb     DEVNUM
        incb
        beq     CMD_OK
CMD_PRINT_NEWLINE
        lda     #13
        lbsr     NATIVE_ISLAND_COMMANDS_INC_3
        clra
        rts
CMD_PRINT_VALUE_STRING
        jsr     STRING_PRINT
        jsr     PRINT_COLUMN
        ldb     DEVNUM
        incb
        bne     CMD_PRINT_END
CMD_PRINT_FILE_RECORD
        lda     #13
        jsr     CHROUT
        jsr     EX_END
        beq     CMD_OK
        bra     CMD_PRINT_NUMBER
CMD_PRINT_ZONE
        jsr     PRINT_COLUMN
CMD_PRINT_ZONE_KNOWN
        ldb     DEVNUM
        cmpb    #$ff
        beq     CMD_PRINT_MORE
        tstb
        beq     CMD_PRINT_SCREEN_ZONE
        cmpa    $9a
        bhs     CMD_PRINT_ZONE_CR
        ldb     $99
        beq     CMD_PRINT_MORE
CMD_PRINT_ZONE_REMAINDER
        suba    $99
        bcc     CMD_PRINT_ZONE_REMAINDER
        nega
        tfr     a,b
CMD_PRINT_PRINTER_SPACE
        lda     #32
        bsr     NATIVE_ISLAND_COMMANDS_INC_3
        decb
        bne     CMD_PRINT_PRINTER_SPACE
        ; Native tail continues into the following block.
CMD_PRINT_MORE
        leax    1,x
        jsr     EX_END
        beq     CMD_OK
        bra     CMD_PRINT_NUMBER
CMD_PRINT_ZONE_CR
        lda     #13
        bsr     NATIVE_ISLAND_COMMANDS_INC_3
        bra     CMD_PRINT_MORE
CMD_PRINT_SCREEN_ZONE
        anda    #15
        nega
        adda    #16
        tfr     a,b
        ; Both devices already resolved the bounded number of spaces in B.
        bra     CMD_PRINT_PRINTER_SPACE
DIRECT_LINE
        clra
        clrb
        std     EXEC_NEXT
        ldd     #$ffff
        std     CURLINE
CMD_OK
        clra
        rts
CMD_LLIST
        ldu     #CMD_LIST
        ldb     #$fe
        jmp     PRINT_SELECTED

CMD_LIST
        pshs    x,y,u
        leas    -4,s
        tfr     s,u
        jsr     LINE_RANGE
        jsr     ST_CHECK
        stx     4,s
        jsr     VALIDATE_PROGRAM
        jsr     ST_CHECK
        ; Both range parsing and validation preserve this frame owner.
        ldy     TXTTAB
CMD_LIST_LINE
        ldd     ,y
        beq     CMD_LIST_END
        ldd     2,y
        cmpd    2,u
        bhi     CMD_LIST_END
        cmpd    ,u
        blo     CMD_LIST_NEXT
        pshs    y
        ldd     2,y
        jsr     PRINT_UNSIGNED
        lda     #32
        bsr     NATIVE_ISLAND_COMMANDS_INC_3
        leax    4,y
        bsr     DETOKENISE_LINE
        lda     DEVNUM
        cmpa    #$fd
        beq     CMD_LIST_NO_CR
        lda     #13
        bsr     NATIVE_ISLAND_COMMANDS_INC_3
CMD_LIST_NO_CR
        puls    y
CMD_LIST_NEXT
        ldy     ,y
        bra     CMD_LIST_LINE
NATIVE_ISLAND_COMMANDS_INC_3
        jmp     CHROUT
CMD_LIST_END
        leas    4,s
        clra
        puls    x,y,u,pc
DETOKENISE_LINE
        jsr     $01a6
        clrb
LIST_BYTE
        lda     ,x+
        beq     LIST_END
        jsr     TEXT_CONTEXT
        bcs     LIST_OUT
        ; The tokeniser adds a structural colon before ELSE and apostrophe.
        ; Listing suppresses only that colon, preserving user text and DATA.
        cmpa    #':
        bne     LIST_TOKEN
        lda     ,x
        suba    #$83
        cmpa    #1
        bls     LIST_BYTE
        lda     #':
LIST_TOKEN
        cmpa    #$ff
        beq     LIST_FUNCTION
        cmpa    #$80
        lblo    LIST_OUT
        ldy     #$0120
LIST_KEYWORD
        pshs    b
        suba    #$80
LIST_DESCRIPTOR
        tst     ,y
        beq     LIST_UNKNOWN
        cmpa    ,y
        blo     LIST_DESCRIPTOR_FOUND
        suba    ,y
        leay    10,y
        bra     LIST_DESCRIPTOR
LIST_DESCRIPTOR_FOUND
        ldy     1,y
        tfr     a,b
LIST_SKIP_WORD
        tstb
        beq     LIST_WORD
LIST_SKIP_CHAR
        lda     ,y+
        bpl     LIST_SKIP_CHAR
        decb
        bra     LIST_SKIP_WORD
LIST_WORD
        lda     ,y+
        pshs    a
        anda    #$7f
        bsr     NATIVE_ISLAND_COMMANDS_INC_3
        puls    a
        tsta
        bpl     LIST_WORD
        puls    b
        bra     LIST_BYTE
LIST_FUNCTION
        clrb
        lda     ,x+
        cmpa    #$80
        blo     LIST_END
        ldy     #$0125
        bra     LIST_KEYWORD
LIST_UNKNOWN
        puls    b
        ; Native tail continues into the following block.
LIST_END
        leax    -1,x
        rts
LIST_OUT
        jsr     CHROUT
        bra     LIST_BYTE
