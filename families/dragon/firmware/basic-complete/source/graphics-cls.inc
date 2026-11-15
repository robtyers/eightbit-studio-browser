; Original colour clearing of the text/semigraphics screen.
CMD_CLS_COLOUR
        jsr     EX_END
        lbeq    CMD_CLS
        jsr     GFX_BYTE
        cmpb    #8
        jsr     NATIVE_CHECK_HI
        ; The end guard preserves B; derive the fill byte only afterwards.
        jsr     CMD_REQUIRE_END
        lda     #$80
        tstb
        beq     SG_CLS_FILL
        decb
        lda     #16
        mul
        orb     #$8f
        tfr     b,a
SG_CLS_FILL
        jmp     CLS_COLOUR





