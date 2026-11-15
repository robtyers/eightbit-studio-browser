; Original DAC tone generator. Pitch law is a black-box signal observation:
; quarter period = 5*(256-pitch)+23 E cycles. No reference instructions used.
CMD_SOUND
        jsr     GFX_BYTE
        tstb
        lbeq    CMD_UNSUPPORTED
        pshs    b
        jsr     GFX_NEXT_BYTE
        tstb
        lbeq    CMD_UNSUPPORTED
        pshs    b
        jsr     CMD_REQUIRE_END
        puls    a,b
        jmp     SOUND_TONE
; A duration in approximately 1/16 second units, B pitch 1..255.
SOUND_TONE
        pshs    d,x,y,u
        sta     $8a
        stb     $8c
        lda     #5
        mul
        pshs    d
        ldd     #1303
        subd    ,s++
        std     $8d
        ldd     #55852
        ldy     #0
SOUND_QUOTIENT
        subd    $8d
        bcs     SOUND_QUOTA
        leay    1,y
        bra     SOUND_QUOTIENT
SOUND_QUOTA
        tfr     y,d
        addd    #2
        andb    #$fc
        std     $8d
        tfr     d,y
        jsr     DAC_ENABLE
SOUND_PHASE0
        lda     #126
        sta     $ff20
        ldb     $8c
        negb
SOUND_DELAY0
        decb
        bne     SOUND_DELAY0
        leay    -1,y
        beq     SOUND_CHUNK
        nop
        lda     #2
        sta     $ff20
        ldb     $8c
        negb
SOUND_DELAY1
        decb
        bne     SOUND_DELAY1
        leay    -1,y
        beq     SOUND_CHUNK
        nop
        lda     #126
        sta     $ff20
        ldb     $8c
        negb
SOUND_DELAY2
        decb
        bne     SOUND_DELAY2
        leay    -1,y
        beq     SOUND_CHUNK
        nop
        lda     #254
        sta     $ff20
        ldb     $8c
        negb
SOUND_DELAY3
        decb
        bne     SOUND_DELAY3
        leay    -1,y
        nop
        bne     SOUND_PHASE0
SOUND_CHUNK
        dec     $8a
        beq     SOUND_FINISHED
        ldy     $8d
        bra     SOUND_PHASE0
SOUND_FINISHED
        lda     #2
        sta     $ff20
        jsr     DAC_DISABLE
        puls    d,x,y,u,pc

