; Original guest statement execution. Apache-2.0.
EXECUTE
        clr     EXEC_ORIGIN
        pshs    y,u
        sts     EXEC_SAVED
        bra     EXEC_START
EXECUTE_DIRECT
        lda     #1
        sta     EXEC_ORIGIN
        lds     STACKTOP
        leas    1,s
        sts     EXEC_SAVED
; The executor owns U between statements. Native boundaries preserve it;
; control-stack/machine-code returns explicitly restore this private base.
EXEC_START
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
        clr     EXEC_CALLED-$03e8,u
        std     CTL_RESET-$03e8,u
        std     STR_ROOTS-$03e8,u
        clr     EXEC_MODE-$03e8,u
        clr     EXEC_JUMP-$03e8,u
        jsr     DIRECT_LINE
EXEC_LOOP
        jsr     BREAK_POLL
        bne     EXEC_TOKEN
        jsr     CMD_END_VALIDATED
        jsr     BREAK_MESSAGE
        bra    EXEC_COMPLETE
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
        ldu     #$03e8
        tst     EXEC_CALLED-$03e8,u
        beq     EXEC_CHECK_ERROR
        clr     EXEC_CALLED-$03e8,u
        ldx     TXT_PTR
EXEC_CHECK_ERROR
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
        lda     EXEC_MODE-$03e8,u
        cmpa    #$ff
        beq     EXEC_COMPLETE
        tst     EXEC_JUMP-$03e8,u
        beq     EXEC_CHECK_SEPARATOR
        clr     EXEC_JUMP-$03e8,u
        bra     EXEC_LOOP
EXEC_CHECK_SEPARATOR
        bsr     NATIVE_SHARED12_COMMANDS_INC_0
        tsta
        beq     EXEC_LINE_END
        cmpa    #':
        beq     EXEC_SEPARATOR
        cmpa    #$84
        bne     EXEC_ERROR
        jsr     CMD_REM
        bra     EXEC_LINE_END
NATIVE_SHARED12_COMMANDS_INC_0
        jmp     EX_SPACE
EXEC_SEPARATOR
        leax    1,x
        bra     EXEC_LOOP
EXEC_LINE_END
        tst     EXEC_MODE-$03e8,u
        beq     EXEC_COMPLETE
        ldx     EXEC_NEXT-$03e8,u
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
        lbne    DIRECT_ERROR
        puls    y,u,pc

EXECUTE_STATEMENT
        pshs    d,y,u
        jsr     NATIVE_INLINE
STATEMENT_FAIL equ NATIVE_FAIL
STATEMENT_DISPATCH
        clr     ERROR_CODE
        bsr     NATIVE_SHARED12_COMMANDS_INC_0
        cmpa    #$ff
        beq     CMD_LET
        cmpa    #$80
        blo     CMD_LET
        cmpa    #$ba
        bhi     NATIVE_ISLAND_COMMANDS_INC_0
        leax    1,x
        suba    #$80
        lsla
        ldu     #STATEMENT_TABLE
        jmp     [a,u]
CMD_UNSUPPORTED equ NATIVE_FAIL
CMD_LET
        bsr     NATIVE_SHARED12_COMMANDS_INC_0
        cmpa    #$ff
        bne     CMD_LET_VARIABLE
        ldb     1,x
        cmpb    #$9e
        beq     CMD_TIMER
        jmp     CMD_MID
CMD_LET_VARIABLE
        jsr     VAR_FIND_NATIVE
        pshs    y
        ldd     ARYTAB
        pshs    d
        lda     $06
        pshs    a
        jsr     EX_EQUALS
        jsr     EVAL_NATIVE
        lda     $06
        cmpa    ,s+
        lbne    NATIVE_TYPE_ERROR
        puls    u
        ; Creating an RHS scalar moves the array table, including an array LHS.
        cmpu    ,s
        puls    y
        bhi     CMD_LET_ADDRESS_READY
        ldd     ARYTAB
        pshs    u
        subd    ,s++
        leay    d,y
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
NATIVE_ISLAND_COMMANDS_INC_0
        jmp     NATIVE_FAIL

CMD_STOP
        jsr     CMD_REQUIRE_END
        jsr     BREAK_MESSAGE
        bra     CMD_END_VALIDATED
CMD_END
        bsr     NATIVE_ISLAND_COMMANDS_INC_1
CMD_END_VALIDATED
        tst     EXEC_MODE
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
        bra    CMD_CLEAR_VARIABLES
