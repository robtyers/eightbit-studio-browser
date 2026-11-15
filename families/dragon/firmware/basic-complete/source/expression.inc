; Original bounded numeric expression parser. Apache-2.0.
; X is the token cursor; FAC is the result. C reports unsupported/invalid input.
EX_BODY
        pshs    y,u
        bsr     EX_ENTRY
        puls    y,u,pc
EVAL
EX_ONE
        pshs    d,y,u
        jsr     NATIVE_INLINE

EX_ENTRY
        ldb     #1
        ; Native tail continues into the following block.
; Native return addresses own precedence and prefix continuations.
; Pending typed operands alone create seven-byte value/reference records.
EX_PARSE
        ; A precedence call owns its threshold. Registers are saved
        ; once by EX_BODY; pending operands alone allocate collector records.
        pshs    b
        ldd     STREND
        jsr     NATIVE_STACK_ROOM
        jsr     EX_PRIMARY
        lbcs    EX_PARSE_ERROR
EX_OPERATOR
        bsr     EX_SPACE
; The sole caller owns this classifier and establishes later U values.
        ; Only EX_PARSE calls this internal classifier. Its enclosing EX_BODY
        ; preserves U; the operator loop establishes any later U value itself.
        tfr     a,b
        subb    #$c3
        cmpb    #9
        bhi     EX_PREC_END
        ldu     #EX_PRECEDENCES
        ldb     b,u
        bra     EX_PREC_DONE
EVAL_NUMBER
        bsr     EVAL_NATIVE
EX_NUMBER_CHECK
        tst     $06
        lbne    NATIVE_TYPE_ERROR
EX_NUMBER_RETURN
        rts
EX_END
        bsr     EX_SPACE
        ; Native tail continues into the following block.
; A BASIC statement may also end at ELSE without consuming its token.
EX_END_CHAR
        tsta
        beq     EX_END_CHAR_RETURN
        cmpa    #':
        beq     EX_END_CHAR_RETURN
        cmpa    #$84
EX_END_CHAR_RETURN
        rts
EX_NEXT_SPACE
NATIVE_TAIL_12
        leax    1,x
        ; Native tail continues into the following block.
EX_SPACE
        jsr     TEXT_READ
        cmpa    #32
        beq     NATIVE_TAIL_12

EX_SPACE_END
        rts
NEGATE_D
        coma
        comb
        addd    #1
        rts
EX_ALPHA
        bsr     EX_UPCASE
        jmp     IS_UPPER_NAME


; Required delimiters consume their byte only on equality (which also clears C).
EX_UPCASE
        bsr     EX_SPACE
        jmp     UPCASE

EX_COMMA
        bsr     EX_SPACE
        cmpa    #',
        rts
EX_PREC_END
        clrb
EX_PREC_DONE

        cmpb    ,s
        lblo     EX_PARSE_END
        leax    1,x
        cmpb    #4
        bne     EX_HAVE_OPERATOR
        suba    #$ca
        ldu     #EX_REL_MASKS
        lda     a,u
EX_REL_NEXT
        pshs    a
        bsr     EX_SPACE
        cmpa    #$ca
        blo     EX_REL_DONE
        cmpa    #$cc
        bhi     EX_REL_DONE
        leax    1,x
        suba    #$ca
        lda     a,u
        bita    ,s
        bne     EX_PARSE_ERROR
        ora     ,s+
        cmpa    #7
        beq     EX_PARSE_ERROR
        bra     EX_REL_NEXT
EX_REL_DONE
        puls    a
        ldb     #4
EX_HAVE_OPERATOR
        pshs    a,b
        leas    -8,s
        ; Reference0, packed value2, type7, operator8, precedence9.
        ; One capture helper retains numbers or authoritative string references.
        tfr     s,u
        jsr     VALUE_CAPTURE
        sta     7,s

EX_VALUE_SAVED
        ldb     9,s
        incb
EX_RIGHT
        lbsr     EX_PARSE
        lda     $06
        eora    7,s
        lbmi    NATIVE_TYPE_ERROR
        pshs    x
        jsr     F_SWAP
        leau    2,s
        jsr     VALUE_FRAME_LOAD
