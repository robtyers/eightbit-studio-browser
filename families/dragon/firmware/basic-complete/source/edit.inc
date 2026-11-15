; Original bounded native EDIT candidate. No changes reach the programme until
; Enter tokenises and stores the complete line. U owns four transient bytes.
CMD_EDIT
        jsr     ST_LINE_NUMBER
        jsr     CMD_REQUIRE_END
        jsr     CTL_FIND_LINE
        jsr     ST_CHECK
        leas    -4,s
        tfr     s,u
        sty     2,u
        ldd     2,y
        std     ,u
EDIT_RELOAD
        ldx     2,u
        leax    4,x
        jsr     LINE_BUFFER_OUTPUT
        jsr     DETOKENISE_LINE
        clr     FORMAT_BUFFER
        tst     FORMAT_LENGTH
        lbeq    NATIVE_FAIL
        ldy     FORMAT_TARGET
        clr     ,y
        ldb     #252
        subb    FORMAT_LENGTH
        stb     LINELEN
        ldx     #INPUTBUF
EDIT_LIST
        jsr     PUTS
EDIT_RESTART
        lda     #13
        jsr     SCREENOUT
        ldd     ,u
        jsr     PRINT_UNSIGNED
        lda     #32
        jsr     SCREENOUT
        ldx     #INPUTBUF
EDIT_NEXT
        clrb
EDIT_COUNT
        bsr     EDIT_KEY
        suba    #'0
        cmpa    #9
        bhi     EDIT_DISPATCH
        pshs    a
        lda     #10
        mul
        addb    ,s+
        bra     EDIT_COUNT
EDIT_DISPATCH
        adda    #'0
        tstb
        bne     EDIT_HAVE_COUNT
        incb
EDIT_HAVE_COUNT
        jsr     BYTE_EQUAL_DISPATCH
        fcb     13
        fcb     $0d
        fdb     EDIT_STORE
        fcb     $51
        fdb     EDIT_EXIT
        fcb     $41
        fdb     EDIT_RELOAD
        fcb     $4c
        fdb     EDIT_LIST
        fcb     $08
        fdb     EDIT_BACK
        fcb     $20
        fdb     EDIT_FORWARD
        fcb     $44
        fdb     EDIT_DELETE_COUNT
        fcb     $43
        fdb     EDIT_CHANGE
        fcb     $49
        fdb     EDIT_INSERT
        fcb     $58
        fdb     EDIT_EXTEND
        fcb     $48
        fdb     EDIT_HACK
        fcb     $53
        fdb     EDIT_SEARCH
        fcb     $4b
        fdb     EDIT_KILL
        bra     EDIT_NEXT
EDIT_KEY
        jmp     READ_CHARACTER
EDIT_BACK
        cmpx    #INPUTBUF
        beq     EDIT_NEXT
        leax    -1,x
        lda     #8
        jsr     SCREENOUT
        decb
        bne     EDIT_BACK
        bra     EDIT_NEXT
EDIT_FORWARD
        bsr     EDIT_ADVANCE
        bra     EDIT_NEXT
EDIT_ADVANCE
        lda     ,x
        beq     EDIT_ADVANCED
        leax    1,x
        jsr     SCREENOUT
        decb
        bne     EDIT_ADVANCE
EDIT_ADVANCED
        rts
EDIT_EXTEND
        ; The complete input span is at most 251 bytes.
        ldb     #255
        bsr     EDIT_ADVANCE
        bra     EDIT_INSERT
EDIT_HACK
        tfr     x,y
        clr     ,y
        tfr     y,d
        subd    #INPUTBUF
        stb     LINELEN
EDIT_INSERT
        bsr     EDIT_KEY
        jsr     BYTE_EQUAL_DISPATCH
        fcb     3
        fcb     $0d
        fdb     EDIT_STORE
        fcb     $08
        fdb     EDIT_INSERT_BACK
        fcb     $15
        fdb     EDIT_NEXT
        cmpa    #32
        blo     EDIT_INSERT
        ldb     LINELEN
        cmpb    #251
        beq     EDIT_INSERT
        pshs    a,x,y
EDIT_SHIFT_RIGHT
        lda     ,y
        sta     1,y
        cmpy    1,s
        beq     EDIT_INSERT_READY
        leay    -1,y
        bra     EDIT_SHIFT_RIGHT
EDIT_INSERT_READY
        puls    a,x,y
        leay    1,y
        sta     ,x+
        inc     LINELEN
        jsr     SCREENOUT
        bra     EDIT_INSERT
EDIT_INSERT_BACK
        cmpx    #INPUTBUF
        beq     EDIT_INSERT
        leax    -1,x
        bsr     EDIT_DELETE_ONE
        lda     #8
        jsr     SCREENOUT
        bra     EDIT_INSERT
; X/Y remain the cursor and exclusive text end. Shift includes the NUL.
EDIT_DELETE_ONE
        pshs    x
        ; Every caller owns a bounded INPUTBUF cursor and deletion retains it.
        ; The NUL test alone establishes whether a byte may be shifted.
        tst     ,x
        beq     EDIT_DELETE_END
EDIT_SHIFT_LEFT
        lda     1,x
        sta     ,x+
        bne     EDIT_SHIFT_LEFT
        leay    -1,y
        dec     LINELEN
EDIT_DELETE_END
        puls    x,pc
EDIT_CHANGE
        jsr     EDIT_KEY
        cmpa    #13
        beq     EDIT_STORE
        cmpa    #32
        blo     EDIT_CHANGE
        tst     ,x
        lbeq    EDIT_NEXT
        sta     ,x+
        jsr     SCREENOUT
        decb
        bne     EDIT_CHANGE
        lbra    EDIT_NEXT
EDIT_SEARCH
EDIT_KILL
        ; The matched command byte itself distinguishes search from kill.
        ; Keep it beside the original cursor, rather than a second frame flag.
        pshs    a,x
        jsr     EDIT_KEY
        ; Keep the search byte in A; scanning need not overwrite it.
EDIT_FIND_NEXT
        tst     ,x
        beq     EDIT_FOUND
        leax    1,x
        cmpa    ,x
        bne     EDIT_FIND_NEXT
        decb
        bne     EDIT_FIND_NEXT
EDIT_FOUND
        puls    a
        cmpa    #'S
        beq     EDIT_SEARCH_DONE
        tfr     x,d
        subd    ,s
        puls    x
        tstb
        lbeq    EDIT_NEXT
        ; Native tail continues into the following block.
EDIT_DELETE_COUNT
        bsr     EDIT_DELETE_ONE
        decb
        bne     EDIT_DELETE_COUNT
        lbra    EDIT_NEXT
EDIT_SEARCH_DONE
        leas    2,s
        lbra    EDIT_NEXT
EDIT_STORE
        ldx     #INPUTBUF
        stx     TXT_PTR
        ldx     ,u
        stx     LINE_NUMBER
        stx     $02da
        jsr     TOKENISE_EDIT
        jsr     ST_CHECK
        jsr     STORE_TOKENISED
        jsr     ST_CHECK
        jsr     CMD_CLEAR_VARIABLES
EDIT_EXIT
        ldb     #4
; EDIT and RENUM discard only their owned transient frame at successful exit.
; B is the frame extent; the enclosing statement restores the caller's D/U.
PRESENTATION_EXIT
        lda     #$ff
        sta     EXEC_MODE
        ldx     #EMPTY_TEXT
        leas    b,s
        clra
        rts
