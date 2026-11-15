; Original bounded guest tokeniser. Alphabet and token numbers are interface facts.
; Input X: NUL-terminated ASCII, maximum 251 bytes; output TOKENBUF with NUL.
; C set: invalid/too-long input. Programme storage is not changed here.
; EDIT's acceptance key remains visible in A at the crunch hook.
TOKENISE_EDIT
        pshs    d,y,u
        jsr     NATIVE_INLINE
        bra     TOKENISE_HOOK
TOKENISE
        stx     TXT_PTR
        leax    -1,x
; Numbered source publishes its line word and passes that number in X. Both
; paths keep the raw source cursor authoritative through the public hook.
TOKENISE_LINE
        pshs    d,y,u
        jsr     NATIVE_INLINE
        lda     [TXT_PTR]
TOKENISE_HOOK
        jsr     $01a3
        ldx     TXT_PTR
        ldy     #TOKENBUF
        ; B owns lexical context until a keyword search starts in normal code.
        ; Descriptor misses and function tokens restore the normal state.
        clrb
TK_NEXT
        lda     ,x
        lbeq    TK_END
        cmpa    #$80
        lbhs    TK_ERROR
        jsr     TEXT_CONTEXT
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
        cmpa    #'?
        bne     TK_SEARCH_WORDS
        leax    1,x
        ldb     #$87
        ; Native tail continues into the following block.
TK_STATEMENT
        cmpb    #$83
        lbeq    TK_APOSTROPHE
        cmpb    #$84
        bne     TK_EMIT_CODE
        lda     #':
        bsr     TK_WRITE
TK_EMIT_CODE
        tfr     b,a
        bsr     TK_WRITE
        clrb
        jsr     TEXT_CONTEXT

TK_NEXT_JUMP
        bra     TK_NEXT
NATIVE_SHARED12_TOKENISE_INC_0
        ; Native tail continues into the following block.
TK_COPY
        lda     ,x+
        bra     NATIVE_TAIL_13
TK_SEARCH_WORDS
        ldu     #$0120
        bsr     TK_DESCRIPTOR
        bcc     TK_STATEMENT
        ldu     #$0125
        bsr     TK_DESCRIPTOR
        bcs     TK_LITERAL
        lda     #$ff
        lbsr     TK_WRITE
        tfr     b,a
        clrb
NATIVE_TAIL_13
        lbsr     TK_WRITE
        bra     TK_NEXT_JUMP
; Each five-byte kind descriptor occupies the same half of a ten-byte
; record. A zero count terminates that kind; cumulative token space bounds it.
; The word comparison is part of the descriptor walk. Its saved X is
; discarded on a match; a miss reloads X before advancing the token ordinal.
TK_DESCRIPTOR
        ldb     #$80
TK_DESCRIPTOR_NEXT
        lda     ,u
        beq     TK_DESCRIPTOR_NONE
        pshs    a,x,u
        ldu     1,u
TK_CANDIDATE
TK_COMPARE_CHAR
        lda     ,x+
        eora    ,u+
        bita    #$7f
        bne     TK_MISMATCH_LAST
        tsta
        bpl     TK_COMPARE_CHAR
        leas    5,s
        clra
        rts
TK_MISMATCH_LAST
        tsta
        bmi     TK_TRY_FAILED
TK_MISMATCH
        lda     ,u+
        bpl     TK_MISMATCH
TK_TRY_FAILED
        ldx     1,s
        incb
        beq     TK_DESCRIPTOR_LIMIT
        dec     ,s
        bne     TK_CANDIDATE
        ldu     3,s
        leau    10,u
        leas    5,s
        bra     TK_DESCRIPTOR_NEXT
TK_DESCRIPTOR_LIMIT
        leas    5,s
TK_DESCRIPTOR_NONE
        coma
        rts
TK_LITERAL
        clrb
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
        cmpb    #5
        beq     TEXT_RESET
        cmpb    #4
        beq     TEXT_LITERAL
        ; Quotes toggle the low context bit in code, strings and DATA alike.
        ; Comments and the second byte of a function token were handled first.
        cmpa    #34
        beq     TEXT_QUOTE
        tstb
        beq     TEXT_CODE
        cmpb    #2
        bne     TEXT_LITERAL
        cmpa    #':
        bne     TEXT_LITERAL
TEXT_RESET
        clrb
TEXT_LITERAL
        orcc    #1
        rts
TEXT_QUOTE
        eorb    #1
        bra     TEXT_LITERAL
TEXT_CODE
        cmpa    #$ff
        beq     TEXT_OPEN_FUNCTION
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

TEXT_NORMAL equ WORD_NO

