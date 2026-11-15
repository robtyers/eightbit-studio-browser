; Explicit cassette motor/audio selection through the physical PIA pins.
CMD_MOTOR
        bsr     DEVICE_SWITCH
        tsta
        lbeq    MOTOROFF
        jmp     MOTORON
CMD_AUDIO
        bsr     DEVICE_SWITCH
        pshs    a
        ldb     #1
        jsr     SELECT_MUX
        lda     $ff23
        anda    #$f7
        tst     ,s+
        beq     AUDIO_SWITCH
        ora     #$38
AUDIO_SWITCH
        sta     $ff23
        clrb
        rts
DEVICE_SWITCH
        bsr     EX_SPACE
        cmpa    #$88
        beq     DEVICE_ON
        cmpa    #$c2
        bsr     NATIVE_CHECK_NE
        leax    1,x
        jsr     CMD_REQUIRE_END
        clra
        rts
DEVICE_ON
        leax    1,x
        jsr     CMD_REQUIRE_END
        lda     #1
DEVICE_RETURN
        rts

; SOUND and PLAY share selection; their timed waveform loops remain separate.
DAC_ENABLE
        clrb
        jsr     SELECT_MUX
        lda     $ff23
        ora     #$38
        bra     AUDIO_SWITCH

DAC_DISABLE
        lda     $ff23
        anda    #$f7
        bra     AUDIO_SWITCH