EX_VALUE_RESTORED
        tst     $06
        beq     EX_VALUE_NUMERIC
        lda     10,s
        cmpa    #$c3
        beq     EX_VALUE_NUMERIC
        ldb     5,s
        jsr     STRING_TEMP_REWIND
EX_VALUE_NUMERIC
        ldd     10,s
        bsr     EX_APPLY
        puls    x
        leas    10,s
        jmp     EX_OPERATOR
EX_PARSE_ERROR
        jmp     NATIVE_FAIL
EX_PARSE_END
        ; Every typed operation consumes its own arguments. A complete value
        ; therefore already owns exactly one public descriptor, or none.
        clra
        puls    b,pc

EX_PRECEDENCES
        fcb     5,5,6,6,8,2,1,4,4,4

EX_REL_MASKS
        fcb     4,2,1
EX_APPLY
        cmpb    #4
        beq     EX_RELATION
        tst     $06
        beq     EX_NUMERIC_APPLY
        cmpa    #$c3
        lbne    NATIVE_TYPE_ERROR
        ldb     7,s
        jmp     STRING_CONCAT
EX_NUMERIC_APPLY
        cmpa    #$c7
        lbeq    EX_POWER
        bhi     EX_BITS
        cmpa    #$c6
        bne     EX_ARITHMETIC
        tst     ARG
        lbeq    NATIVE_ZERO_ERROR
EX_ARITHMETIC
        suba    #$c3
        lsla
        ldy     #FP_OPERATIONS
        jsr     [a,y]
        lbcs    NATIVE_OVERFLOW_ERROR
        rts
EX_BITS
        pshs    a
        jsr     PUBLIC_FAC_WORD
        pshs    d
        jsr     F_SWAP
        jsr     PUBLIC_FAC_WORD
        ; AND/OR tokens differ in bit zero. Shift that saved selector without
        ; disturbing either word operand or needing a second operand copy.
        lsr     2,s
        bcc     EX_BITS_AND
        ora     ,s
        orb     1,s
        bra     EX_BITS_DONE
EX_BITS_AND
        anda    ,s
        andb    1,s
EX_BITS_DONE
        leas    3,s
        jmp     INT_TO_FAC

LOCAL_FAILURE_22
        jmp     NUMBER_PARSE
EX_RELATION
        pshs    a
        tst     $06
        beq     EX_COMPARE_NUMBER
; The enclosing expression owns the cursor and caller registers.
        ldx     STR_PTR
        ldy     ARG+1
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
        bra     EX_COMPARE_RESULT
EX_COMPARE_NUMBER
        jsr     F_COMPARE
EX_COMPARE_RESULT
        nega
        inca
        ldy     #EX_REL_MASKS
        lda     a,y
EX_REL_TEST
        bita    ,s+
        beq     EX_FALSE
        ldd     #-1
        jmp     INT_TO_FAC
EX_FALSE
        clr     $06
        jmp     FAC_ZERO

EX_PRIMARY
        jsr     EX_SPACE
        leax    -1,x
        stx     TXT_PTR
        ; Recreate EX_SPACE flags exactly after publishing the hook cursor.
        cmpa    #32
        jsr     $018b
        ldx     TXT_PTR
        leax    1,x
        jsr     EX_SPACE
        jsr     BYTE_EQUAL_DISPATCH
        fcb     7
        fcb     $22
        fdb     STRING_LITERAL
        fcb     $c3
        fdb     EX_SIGN
        fcb     $c4
        fdb     EX_SIGN
        fcb     $c0
        fdb     EX_NOT
        fcb     $26
        fdb     EX_BASE_NUMBER
        fcb     $28
        fdb     EX_PAREN
        fcb     $2e
        fdb     LOCAL_FAILURE_22
        cmpa    #'0
        blo     EX_PRIMARY_NAME
        cmpa    #'9
        lbls     LOCAL_FAILURE_22
EX_PRIMARY_NAME
        jsr     UPCASE
        cmpa    #$be
        lbeq    EX_FN
        cmpa    #$ff
        beq     EX_FUNCTION
EX_VARIABLE
        jsr     VAR_FIND_NATIVE
        pshs    x
        tfr     y,x
        jsr     VALUE_LOAD
        puls    x,pc
