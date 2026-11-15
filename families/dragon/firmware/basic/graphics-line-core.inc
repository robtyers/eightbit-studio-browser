; U is eight bytes into the native frame. Relative to its start: x,y,x2,y2,
; dx,negative dy,sx,sy,error,twice error (words). This bias keeps every
; line/box field in signed five-bit reach without changing public coordinates.
GFX_LINE
        coma
        bra     LINE_SAVE_TARGET
GFX_PHYSICAL_LINE
        clra
LINE_SAVE_TARGET
        ; Rasterise in physical pixels. Mapping each logical step separately
        ; adds extra pixels when an odd endpoint is halved in a lower mode.
        ldd     -4,u
        ldy     -2,u
        pshs    d,y
        bcc     LINE_PHYSICAL
        lda     $b6
        cmpa    #4
        bhs     LINE_PHYSICAL
        asr     -8,u
        ror     -7,u
        asr     -4,u
        ror     -3,u
        cmpa    #2
        bhs     LINE_PHYSICAL
        asr     -6,u
        ror     -5,u
        asr     -2,u
        ror     -1,u
LINE_PHYSICAL
        ldy     #1
        ldd     -4,u
        subd    -8,u
        bpl     LINE_DX
        jsr     NEGATE_D
        leay    -2,y
LINE_DX
        std     0,u
        sty     4,u
        ldy     #1
        ldd     -6,u
        subd    -2,u
        bmi     LINE_DY
        jsr     NEGATE_D
        leay    -2,y
LINE_DY
        sty     6,u
        std     2,u
        ldd     0,u
        addd    2,u
        std     8,u
LINE_PIXEL
        lda     $b6
        cmpa    #4
        ldd     -8,u
        bhs     LINE_LOGICAL_X
        lslb
        rola
LINE_LOGICAL_X
        std     $c3
        lda     $b6
        cmpa    #2
        ldd     -6,u
        bhs     LINE_LOGICAL_Y
        lslb
        rola
LINE_LOGICAL_Y
        std     $c5
        bsr     GFX_CLIPPED_PIXEL
        ldd     -8,u
        cmpd    -4,u
        bne     LINE_STEP
        ldd     -6,u
        cmpd    -2,u
        beq     LINE_FINISHED
LINE_STEP
        ldd     8,u
        lslb
        rola
        std     10,u
        cmpd    2,u
        blt     LINE_STEP_Y
        bne     LINE_STEP_X
        tst     6,u
        bmi     LINE_STEP_Y
LINE_STEP_X
        ldd     8,u
        addd    2,u
        std     8,u
        ldd     -8,u
        addd    4,u
        std     -8,u
LINE_STEP_Y
        ldd     10,u
        cmpd    0,u
        bgt     LINE_PIXEL
        bne     LINE_ADVANCE_Y
        tst     4,u
        bmi     LINE_PIXEL
LINE_ADVANCE_Y
        ldd     8,u
        addd    0,u
        std     8,u
        ldd     -6,u
        addd    6,u
        std     -6,u
        bra     LINE_PIXEL
LINE_FINISHED
        ; Callers retain the original logical target, including its low bits.
        puls    d,y
        std     -4,u
        std     -8,u
        sty     -2,u
        sty     -6,u
        rts
; Curve chords may cross the canvas edge. Reject signed/outside coordinates
; before the public address helper; no graphics or programme byte can escape.
GFX_CLIPPED_PIXEL
        ldd     $c3
        tsta
        bne     GFX_CLIPPED_END
        ldd     $c5
        cmpd    #191
        bls     GFX_PIXEL
GFX_CLIPPED_END
        rts

