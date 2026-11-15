; Original streaming ASCII programme input. A partial line occupies only the
; published input buffer; cassette blocks remain in their own legacy buffer.
FILE_ASCII_DATA
        ldx     #$01da
        ldb     $7d
        beq     FILE_ASCII_NEXT_BLOCK
FILE_ASCII_INPUT
        lda     ,x+
        cmpa    #13
        beq     FILE_ASCII_STORE
        cmpa    #10
        beq     FILE_ASCII_NEXT_BYTE
        cmpa    #32
        blo     NATIVE_SHARED12_TAPE_ASCII_IN_INC_0
        ldy     6,u
        cmpy    #INPUTBUF+251
        bhs     NATIVE_SHARED12_TAPE_ASCII_IN_INC_0
        sta     ,y+
        sty     6,u
FILE_ASCII_NEXT_BYTE
        decb
        bne     FILE_ASCII_INPUT
FILE_ASCII_NEXT_BLOCK
        jmp     FILE_SKIP
NATIVE_SHARED12_TAPE_ASCII_IN_INC_0
        jmp     FILE_BAD_BLOCK
FILE_ASCII_STORE
        pshs    b,x
        bsr     FILE_ASCII_LINE
        puls    b,x
        bra     FILE_ASCII_NEXT_BYTE
FILE_ASCII_LINE
        bsr     FILE_ASCII_SCOPE
        bcs     NATIVE_SHARED12_TAPE_ASCII_IN_INC_0
        rts
FILE_ASCII_SCOPE
        pshs    d,y,u
        jsr     NATIVE_INLINE
FILE_ASCII_LINE_BODY
        ldy     6,u
        clr     ,y
        ldx     #INPUTBUF
        stx     6,u
        lda     ,x
        beq     FILE_ASCII_LINE_END
        jsr     STORE_SOURCE_LINE
        jsr     ST_CHECK
FILE_ASCII_LINE_END
        rts