EX_SIGN
        leax    1,x
        pshs    a
        ldb     #7
        jsr     EX_PARSE
        puls    a
        jsr     EX_NUMBER_CHECK
        cmpa    #$c3
        beq     EX_PRIMARY_OK
        tst     FAC
        beq     EX_PRIMARY_OK
        jsr     F_NEGATE
EX_PRIMARY_OK
        rts
EX_NOT
        leax    1,x
        ldb     #3
        jsr     EX_PARSE
        jsr     EX_NUMBER_CHECK
        jsr     PUBLIC_FAC_WORD
        coma
        comb
        jmp     INT_TO_FAC
EX_PAREN
        jsr     EX_OPEN
        ldb     #1
        jsr     EX_PARSE
        jmp     MULTI_CLOSE
EX_ERROR equ NATIVE_FAIL
EX_FUNCTION
        leax    1,x
        jsr     TEXT_READ
        leax    1,x
        suba    #$80
        cmpa    #$22
        bhs     EX_FUNCTION_EXTENSION
        cmpa    #$16
        blo     EX_FUNCTION_SINGLE
        tfr     a,b
        lslb
        ldu     $0128
        tfr     b,a
        jsr     EX_ORIGINAL_ENTRY
        beq     EX_FUNCTION_NATIVE
        ; Native tail continues into the following block.
EX_REBOUND_RAW
        cmpa    #50
        lblo    EX_MULTI_STRING
        bne     EX_REBOUND_UNPARSED
        ; POINT's public entry receives its cursor just after the opening
        ; parenthesis. Unlike the later table entries, A also holds the index.
        pshs    a
        jsr     EX_OPEN
        puls    a
        stx     TXT_PTR
        leax    a,u
        tfr     a,b
        jsr     [,x]
        ldx     TXT_PTR
        jmp     EX_NUMBER_CHECK
EX_FUNCTION_NATIVE
        lsra
        adda    #$80
        jmp     [b,u]

EX_REBOUND_FUNCTION
        pshs    a
        ldb     TEMP_FREE+1
        tst     $06
        beq     EX_REBOUND_ARGUMENT_READY
        jsr     STRING_OWNED
        beq     EX_REBOUND_ARGUMENT_READY
        ldb     TEMP_FREE+1
EX_REBOUND_ARGUMENT_READY
        pshs    b
        tst     $06
        beq     EX_REBOUND_CALL
        ; Expose the canonical public string union at machine-code entry.
        clr     STR_PTR
        clr     STR_PTR+1
EX_REBOUND_CALL
        lda     1,s
        jsr     CALL_REBOUND_TABLE
        puls    b
        leas    1,s
        jsr     STRING_TEMP_REWIND
        jmp     EX_NUMBER_CHECK
EX_REBOUND_UNPARSED
        jsr     CALL_REBOUND_TABLE
        ; Primary-table callbacks return through the numeric continuation,
        ; including replacement entries for the built-in string functions.
        jmp     EX_NUMBER_CHECK


EX_FUNCTION_EXTENSION
        tfr     a,b
        lslb
        jsr     TEXT_READ
        stx     TXT_PTR
        jsr     [$0132]
        ldx     TXT_PTR
EX_EXTERNAL_RESULT
        tst     $06
        beq     EX_EXTERNAL_READY
        ; The published external FAC string result is a descriptor pointer.
        ; Refresh the byte cache from that authoritative public descriptor.
        pshs    x
        ldx     STR_DESC
        jsr     STRING_LOAD
        puls    x
EX_EXTERNAL_READY
        clra
        rts
EX_FUNCTION_SINGLE
        pshs    a
        bsr     EX_PAREN
        puls    a
        lbra    EX_FUNC_VALIDATED
EX_MEM
        ldd     FRETOP
        subd    STREND
        jmp     INT_TO_FAC
EX_TIMER
        ldd     TIMER+1
        ; Unsigned timer values are normalised through the original helper.
        jmp     UNSIGNED_TO_FAC
EX_FUNC_APPLY
        suba    #$80
        cmpa    #$15
        jsr     NATIVE_CHECK_HI
