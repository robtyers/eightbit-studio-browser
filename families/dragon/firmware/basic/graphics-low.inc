; Original 64 by 32 semigraphics interface.
EX_POINT
        jsr     GFX_PAIR
        pshs    x
        bsr     SG_ADDRESS
        bcs     LOCAL_FAILURE_3
        lda     ,y
        bpl     SG_POINT_TEXT
        bitb    ,y
        beq     SG_POINT_OFF
        ldb     #16
        mul
        anda    #7
        inca
        tfr     a,b
        bra     SG_POINT_VALUE
SG_POINT_TEXT
        ldd     #-1
        bra     SG_POINT_RESULT
SG_POINT_OFF
        clrb
SG_POINT_VALUE
        clra
SG_POINT_RESULT
        jmp     NATIVE_TAIL_8

LOCAL_FAILURE_3
        jmp     CMD_UNSUPPORTED
CMD_RESET
        clr     $c2
        bra     SG_PLOT
CMD_SET
        lda     #1
        sta     $c2
SG_PLOT
        jsr     GFX_COORDINATES
        tst     $c2
        beq     SG_RESET_COLOUR
        jsr     GFX_NEXT_BYTE
        tstb
        beq     LOCAL_FAILURE_3
        cmpb    #8
        bhi     LOCAL_FAILURE_3
        decb
        lda     #16
        mul
        stb     $b4
SG_RESET_COLOUR
        jsr     MULTI_CLOSE
        jsr     CMD_REQUIRE_END
        pshs    x
        bsr     SG_ADDRESS
        bcs     LOCAL_FAILURE_3
        lda     ,y
        bmi     SG_HAVE_CELL
        lda     #$80
SG_HAVE_CELL
        tst     $c2
        beq     SG_CLEAR_BIT
        pshs    b
        anda    #$8f
        ora     $b4
        ora     ,s+
        bra     SG_STORE
SG_CLEAR_BIT
        comb
        pshs    b
        anda    ,s+
SG_STORE
        sta     ,y
        clra
        puls    x,pc
SG_ADDRESS
        ldb     $c4
        cmpb    #63
        bhi     SG_ADDRESS_ERROR
        lda     $c6
        cmpa    #31
        bhi     SG_ADDRESS_ERROR
        pshs    a,b
        lsra
        ldb     #32
        mul
        addd    #$0400
        tfr     d,y
        ldb     1,s
        lsrb
        clra
        leay    d,y
        puls    a,b
        lsrb
        ldb     #8
        bcc     SG_COLUMN_READY
        lsrb
SG_COLUMN_READY
        lsra
        bcc     SG_MASK_READY
        lsrb
        lsrb
SG_MASK_READY
        clra
        rts

SG_ADDRESS_ERROR
        coma
        rts