CMD_CLEAR
        jsr     EX_END
        beq     CMD_CLEAR_VARIABLES
        jsr     EVAL_ADDRESS
        pshs    d
        jsr     EX_OPTIONAL_COMMA
        bne     CMD_CLEAR_CURRENT_LIMIT
        jsr     EVAL_ADDRESS
        tsta
        bmi     NATIVE_ISLAND_COMMANDS_INC_0
        subd    #1
        bcs     NATIVE_ISLAND_COMMANDS_INC_0
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
        bcs     NATIVE_ISLAND_COMMANDS_INC_0
        pshs    d
        ldd     VARTAB
        addd    #128
        cmpd    ,s++
        jsr     NATIVE_CHECK_HS
        ldd     ,s
        std     MEMTOP
        subd    2,s
        std     STACKTOP
        leas    4,s

CMD_CLEAR_VARIABLES
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
        jsr     GFX_RUNTIME_RESET
        jsr     EX_END
        beq     CMD_RUN_ALL
        suba    #'0
        cmpa    #9
        bls     CMD_RUN_LINE
        ldb     #2
        jsr     FILE_BEGIN
        bra     CMD_RUN_ALL
CMD_RUN_LINE
        jsr     ST_LINE_NUMBER
        bsr     NATIVE_ISLAND_COMMANDS_INC_1
        jsr     CTL_FIND_LINE
        bcs     LOCAL_FAILURE_2
        pshs    y
        bsr     CMD_CLEAR_VARIABLES
        puls    y
        jmp     CTL_GO_TARGET
CMD_RUN_ALL
        jsr     VALIDATE_TEXT
        bcs     LOCAL_FAILURE_2
        bsr     CMD_CLEAR_VARIABLES
        lda     #1
        sta     EXEC_MODE
        jsr     DIRECT_LINE
        ldd     TXTTAB
        std     EXEC_NEXT
        ldx     #EMPTY_TEXT
        rts
LOCAL_FAILURE_2
        jmp     CMD_UNSUPPORTED
CMD_GO
        bsr     NATIVE_ISLAND_COMMANDS_INC_2
        cmpa    #$bd
        lbeq    CMD_GOSUB
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
        jmp     NATIVE_TAIL_3
CMD_IF_STATEMENT
        lda     #1
        sta     EXEC_JUMP
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
        lbcs    NATIVE_RANGE_ERROR
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
CMD_PRINT
        bsr     NATIVE_ISLAND_COMMANDS_INC_2
        cmpa    #'#
        lbeq    PRINT_DEVICE
        cmpa    #'@
        lbeq    PRINT_AT
PRINT_BODY
        jsr     EX_END_CHAR
        beq     CMD_PRINT_NEWLINE
CMD_PRINT_NUMBER
        cmpa    #$bb
        lbeq    PRINT_TAB
        cmpa    #';
        beq     CMD_PRINT_MORE
        cmpa    #',
        beq     CMD_PRINT_ZONE
        jsr     EVAL_NATIVE
        tst     $06
        bne     CMD_PRINT_VALUE_STRING
        jsr     PRINT_NUMBER
        bra     CMD_PRINT_END
CMD_PRINT_VALUE_STRING
        jsr     STRING_PRINT
CMD_PRINT_END
        bsr     NATIVE_ISLAND_COMMANDS_INC_2
        jsr     EX_END_CHAR
        bne     CMD_PRINT_NUMBER
CMD_PRINT_NEWLINE
        lda     #13
        bsr     NATIVE_ISLAND_COMMANDS_INC_3
        clra
        rts
CMD_PRINT_ZONE
        lda     #32
        bsr     NATIVE_ISLAND_COMMANDS_INC_3
        jsr     PRINT_COLUMN
        anda    #15
        bne     CMD_PRINT_ZONE
CMD_PRINT_MORE
        leax    1,x
        jsr     EX_END
        beq     CMD_OK
        bra     CMD_PRINT
DIRECT_LINE
        ldd     #$ffff
        std     CURLINE
CMD_OK
        clra
        rts
CMD_LLIST
        ldu     #CMD_LIST
        ldb     #$fe
        jmp     PRINT_SELECTED

LOCAL_FAILURE_26
        jmp     LIST_OUT
CMD_LIST
        pshs    x,y,u
        leas    -4,s
        tfr     s,u
        jsr     LINE_RANGE
        jsr     ST_CHECK
        stx     4,s
        jsr     VALIDATE_PROGRAM
        jsr     ST_CHECK
        tfr     s,u
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
        lda     #13
        bsr     NATIVE_ISLAND_COMMANDS_INC_3
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
        blo     LOCAL_FAILURE_26
        cmpa    #$cd
        bhi     LOCAL_FAILURE_26
        ldy     #LEGACY_STATEMENT_WORDS
LIST_KEYWORD
        pshs    b
        suba    #$80
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
        cmpa    #$a1
        bhi     LIST_END
        ldy     #LEGACY_FUNCTION_WORDS
        bra     LIST_KEYWORD
LIST_OUT
        jsr     CHROUT
        bra     LIST_BYTE
LIST_END
        leax    -1,x
        rts