EX_FUNC_VALIDATED
        cmpa    #$0e
        beq     EX_FUNC_NUMERIC
        cmpa    #$0d
        blo     EX_FUNC_NUMERIC
        cmpa    #$11
        bls     EX_FUNC_DISPATCH
EX_FUNC_NUMERIC
        tst     $06
        beq     EX_FUNC_DISPATCH
        cmpa    #4
        lbne    NATIVE_TYPE_ERROR
        ; The legacy RND call advances once even when its argument is a string.
        ; Discard that fraction, then use the ordinary typed-error boundary.
        jsr     FAC_ZERO
        jsr     VALUE_RND
        jmp     NATIVE_TYPE_ERROR
EX_FUNC_DISPATCH
        lsla
        ldu     $0128
        jsr     EX_ORIGINAL_ENTRY
        lbne    EX_REBOUND_FUNCTION
        pshs    x
        jsr     [a,u]
        puls    x
        lbcs    NATIVE_RANGE_ERROR
        rts
EX_ABS
        lda     FSIGN
        anda    #$7f
        jmp     NATIVE_TAIL_5


EX_SGN
        tst     FAC
        lbeq    FAC_ZERO
        ldd     #1
        tst     FSIGN
        bpl     EX_SGN_END
        ldd     #-1
EX_SGN_END
        jmp     INT_TO_FAC
EX_PEEK
        jsr     EX_TO_ADDRESS
        tfr     d,y
        clra
        ldb     ,y
        jmp     INT_TO_FAC
FAC_TO_ADDRESS
        tst     FSIGN
        bmi     FAC_ADDRESS_ERROR
        jmp     FAC_WORD_MAGNITUDE
FAC_ADDRESS_ERROR
        coma
        rts



; Scalar records retain the published two-name-byte/five-value-byte layout.
; Lookup requires the caller's native failure owner. Scalar and array paths
; share one D/U save and return the lvalue in Y.
VAR_FIND_BARE
        ldb     #4
        bra     VAR_FIND_MODE
VAR_FIND
        clrb
VAR_FIND_MODE
        pshs    d,u
VAR_BODY
VAR_PARSE
        pshs    b
        jsr     EX_ALPHA
        lbcc    NATIVE_FAIL
        leax    1,x
        sta     VAR_NAME
        clr     VAR_NAME+1
        ldb     #1
VAR_NAME_TAIL
        ; Spaces are insignificant throughout legacy variable names.
        jsr     EX_UPCASE
        jsr     IS_NAME_BYTE
        bcc     VAR_NAME_DONE
        tstb
        beq     VAR_NAME_SKIP
        sta     VAR_NAME+1
        clrb
VAR_NAME_SKIP
        leax    1,x
        bra     VAR_NAME_TAIL
NATIVE_ISLAND_EXPRESSION_INC_1
        jmp     NATIVE_FAIL
VAR_NAME_DONE
        cmpa    #'$
        bne     VAR_NAME_TYPE
        lda     VAR_NAME+1
        ora     #$80
        sta     VAR_NAME+1
        leax    1,x
VAR_NAME_TYPE
        jsr     EX_SPACE
        ldb     ,s
        bitb    #2
        beq     VAR_ARRAY_TEST
        lda     VAR_NAME
        ora     #$80
        sta     VAR_NAME
        ; Native tail continues into the following block.
VAR_SCALAR
        puls    b
        andb    #1
        bne     NATIVE_ISLAND_EXPRESSION_INC_1
        ldy     VARTAB
        ldd     VAR_NAME
VAR_SEARCH
        cmpy    ARYTAB
        beq     VAR_CREATE
        bhi     NATIVE_ISLAND_EXPRESSION_INC_1
        cmpd    ,y
        beq     VAR_FOUND
        leay    7,y
        bra     VAR_SEARCH
VAR_ARRAY_TEST
        bitb    #4
        bne     VAR_SCALAR
        cmpa    #'(
        bne     VAR_SCALAR
        puls    b
        jmp     ARRAY_FIND
VAR_CREATE
        ldu     STREND
        leau    7,u
        tfr     u,d
        addd    #2
        jsr     NATIVE_STACK_ROOM
        pshs    x
        ldx     STREND
