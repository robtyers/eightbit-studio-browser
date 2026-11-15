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
        bra     EX_PARSE
EVAL_NUMBER
        bsr     EVAL_NATIVE
EX_NUMBER_CHECK
        tst     $06
        lbne    NATIVE_TYPE_ERROR
EX_NUMBER_RETURN
        rts
EX_END
        bsr     EX_SPACE
        jmp     EX_END_CHAR
EX_NEXT_SPACE
NATIVE_TAIL_12
        leax    1,x
        bra     EX_SPACE
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
EX_SPACE
        lda     ,x
        cmpa    #32
        bne     EX_SPACE_END
        bra     NATIVE_TAIL_12

EX_SPACE_END
        rts
EX_PARSE
        ; A precedence call owns only its threshold. Caller registers are saved
        ; once by EX_BODY; pending operands alone allocate collector records.
        pshs    b
        ldd     STREND
        jsr     NATIVE_STACK_ROOM
        jsr     EX_PRIMARY
        lbcs    EX_PARSE_ERROR
EX_OPERATOR
        bsr     EX_SPACE
        jsr     EX_PRECEDENCE
        cmpb    ,s
        blo     EX_PARSE_END
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
        ; Link0, packed value2, type7, operator8, precedence9. The same packed
        ; string value is a live collector root while the RHS allocates.
        ldd     STR_ROOTS
        std     ,s
        lda     $06
        sta     7,s
        leay    2,s
        ; The pending value borrows a string until its rooted RHS completes.
        ; Numeric assignment ignores ownership and retains the packed ABI.
        lda     #1
        sta     STR_OWNER
        jsr     VALUE_STORE
        tst     $06
        beq     EX_VALUE_SAVED
        sts     STR_ROOTS
EX_VALUE_SAVED
        ldb     9,s
        cmpb    #8
        beq     EX_RIGHT
        incb
EX_RIGHT
        bsr     EX_PARSE
        lda     $06
        cmpa    7,s
        lbne    NATIVE_TYPE_ERROR
        pshs    x
        jsr     F_SWAP
        leax    4,s
        jsr     VALUE_LOAD
EX_VALUE_RESTORED
        ldd     10,s
        bsr     EX_APPLY
        puls    x
        bcs     EX_PARSE_ERROR
        ldd     ,s
        std     STR_ROOTS
        leas    10,s
        jmp     EX_OPERATOR
EX_PARSE_ERROR
        jmp     NATIVE_FAIL
EX_PARSE_END
        clra
        puls    b,pc

EX_PRECEDENCE
        pshs    a,x
        suba    #$c3
        cmpa    #9
        bhi     EX_PREC_END
        ldx     #EX_PRECEDENCES
        ldb     a,x
        puls    a,x,pc
EX_PREC_END
        clrb
        puls    a,x,pc

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
        lbeq    STRING_CONCAT
        jmp     NATIVE_TYPE_ERROR
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
        jsr     FAC_TO_INT
        bcs     EX_PARSE_ERROR
        pshs    d
        jsr     F_SWAP
        jsr     FAC_TO_INT
        bcs     EX_PARSE_ERROR
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
        jsr     STRING_COMPARE
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
        cmpa    #'"
        lbeq    STRING_LITERAL
        cmpa    #$c3
        beq     EX_SIGN
        cmpa    #$c4
        beq     EX_SIGN
        cmpa    #$c0
        beq     EX_NOT
        cmpa    #'&
        lbeq    EX_BASE_NUMBER
        cmpa    #'(
        beq     EX_PAREN
        cmpa    #'.
        beq     LOCAL_FAILURE_22
        cmpa    #'0
        blo     EX_PRIMARY_NAME
        cmpa    #'9
        bls     LOCAL_FAILURE_22
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
        jsr     FAC_TO_INT
        bcs     EX_NOT_ERROR
        coma
        comb
        jmp     INT_TO_FAC
EX_NOT_ERROR
        rts
EX_PAREN
        jsr     EX_OPEN
        ldb     #1
        jsr     EX_PARSE
        jmp     MULTI_CLOSE
EX_ERROR equ NATIVE_FAIL
EX_FUNCTION
        leax    1,x
        lda     ,x+
        suba    #$80
        cmpa    #$21
        jsr     NATIVE_CHECK_HI
        cmpa    #$16
        blo     EX_FUNCTION_SINGLE
        tfr     a,b
        lslb
        ldu     #FUNCTION_TABLE
        adda    #$80
        jmp     [b,u]

