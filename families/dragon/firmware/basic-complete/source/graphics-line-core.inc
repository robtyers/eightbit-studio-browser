; U is eight bytes into the native frame. Relative to its start: x,y,x2,y2,
; dx,dy,sx,sy,minor-axis pointer,error (words). This bias keeps every
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
        pshs    d,x,y
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
        ; Axis-aligned legacy raster calls retain their starting position in
        ; the public physical cursor even though the complete span is drawn.
        ldd     -8,u
        ldy     -6,u
        pshs    d,y
        leax    -8,u
        bsr     LINE_AXIS
        leax    2,x
        bsr     LINE_AXIS
        leay    -8,u
        ldd     0,u
        cmpd    2,u
        blo     LINE_MAJOR_READY
        exg     x,y
LINE_MAJOR_READY
        sty     8,u
        ldd     8,x
        std     10,u
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
        ldd     ,x
        cmpd    4,x
        beq     LINE_FINISHED
        addd    12,x
        std     ,x
        ldy     8,u
        ldd     10,u
        subd    8,y
        subd    8,y
        std     10,u
        bmi     LINE_MINOR
        bne     LINE_PIXEL
        tst     12,x
        bmi     LINE_PIXEL
LINE_MINOR
        ldd     ,y
        addd    12,y
        std     ,y
        ldd     10,u
        addd    8,x
        addd    8,x
        std     10,u
        bra     LINE_PIXEL
LINE_FINISHED
        ldd     0,u
        beq     LINE_AXIS_CURSOR
        ldd     2,u
        bne     LINE_FINAL_CURSOR
LINE_AXIS_CURSOR
        ldd     ,s
        std     $bd
        ldd     2,s
        std     $bf
LINE_FINAL_CURSOR
        leas    4,s
        ; Callers retain the original logical target, including its low bits.
        puls    d,x,y
        std     -4,u
        std     -8,u
        sty     -2,u
        sty     -6,u
        rts
; X selects an axis word. The paired target, magnitude and signed unit
; step are at fixed displacements, shared by X and Y setup.
LINE_AXIS
        ldy     #1
        ldd     4,x
        subd    ,x
        bpl     LINE_AXIS_POSITIVE
        jsr     NEGATE_D
        leay    -2,y
LINE_AXIS_POSITIVE
        std     8,x
        sty     12,x
        rts
; Curve chords may cross the canvas edge. Reject signed/outside coordinates
; before the public address helper; no graphics or programme byte can escape.
GFX_CLIPPED_PIXEL
        ldd     $c3
        tsta
        bne     GFX_CLIPPED_END
        ldd     $c5
        cmpd    #191
        lbls     GFX_PIXEL
GFX_CLIPPED_END
        rts
