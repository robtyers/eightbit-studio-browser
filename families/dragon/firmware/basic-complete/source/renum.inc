; Original native RENUM candidate; staged until the fixed ROM can accommodate it.
; U anchors parameters/new-line state. A dry pass validates all growth before
; any linked programme bytes change; the same rendering pass then applies it.
; Frame: new0,start2,step4,current6,header8,end10,phase12,context13,branch14,go15.
CMD_RENUM
        leas    -16,s
        tfr     s,u
        ldd     #10
        std     ,u
        std     4,u
        clra
        clrb
        std     2,u
        sta     12,u
        lda     #3
        sta     14,u
        leay    ,u
RENUM_PARAMETER
        jsr     EX_SPACE
        cmpa    #',
        beq     RENUM_PARAMETER_NEXT
        jsr     EX_END_CHAR
        beq     RENUM_PARAMETERS_DONE
        jsr     ST_LINE_NUMBER
        std     ,y
RENUM_PARAMETER_NEXT
        jsr     EX_OPTIONAL_COMMA
        bne     RENUM_PARAMETERS_DONE
        dec     14,u
        lbeq    NATIVE_FAIL
        leay    2,y
        bra     RENUM_PARAMETER
RENUM_PARAMETERS_DONE
        jsr     CMD_REQUIRE_END
        ldd     4,u
        lbeq    NATIVE_RANGE_ERROR
        jsr     VALIDATE_PROGRAM
        jsr     ST_CHECK
        ldy     TXTTAB
RENUM_FIND_START
        ldd     ,y
        beq     RENUM_PREFLIGHT
        ldd     2,y
        cmpd    2,u
        bhs     RENUM_PREFLIGHT
        cmpd    ,u
        lbhs    NATIVE_RANGE_ERROR
        ldy     ,y
        bra     RENUM_FIND_START
RENUM_PREFLIGHT
        ldd     ,u
        std     6,u
RENUM_CHECK_NUMBER
        ; FIND_START supplies clear carry for a nonempty first record. Later
        ; iterations retain addition carry until this record's presence is
        ; known: overflow after the final header must not reject a valid run.
        ldx     ,y
        beq     RENUM_CHECK_DONE
        lbcs    NATIVE_RANGE_ERROR
        ldd     6,u
        cmpd    #63999
        lbhi    NATIVE_RANGE_ERROR
        ; The final phase reuses the proven numbering walk for header writes.
        ; Both text passes retain the original headers until this point.
        tst     12,u
        beq     RENUM_CHECK_NEXT
        std     2,y
RENUM_CHECK_NEXT
        tfr     x,y
        addd    4,u
        std     6,u
        bra     RENUM_CHECK_NUMBER
RENUM_CHECK_DONE
        tst     12,u
        lbne    RENUM_DONE
RENUM_BEGIN_PASS
        ldd     VARTAB
        std     10,u
        ldy     TXTTAB
RENUM_LINE
        sty     8,u
        ldd     ,y
        lbeq    RENUM_PASS_DONE
        leax    4,y
        ; Rendering consumes X; the following record and STORE_LINE each own
        ; their next cursor, so no previous text cursor needs preservation.
        ldd     #TOKENBUF
        jsr     LINE_BUFFER_OUTPUT_AT
        clra
        clrb
        sta     13,u
        std     14,u
RENUM_TEXT
        lda     ,x
        lbeq     RENUM_TEXT_END
        ldb     13,u
        jsr     TEXT_CONTEXT
        stb     13,u
        bcs     RENUM_COPY
        cmpa    #32
        beq     RENUM_COPY
        tst     14,u
        beq     RENUM_TOKEN
        cmpa    #',
        beq     RENUM_COPY
        suba    #'0
        cmpa    #9
        bls     RENUM_REFERENCE
        adda    #'0
        clr     14,u
RENUM_TOKEN
        cmpa    #$bf
        beq     RENUM_BRANCH
        cmpa    #$84
        beq     RENUM_BRANCH
        cmpa    #$81
        bne     RENUM_TO_SUB
        inc     15,u
        bra     RENUM_COPY
RENUM_TO_SUB
        tst     15,u
        beq     RENUM_COPY
        clr     15,u
        cmpa    #$bc
        beq     RENUM_BRANCH
        cmpa    #$bd
        bne     RENUM_COPY
RENUM_BRANCH
        inc     14,u
