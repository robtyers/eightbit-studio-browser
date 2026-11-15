; Original bounded guest tokeniser. Alphabet and token numbers are interface facts.
; Input X: NUL-terminated ASCII, maximum 251 bytes; output TOKENBUF with NUL.
; C set: invalid/too-long input. Programme storage is not changed here.
TOKENISE
        pshs    d,y,u
        jsr     NATIVE_INLINE
        jsr     $01a3
        ldy     #TOKENBUF
        clr     TK_STATE
TK_NEXT
        lda     ,x
        lbeq    TK_END
        cmpa    #$80
        lbhs    TK_ERROR
        ldb     TK_STATE
        jsr     TEXT_CONTEXT
        stb     TK_STATE
        bcs     NATIVE_SHARED12_TOKENISE_INC_0
TK_FAST_LITERAL
        cmpa    #33
        blo     NATIVE_SHARED12_TOKENISE_INC_0
        cmpa    #'0
        blo     TK_SEARCH
        cmpa    #'9
        bls     NATIVE_SHARED12_TOKENISE_INC_0
TK_SEARCH
TK_SEARCH_ALL
        lda     ,x
        cmpa    #'?
        bne     TK_SEARCH_WORDS
        leax    1,x
        ldb     #$87
        bra     TK_EMIT
NATIVE_SHARED12_TOKENISE_INC_0
        bra     TK_COPY
TK_SEARCH_WORDS
        ldu     #LEGACY_STATEMENT_WORDS
        ldb     #$80
TK_CANDIDATE
        bsr     TK_TRY_WORD
        bcc     TK_EMIT
        incb
        ; The public word lists are contiguous. B is their ordinal until the
        ; matched function tail is converted into its two-byte token.
        cmpb    #$f0
        blo     TK_CANDIDATE
        bra     TK_LITERAL
TK_EMIT
        cmpb    #$ce
        blo     TK_STATEMENT
        lda     #$ff
        bsr     TK_WRITE
        subb    #$4e
        tfr     b,a
NATIVE_TAIL_13
        bsr     TK_WRITE
        bra     TK_NEXT_JUMP
TK_STATEMENT
        cmpb    #$83
        beq    TK_APOSTROPHE
        cmpb    #$84
        bne     TK_EMIT_CODE
        lda     #':
        bsr     TK_WRITE
TK_EMIT_CODE
        tfr     b,a
        bsr     TK_WRITE
        clrb
        jsr     TEXT_CONTEXT
        stb     TK_STATE

TK_NEXT_JUMP
        bra     TK_NEXT
; U advances to the next word even on mismatch; X remains the input cursor.
TK_TRY_WORD
        pshs    x

TK_COMPARE_CHAR
        lda     ,x+
        eora    ,u+
        bita    #$7f
        bne     TK_MISMATCH_LAST
        tsta
        bpl     TK_COMPARE_CHAR
TK_MATCHED
        clra
        leas    2,s
        rts
TK_MISMATCH_LAST
        tsta
        bmi     TK_TRY_FAILED
TK_MISMATCH
        lda     ,u+
        bpl     TK_MISMATCH
TK_TRY_FAILED
        coma
        puls    x,pc
TK_LITERAL
        lda     ,x
        bsr     IS_UPPER_NAME
        bcc     TK_COPY
; A failed keyword starts one complete identifier. Consume its following
; uppercase letters/digits without repeating the keyword search.
TK_NAME
        lda     ,x+
        bsr     TK_WRITE
        lda     ,x
        bsr     IS_NAME_BYTE
        bcs     TK_NAME
        jmp     TK_NEXT
TK_COPY
        lda     ,x+
        bra     NATIVE_TAIL_13

TK_ERROR
        jmp     NATIVE_FAIL
TK_END
        clra
        bsr     TK_WRITE
TK_RETURN
        sty     10,s
        clra
        rts
TK_WRITE
        cmpy    #TOKENBUF+252
        bhs     TK_ERROR
        sta     ,y+
        rts
