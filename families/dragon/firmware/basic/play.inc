; Original guest PLAY macros and PAL-frame duration, no host music parser.
CMD_PLAY
        lbsr    PLAY_OK
        ldy     #PLAY_COMMAND
        jmp     MACRO_EVALUATE
PLAY_COMMAND
        clr     $d8
        cmpa    #'X
        bne     PLAY_NOT_NESTED
        jsr     DRAW_NESTED
        lbra    PLAY_OK
PLAY_NOT_NESTED
        ldy     #$e1
        cmpa    #'L
        beq     PLAY_STATE
        leay    1,y
        cmpa    #'T
        beq     PLAY_STATE
        ldy     #$de
        cmpa    #'O
        beq     PLAY_STATE
        leay    1,y
        cmpa    #'V
        lbne    PLAY_NOTE
PLAY_STATE
        ldb     ,y
        cmpy    #$df
        bne     PLAY_STATE_OCTAVE
        subb    #126
        lsrb
        lsrb
PLAY_STATE_OCTAVE
        cmpy    #$de
        bne     PLAY_MODIFIER
        incb
PLAY_MODIFIER
        pshs    y
        jsr     EX_SPACE
        leax    1,x
        cmpa    #'+
        beq     PLAY_PLUS
        cmpa    #'-
        beq     PLAY_MINUS
        cmpa    #'>
        beq     PLAY_DOUBLE
        cmpa    #'<
        beq     PLAY_HALF
        leax    -1,x
        bsr     PLAY_NUMBER
        bra     PLAY_STATE_VALUE
PLAY_PLUS
        incb
        bra     PLAY_STATE_VALUE
PLAY_MINUS
        subb    #1
        bcs     PLAY_ERROR
        bra     PLAY_STATE_VALUE
PLAY_DOUBLE
        lslb
        bcs     PLAY_ERROR
        bra     PLAY_STATE_VALUE
PLAY_HALF
        lsrb
PLAY_STATE_VALUE
        puls    y
        cmpy    #$df
        beq     PLAY_VOLUME
        tstb
        beq     PLAY_ERROR
        cmpy    #$de
        bne     PLAY_STATE_STORE
        cmpb    #5
        bhi     PLAY_ERROR
        decb
PLAY_STATE_STORE
        stb     ,y
        cmpy    #$e1
        bne     PLAY_OK
        clr     $e5
PLAY_DOT
        jsr     EX_SPACE
        cmpa    #'.
        bne     PLAY_OK
        leax    1,x
        inc     $e5
        bra     PLAY_DOT
PLAY_VOLUME
        cmpb    #31
        bhi     PLAY_ERROR
        lslb
        lslb
        addb    #126
        stb     $df
        negb
        addb    #252
        stb     $e0
PLAY_OK
        lda     #1
        sta     $d8
        rts
PLAY_NUMBER
        jsr     MACRO_NUMBER
        bcs     PLAY_ERROR
        rts
PLAY_ERROR
        jmp     NATIVE_RANGE_ERROR
PLAY_NOTE
        cmpa    #'P
        beq     PLAY_PAUSE
        cmpa    #'N
        beq     PLAY_NUMBER_NOTE
        cmpa    #'A
        blo     PLAY_RAW_NUMBER
        cmpa    #'G
        bhi     PLAY_ERROR
        suba    #'A
        ldy     #PLAY_NOTES
        ldb     a,y
        jsr     EX_SPACE
        cmpa    #'-
        beq     PLAY_FLAT
        cmpa    #'+
        beq     PLAY_SHARP
        cmpa    #'#
        bne     PLAY_NOTE_READY
PLAY_SHARP
        incb
        bra     PLAY_ACCIDENTAL
PLAY_FLAT
        decb
PLAY_ACCIDENTAL
        leax    1,x
        bra     PLAY_NOTE_READY
PLAY_RAW_NUMBER
        leax    -1,x
PLAY_NUMBER_NOTE
        bsr     PLAY_NUMBER
PLAY_NOTE_READY
        decb
        cmpb    #11
        bhi     PLAY_ERROR
        pshs    b
        lda     $e5
        ldb     #255
        mul
        lsra
        rorb
        addd    #255
        std     $e3
        lda     $e1
        ldb     $e2
        mul
        std     6,u
        puls    b
        bsr     PLAY_WAVE
        bra     PLAY_FINISH
PLAY_PAUSE
        bsr     PLAY_NUMBER
        tstb
        beq     PLAY_ERROR
        lda     $e2
        mul
        std     6,u
        ldd     #255
        std     $e3
        ldd     $df
        pshs    d
        ldd     #$7e7e
        std     $df
        ldb     #255
        bsr     PLAY_WAVE
        puls    d
        std     $df
PLAY_FINISH
        jmp     PLAY_OK
PLAY_NOTES
        fcb     10,12,1,3,5,6,8
; B=0..11 or 255 for a pause. U remains the bounded macro frame. The rate
; and phase counter are stack locals; the observed public countdown is E3/E4.
PLAY_WAVE
        pshs    x
        stb     -10,u
        lda     #126
        sta     ,u
        sta     2,u
        ldd     $df
        sta     1,u
        stb     3,u
        lda     $de
        sta     -9,u
        ldb     -10,u
        bmi     PLAY_PAUSE_PERIOD
        lslb
        ldx     #PLAY_PERIODS
        ldd     b,x
PLAY_OCTAVE
        tst     -9,u
        beq     PLAY_PERIOD_READY
        lsra
        rorb
        addd    #2
        dec     -9,u
        bra     PLAY_OCTAVE
PLAY_PAUSE_PERIOD
        ldd     #1024
PLAY_PERIOD_READY
        subd    #53
        lsra
        rorb
        lsra
        rorb
        lsra
        rorb
        std     4,u
        ldd     TIMER+1
        std     8,u
        clrb
        stb     -8,u
        jsr     DAC_ENABLE
PLAY_PHASE
        ldb     -8,u
        lda     b,u
        sta     $ff20
        ldy     4,u
PLAY_DELAY
        leay    -1,y
        bne     PLAY_DELAY
        incb
        andb    #3
        stb     -8,u
        ldy     TIMER+1
        cmpy    8,u
        beq     PLAY_COUNT
        sty     8,u
        ldd     $e3
        subd    6,u
        bhs     PLAY_COUNT_STORE
        clra
        clrb
PLAY_COUNT_STORE
        std     $e3
PLAY_COUNT
        ldd     $e3
        bne     PLAY_PHASE
        lda     #126
        sta     $ff20
        jsr     DAC_DISABLE
        puls    x,pc
PLAY_PERIODS
        fdb     3418,3236,3044,2880,2708,2564,2416,2276,2152,2036,1920,1812