RENUM_COPY
        leax    1,x
        bsr     RENUM_EMIT
        bra     RENUM_TEXT
RENUM_REFERENCE
        jsr     ST_LINE_NUMBER
RENUM_KEEP_TRAILING_SPACE
        lda     -1,x
        cmpa    #32
        bne     RENUM_REFERENCE_READY
        leax    -1,x
        bra     RENUM_KEEP_TRAILING_SPACE
RENUM_REFERENCE_READY
        ldd     LINE_NUMBER
        lbsr    RENUM_TRANSLATE
        bcc     RENUM_PRINT_REFERENCE
        tst     12,u
        beq     RENUM_PRINT_REFERENCE
        ; Diagnostics bypass the temporary line sink, retaining its pointer and
        ; remaining capacity. Only the applying pass reports missing targets.
        pshs    d,x
        clr     FORMAT_BUFFER
        ldx     #RENUM_UL
        jsr     PUTS
        ldd     ,s
        jsr     PRINT_UNSIGNED
        ldx     #ERROR_IN
        jsr     PUTS
        ldy     8,u
        ldd     2,y
        lbsr    RENUM_TRANSLATE
        jsr     PRINT_UNSIGNED
        lda     #13
        jsr     CHROUT
        inc     FORMAT_BUFFER
        puls    d,x

RENUM_PRINT_REFERENCE
        jsr     PRINT_UNSIGNED
        lbra     RENUM_TEXT
RENUM_EMIT
        tst     FORMAT_LENGTH
        lbeq    NATIVE_FAIL
        jmp     CHROUT
RENUM_TEXT_END
        clra
        bsr     RENUM_EMIT
        clr     FORMAT_BUFFER
        ldy     8,u
        ; New length minus old length updates the projected exclusive end.
        ldd     FORMAT_TARGET
        subd    #TOKENBUF-4
        addd    8,u
        subd    ,y
        addd    10,u
        std     10,u
        jsr     NATIVE_STACK_ROOM
        ; A machine-code caller may have S above BASIC's configured limit.
        ; Preflight the same storage ceiling STORE_LINE enforces on applying.
        ldd     STACKTOP
        subd    #128
        cmpd    10,u
        lblo    NATIVE_FAIL
        tst     12,u
        beq     RENUM_NEXT_LINE
        ldd     2,y
        ldy     FORMAT_TARGET
        jsr     STORE_LINE
        jsr     ST_CHECK
        ldy     REC_POS
RENUM_NEXT_LINE
        ldy     ,y
        jmp     RENUM_LINE
RENUM_PASS_DONE
        ; Phase is a predicate: dry text, applying text, then writing headers.
        ; Only the first text pass loops back; headers use the nonzero state.
        com     12,u
        lbne    RENUM_BEGIN_PASS
        inc     12,u
        ldy     TXTTAB
        lbra    RENUM_FIND_START
RENUM_DONE
        jsr     CMD_CLEAR_VARIABLES
        ldb     #16
        jmp     PRESENTATION_EXIT

RENUM_TRANSLATE
        std     LINE_NUMBER
        ldy     TXTTAB
        ldd     ,u
        std     6,u
RENUM_LOOKUP
        ldd     ,y
        beq     RENUM_UNDEFINED
        ldd     2,y
        cmpd    LINE_NUMBER
        beq     RENUM_FOUND
        bhi     RENUM_UNDEFINED
        cmpd    2,u
        blo     RENUM_LOOKUP_NEXT
        ldd     6,u
        addd    4,u
        std     6,u
RENUM_LOOKUP_NEXT
        ldy     ,y
        bra     RENUM_LOOKUP
RENUM_FOUND
        cmpd    2,u
        blo     RENUM_TRANSLATED
        ldd     6,u
RENUM_TRANSLATED
        andcc   #$fe
        rts
RENUM_UNDEFINED
        ldd     LINE_NUMBER
        orcc    #1
        rts
RENUM_UL
        fcc     "UL "
        fcb     0

; Shared bounded token/editor sink. Both callers own the same published input
; buffer and retain independent cursors in their temporary native frames.
LINE_BUFFER_OUTPUT
        ldd     #INPUTBUF
LINE_BUFFER_OUTPUT_AT
        std     FORMAT_TARGET
        lda     #252
        sta     FORMAT_LENGTH
        lda     #1
        sta     FORMAT_BUFFER
        rts