UPCASE
        cmpa    #'a
        blo     UPCASE_END
        cmpa    #'z
        bhi     UPCASE_END
        suba    #32
UPCASE_END
        rts
IS_NAME_BYTE
        cmpa    #'0
        blo     IS_UPPER_NAME
        cmpa    #'9
        bls     WORD_YES
IS_UPPER_NAME
        cmpa    #'A
        blo     WORD_NO
        cmpa    #'Z
        bls     WORD_YES
WORD_NO
        andcc   #$fe
        rts
WORD_YES
        orcc    #1
        rts

; Apostrophe expands by one byte. A one-byte lookahead keeps unread input
; intact even when source and output share INPUTBUF and no keyword shrank.
TK_APOSTROPHE
        lda     #':
        bsr     TK_WRITE
        ldb     ,x+
        lda     #$83
        bsr     TK_WRITE
TK_COMMENT_COPY
        tfr     b,a
        tsta
        beq     TK_COMMENT_END
        cmpa    #$80
        bhs     TK_ERROR
        ldb     ,x+
        bsr     TK_WRITE
        bra     TK_COMMENT_COPY
TK_COMMENT_END
        bsr     TK_WRITE
        bra     TK_RETURN

; A is a source byte, B the literal context: 0 code, 1 quote, 2 DATA,
; 3 quoted DATA, 4 comment, 5 function-code byte. C marks a literal byte; B is the next context.
; Neither cursor nor the original byte is changed, including DATA's colon.
TEXT_CONTEXT
        tstb
        beq     TEXT_CODE
        cmpb    #5
        beq     TEXT_FUNCTION_BYTE
        cmpb    #4
        beq     TEXT_LITERAL
        cmpa    #34
        beq     TEXT_QUOTE
        cmpb    #2
        bne     TEXT_LITERAL
        cmpa    #':
        bne     TEXT_LITERAL
        clrb
TEXT_LITERAL
        orcc    #1
        rts
TEXT_QUOTE
        eorb    #1
        bra     TEXT_LITERAL
TEXT_FUNCTION_BYTE
        clrb
        bra     TEXT_LITERAL
TEXT_CODE
        cmpa    #$ff
        beq     TEXT_OPEN_FUNCTION
        cmpa    #34
        beq     TEXT_OPEN_QUOTE
        ; The same token transitions serve listing, DATA search and editing.
        ; The introducer itself remains code so callers can interpret/output it.
        cmpa    #$82
        beq     TEXT_OPEN_COMMENT
        cmpa    #$83
        beq     TEXT_OPEN_COMMENT
        cmpa    #$86
        bne     TEXT_NORMAL
        ldb     #2
        bra     TEXT_NORMAL
TEXT_OPEN_FUNCTION
        ldb     #5
        bra     TEXT_NORMAL
TEXT_OPEN_COMMENT
        ldb     #4
        bra     TEXT_NORMAL
TEXT_OPEN_QUOTE
        incb
        bra     TEXT_LITERAL

TEXT_NORMAL equ WORD_NO

; Make a bounded NUL-terminated stack copy of the FAC string. U anchors the
; caller's saved return word, X is the copy start and Y its terminator. Return
; through that word without reclaiming the copy; the caller restores S from U.
; Collision fails before copying and retains the same anchor for unwinding.
STACK_STRING
        tfr     s,u
        clra
        ldb     FAC
        coma
        comb
        leas    d,s
        tfr     s,x
        cmpx    STREND
        bls     STACK_STRING_ERROR
        ldy     STR_PTR
        exg     x,y
        ldb     FAC
        beq     STACK_STRING_END
STACK_STRING_COPY
        jsr     COPY_BYTES
STACK_STRING_END
        clr     ,y
        tfr     s,x
STACK_STRING_RETURN
        jmp     [,u]
STACK_STRING_ERROR
        coma
        bra     STACK_STRING_RETURN

