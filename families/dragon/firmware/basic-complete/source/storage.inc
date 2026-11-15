; Original linked-programme storage. Apache-2.0.
; Input: token buffer, Y at its end (including NUL), D line number.
; Validate the old chain and all limits before changing programme bytes.
STORE_LINE
        std     LINE_NUMBER
        std     $02da
STORE_TOKENISED
        ldd     $02da
        cmpd    #63999
        bhi     PARSE_FAILURE_18
        tfr     y,d
        subd    #TOKENBUF
        std     REC_TOKENLEN
        cmpd    #1
        blo     PARSE_FAILURE_18
        cmpd    #252
        bhi     PARSE_FAILURE_18
        addd    #4
        std     REC_NEWLEN
        tst     TOKENBUF
        bne     STORE_VALIDATE
        clr     REC_NEWLEN
        clr     REC_NEWLEN+1
STORE_VALIDATE
        ldx     #TOKENBUF
        ldb     REC_TOKENLEN+1
STORE_CHECK_TOKEN
        lda     ,x+
        beq     STORE_CHECK_LAST
        decb
        bne     STORE_CHECK_TOKEN
PARSE_FAILURE_18
        coma
        rts
STORE_CHECK_LAST
        decb
        bne     PARSE_FAILURE_18
        jsr     VALIDATE_PROGRAM
        bcs     PARSE_FAILURE_18
        jsr     PROGRAM_FIND
        tfr     y,x
        bne     STORE_POSITION
STORE_REPLACE
        ldy     ,y
STORE_POSITION
        stx     REC_POS
        sty     REC_OLDSTOP
        ; The public crunch hook owns the lookup workspace and stored header
        ; independently. Reject an inconsistent resulting order before moving
        ; any programme data, including a duplicate introduced by a callback.
        tst     TOKENBUF
        beq     STORE_ORDER_VALID
        ldd     $02da
        ldx     TXTTAB
STORE_ORDER_NEXT
        cmpx    REC_POS
        beq     STORE_ORDER_HIGH
        cmpd    2,x
        bls     STORE_ORDER_BAD
        ldx     ,x
        bra     STORE_ORDER_NEXT
STORE_ORDER_HIGH
        ldx     REC_OLDSTOP
        ldy     ,x
        beq     STORE_ORDER_VALID
        cmpd    2,x
        bhs     STORE_ORDER_BAD
STORE_ORDER_VALID
        ldy     REC_OLDSTOP
        tfr     y,d
        subd    REC_POS
        std     REC_OLDLEN

STORE_LENGTH
        ldd     REC_NEWLEN
        subd    REC_OLDLEN
        std     REC_DELTA
        addd    VARTAB
        std     REC_END
        cmpd    TXTTAB
        blo    STORE_ERROR
        ldd     STACKTOP
        subd    #128
        cmpd    REC_END
        blo    STORE_ERROR
        ldd     REC_DELTA
        beq     STORE_WRITE
        bmi     STORE_MOVE_DOWN
        ldx     VARTAB
        ldy     REC_END
STORE_MOVE_UP
        cmpx    REC_OLDSTOP
        beq     STORE_WRITE
        lda     ,-x
        sta     ,-y
        bra     STORE_MOVE_UP
STORE_MOVE_DOWN
        ldx     REC_OLDSTOP
        ldd     REC_POS
        addd    REC_NEWLEN
        tfr     d,y
STORE_MOVE_LOW
        jsr     COPY_PROGRAM_TAIL

STORE_WRITE
        ldd     REC_NEWLEN
        beq     STORE_REINDEX
        ldx     REC_POS
        ldd     #1
        std     ,x
        ldd     $02da
        std     2,x
        leay    4,x
        ldx     #TOKENBUF
        ldb     REC_TOKENLEN+1
        jsr     COPY_BYTES
STORE_REINDEX
        ldd     REC_END
        jsr     VARIABLE_ENDS
        subd    #2
        std     REC_LIMIT
        ldx     TXTTAB
        ldb     #1
        bra     PROGRAM_WALK
STORE_ERROR
STORE_ORDER_BAD
        coma
        rts

VALIDATE_TEXT
        pshs    x
        bsr     VALIDATE_PROGRAM
        puls    x,pc
