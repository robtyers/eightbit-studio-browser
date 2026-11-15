; Original linked-programme storage. Apache-2.0.
; Input: token buffer, Y at its end (including NUL), D line number.
; Validate the old chain and all limits before changing programme bytes.
STORE_LINE
        std     LINE_NUMBER
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
        cmpb    #1
        beq     STORE_CHECK_LAST
        lda     ,x+
        beq     PARSE_FAILURE_18
        decb
        bra     STORE_CHECK_TOKEN
PARSE_FAILURE_18
        coma
        rts
STORE_CHECK_LAST
        tst     ,x
        bne     PARSE_FAILURE_18
        jsr     VALIDATE_PROGRAM
        bcs     PARSE_FAILURE_18
        ldx     TXTTAB
STORE_FIND
        ldd     ,x
        beq     STORE_NO_OLD
        ldd     2,x
        cmpd    LINE_NUMBER
        bhi     STORE_NO_OLD
        beq     STORE_REPLACE
        ldx     ,x
        bra     STORE_FIND
STORE_REPLACE
        ldy     ,x
        bra     STORE_POSITION
STORE_NO_OLD
        tfr     x,y
STORE_POSITION
        stx     REC_POS
        sty     REC_OLDSTOP
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
        ldd     LINE_NUMBER
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
        pshs    b,u
VALIDATE_LINE
        ldd     ,x
        beq     VALIDATE_END
        tfr     d,u
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
        pshs    u
        cmpy    ,s++
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
        puls    b,u,pc
LOCAL_FAILURE_17
        bra     STORE_ERROR
PARSE_LINE_NUMBER
        jsr     DECIMAL_DIGIT
        bhi     LOCAL_FAILURE_17
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
        jsr     PARSE_LINE_NUMBER
        bcs     SOURCE_LINE_END
        jsr     EX_SPACE
        jsr     TOKENISE
        bcs     SOURCE_LINE_END
        ldd     LINE_NUMBER
        jmp     STORE_LINE
SOURCE_LINE_END
        rts