EX_FUNCTION_SINGLE
        pshs    a
        bsr     EX_PAREN
        puls    a
        pshs    x
        bsr     EX_FUNC_VALIDATED
        puls    x,pc
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
        ldu     #FUNCTION_TABLE
        jsr     [a,u]
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
VAR_FIND_BARE
        ldb     #4
        bra     VAR_FIND_MODE
VAR_FIND
        clrb
VAR_FIND_MODE
        pshs    d,y,u
        jsr     NATIVE_INLINE
VAR_BODY
        bsr     VAR_PARSE
        sty     10,s
        rts


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
        bra     VAR_SCALAR
VAR_ARRAY_TEST
        bitb    #4
        bne     VAR_SCALAR
        cmpa    #'(
        bne     VAR_SCALAR
        puls    b
        jmp     ARRAY_FIND
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
        rts
; Integer powers use bounded squaring; positive real bases use LOG/EXP.
EX_POWER
        pshs    y,u
        leas    -15,s
        lda     ASIGN
        sta     14,s
        bsr     NATIVE_ISLAND_EXPRESSION_INC_0
        fcb     FP_STORE+0,FP_SWAP,FP_STORE+6,FP_END
        jsr     FAC_TO_INT
        bcs     EX_POWER_REAL
        std     12,s
        jsr     INT_TO_FAC
        bsr     NATIVE_ISLAND_EXPRESSION_INC_0
        fcb     FP_ARG+6,FP_COMPARE,FP_END
        tsta
        bne     EX_POWER_REAL
        ldd     #1
        jsr     INT_TO_FAC
        ldd     12,s
        bpl     EX_POWER_MAGNITUDE
        jsr     NEGATE_D
EX_POWER_MAGNITUDE
        std     12,s
EX_POWER_LOOP
        ldd     12,s
        beq     EX_POWER_END
        lsra
        rorb
        std     12,s
        bcc     EX_POWER_NO_PRODUCT
        bsr     NATIVE_ISLAND_EXPRESSION_INC_0
        fcb     FP_ARG+0,FP_MUL,FP_END
        bcs     EX_POWER_FAIL
EX_POWER_NO_PRODUCT
        ldd     12,s
        beq     EX_POWER_END
        bsr     NATIVE_ISLAND_EXPRESSION_INC_0
        fcb     FP_STORE+6,FP_LOAD+0,FP_ARG+0,FP_MUL
        fcb     FP_STORE+0,FP_LOAD+6,FP_END
        bcs     EX_POWER_FAIL
        bra     EX_POWER_LOOP
NATIVE_ISLAND_EXPRESSION_INC_0
        jmp     FP_RUN
EX_POWER_END
        tst     14,s
        bpl     EX_POWER_SUCCESS
        bsr     NATIVE_ISLAND_EXPRESSION_INC_0
        fcb     FP_SWAP,FP_KLOAD+0,FP_DIV,FP_END
        bcs     EX_POWER_FAIL
EX_POWER_SUCCESS
        leas    15,s
        jmp     NATIVE_TAIL_9

EX_POWER_FAIL
        leas    15,s
        jmp     NATIVE_TAIL_6

EX_POWER_REAL
        bsr     NATIVE_ISLAND_EXPRESSION_INC_0
        fcb     FP_LOAD+0,FP_END
        tst     FAC
        bne     EX_POWER_LOG
        tst     14,s
        bmi     EX_POWER_FAIL
        jsr     FAC_ZERO
        bra     EX_POWER_SUCCESS
EX_POWER_LOG
        bsr     NATIVE_ISLAND_EXPRESSION_INC_0
        fcb     FP_LOG,FP_ARG+6,FP_MUL,FP_EXP,FP_END
        bcs     EX_POWER_FAIL
        bra     EX_POWER_SUCCESS
; A BASIC statement may also end at ELSE without consuming its token.
EX_END_CHAR
        tsta
        beq     EX_END_CHAR_RETURN
        cmpa    #':
        beq     EX_END_CHAR_RETURN
        cmpa    #$84
EX_END_CHAR_RETURN
        rts

        ; Shared assignment boundary: numeric writes cannot fail; string writes
        ; retain allocation/collection errors and the caller's token cursor.
VALUE_STORE
        tst     $06
        lbne    STRING_STORE
        pshs    x
        tfr     y,x
        jsr     FAC_STORE
        jmp     NATIVE_TAIL_1



F_NEGATE
        lda     FSIGN
        eora    #$80
        sta     FSIGN
        rts

