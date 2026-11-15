; Original bounded scanline fill. Two-byte seeds and local state live only on
; the guest stack. A border-free row propagates only if it contains a colour
; change: existing paint is horizontally traversable, but a wholly painted row
; does not bridge a vertical gap. This is observable legacy PAINT behaviour.
CMD_PAINT
        jsr     GFX_PAIR
        leas    -12,s
        tfr     s,u
        lda     $c4
        sta     9,u
        lda     $c6
        sta     8,u
        ldb     $b2
        stb     ,u
        stb     1,u
PAINT_COLOURS
        jsr     EX_OPTIONAL_COMMA
        bne     PAINT_ARGUMENTS
        jsr     GFX_PARSE_COLOUR_ZERO
        stb     ,u
        stb     1,u
        jsr     EX_OPTIONAL_COMMA
        bne     PAINT_ARGUMENTS
        jsr     GFX_PARSE_COLOUR_ZERO
        stb     1,u
PAINT_ARGUMENTS
        jsr     CMD_REQUIRE_END
        stx     4,u
        lda     9,u
        sta     $c4
        lda     8,u
        sta     $c6
        ldb     #1
        stb     6,u
        stb     7,u
        lda     $b6
        cmpa    #4
        beq     PAINT_Y_STEP
        inc     6,u
        lda     $c4
        anda    #$fe
        sta     $c4
PAINT_Y_STEP
        lda     $b6
        cmpa    #2
        bhs     PAINT_READY
        inc     7,u
        lda     $c6
        anda    #$fe
        sta     $c6
PAINT_READY
        lda     ,u
        sta     $b4
        lda     $c4
        ldb     $c6
        pshs    d
        stu     10,u
PAINT_SEED
        cmps    10,u
        bhs    PAINT_DONE
        puls    d
        sta     $c4
        stb     $c6
PAINT_NEW
        bsr     PAINT_TEST
        bcs     PAINT_NEXT
PAINT_LEFT
        lda     $c4
        suba    6,u
        bcs     PAINT_START
        sta     $c4
        bsr     PAINT_TEST
        bcc     PAINT_LEFT
        lda     $c4
        adda    6,u
        sta     $c4
PAINT_START
        lda     $c4
        sta     9,u
PAINT_CHANGE
        jsr     GFX_READ
        cmpb    1,u
        beq     PAINT_NEXT
        cmpb    ,u
        bne     PAINT_BEGIN
        lda     $c4
        adda    6,u
        bcs     PAINT_NEXT
        sta     $c4
        bra     PAINT_CHANGE
PAINT_BEGIN
        lda     9,u
        sta     $c4
        clr     2,u
        clr     3,u
        lda     $c6
        sta     8,u
PAINT_PIXEL
        jsr     GFX_PIXEL
        ldd     STREND
        addd    #4
        jsr     NATIVE_STACK_ROOM
        leax    2,u
        ldb     $c6
        subb    7,u
        bsr     PAINT_EDGE
        bcs     PAINT_BELOW
        lda     $c4
        pshs    d
PAINT_BELOW
        leax    3,u
        ldb     $c6
        addb    7,u
        bsr     PAINT_EDGE
        bcs     PAINT_RIGHT
        lda     $c4
        pshs    d
PAINT_RIGHT
        lda     $c4
        adda    6,u
        bcs     PAINT_NEXT
        sta     $c4
        bsr     PAINT_TEST
        bcc     PAINT_PIXEL
PAINT_NEXT
        bra    PAINT_SEED
PAINT_DONE
        clra

PAINT_RETURN
        tfr     u,s
        ldx     4,u
        leas    12,s
        rts
PAINT_TEST
        jsr     GFX_READ
        subb    1,u
        comb
        ; Completing two's complement carries only for a zero difference.
        addb    #1
        rts
PAINT_EDGE
        pshs    b
        cmpb    #191
        bhi     PAINT_EDGE_BLOCKED
        stb     $c6
        bsr     PAINT_TEST
        bcs     PAINT_EDGE_BLOCKED
        tst     ,x
        bne     PAINT_EDGE_OLD
        inc     ,x
        bra     PAINT_EDGE_RETURN
PAINT_EDGE_BLOCKED
        clr     ,x
PAINT_EDGE_OLD
        coma
PAINT_EDGE_RETURN
        lda     8,u
        sta     $c6
        puls    b,pc