VAR_SHIFT
        cmpx    ARYTAB
        beq     VAR_INITIALISE
        lda     ,-x
        sta     ,-u
        bra     VAR_SHIFT
VAR_INITIALISE
        puls    x
        ldd     VAR_NAME
        std     ,y
        clra
        clrb
        std     2,y
        std     4,y
        sta     6,y
        ldd     STREND
        addd    #7
        std     STREND
        stu     ARYTAB
VAR_FOUND
        leay    2,y
        sty     VAR_PTR
VAR_SET_TYPE
        clr     $06
        tst     VAR_NAME+1
        bpl     VAR_NUMERIC
        dec     $06
VAR_NUMERIC
        puls    d,u,pc
; Full-width integral and positive-base real powers share LOG/EXP rounding.
EX_POWER
        ; Keep the two typed operands once. Every magnitude uses the same
        ; logarithm/exponential rounding path, including integral powers.
        leas    -7,s
        bsr     NATIVE_ISLAND_EXPRESSION_INC_0
        fcb     FP_SWAP,FP_STORE+0,FP_SWAP,FP_END
        clr     6,s
        tst     ,s
        beq     EX_POWER_ONE
        tst     FAC
        beq     EX_POWER_ZERO
        tst     FSIGN
        bpl     EX_POWER_MAGNITUDE
        ; A negative base requires an integral exponent. Inspect every
        ; significand bit. The legacy parity cutoff is magnitude 2^31.
        lda     ARG
        cmpa    #129
        lblo    NATIVE_RANGE_ERROR
        suba    #160
        bhs     EX_POWER_MAGNITUDE
        nega
        tfr     a,b
        beq     EX_POWER_PARITY
EX_POWER_INTEGER
        lsr     AMANT
        ror     AMANT+1
        ror     AMANT+2
        ror     AMANT+3
        lbcs    NATIVE_RANGE_ERROR
        decb
        bne     EX_POWER_INTEGER
EX_POWER_PARITY
        ldb     AMANT+3
        andb    #1
        stb     6,s
EX_POWER_MAGNITUDE
EX_POWER_LOG
        clr     FSIGN
        bsr     NATIVE_ISLAND_EXPRESSION_INC_0
        fcb     FP_LOG,FP_ARG+0,FP_MUL,FP_EXP,FP_END
        lbcs    NATIVE_OVERFLOW_ERROR
        tst     6,s
        beq     EX_POWER_SUCCESS
        jsr     F_NEGATE
EX_POWER_SUCCESS
        leas    7,s
        rts
EX_POWER_ZERO
        tst     5,s
        lbmi    NATIVE_ZERO_ERROR
        bra     EX_POWER_SUCCESS
EX_POWER_ONE
        bsr     NATIVE_ISLAND_EXPRESSION_INC_0
        fcb     FP_KLOAD+FPC_ONE,FP_END
        bra     EX_POWER_SUCCESS
NATIVE_ISLAND_EXPRESSION_INC_0
        jmp     FP_RUN

        ; Shared assignment boundary: numeric writes cannot fail; string writes
        ; retain allocation/collection errors and the caller's token cursor.
VALUE_STORE
        tst     $06
        lbne    STRING_STORE
        pshs    x
        tfr     y,x
        jsr     FAC_STORE
NATIVE_TAIL_1
        puls    x
        clra
        rts



F_NEGATE
        lda     FSIGN
        eora    #$80
        sta     FSIGN
        rts

; A is the doubled ordinal and U is the selected public table. Compare only
; the selected target, preserving the argument registers and token cursor.
EX_ORIGINAL_ENTRY
        pshs    x
        ldx     #FUNCTION_TABLE
        ldx     a,x
        cmpx    a,u
        puls    x,pc

; Optional stack-owned [previous,end] context. Only its exact end reads as
; NUL; nested FN bodies and other independently addressed text stay readable.
; X/B/Y/U are preserved. Ordinary text has no active context.
TEXT_READ
        pshs    b,y
        cmps    STR_ROOTS
        bhi     TEXT_READ_BYTE
        ldy     STR_ROOTS
        cmpx    2,y
        bne     TEXT_READ_BYTE
        clra
        puls    b,y,pc
TEXT_READ_BYTE
        lda     ,x
        puls    b,y,pc