VALIDATE_PROGRAM
        ldx     TXTTAB
        cmpx    #$0600
        blo    STORE_ERROR
        cmpx    VARTAB
        bhs    STORE_ERROR
        ldd     VARTAB
        cmpd    STACKTOP
        bhs    STORE_ERROR
        subd    #2
        std     REC_LIMIT
        cmpx    REC_LIMIT
        bhi     STORE_ERROR
        clrb
; Shared sequential walker. B=0 checks links without writing; B=1 repairs them.
; REC_LIMIT is the terminal word, never an input link used as a read address.
PROGRAM_WALK
        pshs    b
VALIDATE_LINE
        ldd     ,x
        beq     VALIDATE_END
        leay    5,x
        cmpy    REC_LIMIT
        bhi     PROGRAM_BAD
        ldd     2,x
        cmpx    TXTTAB
        beq     VALIDATE_FIRST_LINE
        cmpd    MEX
        bls     PROGRAM_BAD
VALIDATE_FIRST_LINE
        cmpd    #63999
        bhi     PROGRAM_BAD
        std     MEX
        leay    4,x
VALIDATE_BODY
        cmpy    REC_LIMIT
        bhs     PROGRAM_BAD
        lda     ,y+
        bne     VALIDATE_BODY
        tst     ,s
        bne     PROGRAM_REPAIR
        ; No callback runs while scanning a record; its checked source link
        ; is still live at X, so validation needs no second saved link.
        cmpy    ,x
        bne     PROGRAM_BAD
        bra     PROGRAM_NEXT
PROGRAM_REPAIR
        sty     ,x
PROGRAM_NEXT
        tfr     y,x
        bra     VALIDATE_LINE
VALIDATE_END
        cmpx    REC_LIMIT
        bne     PROGRAM_BAD
        bra     PROGRAM_RETURN
PROGRAM_BAD
        coma
PROGRAM_RETURN
        puls    b,pc
LOCAL_FAILURE_17
        bra     STORE_ERROR
PARSE_LINE_NUMBER
        jsr     DECIMAL_DIGIT
        bhi     LOCAL_FAILURE_17
PARSE_LINE_NUMBER_OR_ZERO
        clr     LINE_NUMBER
        clr     LINE_NUMBER+1
LINE_NUMBER_NEXT
        jsr     DECIMAL_DIGIT
        bhi     LINE_NUMBER_END
        sta     TEMP
        ldd     LINE_NUMBER
        cmpd    #6399
        bhi     LOCAL_FAILURE_17
        lslb
        rola
        std     MEX
        lslb
        rola
        lslb
        rola
        addd    MEX
        addb    TEMP
        adca    #0
        std     LINE_NUMBER
        leax    1,x
        bra     LINE_NUMBER_NEXT
LINE_NUMBER_END
        ldd     LINE_NUMBER
        rts

; Both validated relocation paths include the two-byte terminal word.
COPY_PROGRAM_TAIL
        lda     ,x+
        sta     ,y+
        cmpx    VARTAB
        blo     COPY_PROGRAM_TAIL
        rts

; Programme replacement invalidates both variable tables as one pointer update.
VARIABLE_ENDS
        std     VARTAB
        std     ARYTAB
        std     STREND
        rts

; Direct input, cassette source and the editor share the same checked ingestion.
; TOKENISE preserves LINE_NUMBER; each stage returns carry before any later
; stage can overwrite its diagnostic or modify programme storage.
STORE_SOURCE_LINE
        bsr     PARSE_LINE_NUMBER
        bcs     SOURCE_LINE_END
        jsr     EX_SPACE
        stx     TXT_PTR
        ldx     LINE_NUMBER
        stx     $02da
        jsr     TOKENISE_LINE
        bcs     SOURCE_LINE_END
        ; Keep the callback's independent lookup and stored line words live.
        jmp     STORE_TOKENISED
SOURCE_LINE_END
        rts

; Caller validates the chain. Preserve X/U/query; EOF cannot compare equal.
PROGRAM_FIND
        ldd     LINE_NUMBER
PROGRAM_LOWER_BOUND
        pshs    d
        ldy     TXTTAB
PROGRAM_FIND_NEXT
        ldd     ,y
        beq     PROGRAM_FIND_EOF
        ldd     2,y
        cmpd    ,s
        bhs     PROGRAM_FIND_DONE
        ldy     ,y
        bra     PROGRAM_FIND_NEXT
PROGRAM_FIND_EOF
        coma
PROGRAM_FIND_DONE
        leas    2,s
        rts
