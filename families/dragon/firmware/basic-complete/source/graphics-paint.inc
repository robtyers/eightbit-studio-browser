; Original directional span fill. Pending [left,right,row,direction] records
; live only on the bounded guest stack; overhangs reverse direction.
; The 16-byte frame pairs row/direction at 8,U and left/right at 12,U.
; A record can therefore move as two words without a second representation.
CMD_PAINT
        jsr     GFX_PAIR
        leas    -16,s
        tfr     s,u
        lda     $c4
        sta     12,u
        lda     $c6
        sta     8,u
        ldb     $b2
        stb     ,u
        stb     1,u
PAINT_COLOURS
        jsr     GFX_OPTIONAL_COLOUR
        stb     ,u
        stb     1,u
        jsr     GFX_OPTIONAL_COLOUR
        stb     1,u
PAINT_ARGUMENTS
        jsr     CMD_REQUIRE_END
        stx     4,u
        lda     12,u
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
        jsr     GFX_READ
        cmpb    1,u
        beq     PAINT_DONE
        stu     10,u
        clr     9,u
        lda     $c4
        tfr     a,b
        std     2,u
        lda     $c6
        sta     8,u
        ; Native tail continues into the following block.
PAINT_LEFT
        lda     $c4
        suba    6,u
        bcs     PAINT_START
        sta     $c4
        jsr     GFX_READ
        cmpb    1,u
        bne     PAINT_LEFT
        lda     $c4
        adda    6,u
        sta     $c4
PAINT_START
        lda     $c4
        sta     12,u
        clr     14,u
PAINT_SPAN
        jsr     GFX_READ
        eorb    ,u
        orb     14,u
        stb     14,u
        jsr     GFX_PIXEL
PAINT_SAME
        lda     $c4
        sta     13,u
        adda    6,u
        bcs     PAINT_SPAN_END
        sta     $c4
        jsr     GFX_READ
        cmpb    1,u
        bne     PAINT_SPAN
PAINT_SPAN_END
        tst     14,u
        beq     PAINT_REMAINDER
        ; The first pass has already filled the complete span. Retain the
        ; same final logical cursor and identical pending-span insertion order.
        lda     13,u
        sta     $c4
PAINT_PUSH_SPANS
        ldd     STREND
        addd    #12
        jsr     NATIVE_STACK_ROOM
        tst     9,u
        beq     PAINT_FIRST_SPAN
        ldd     8,u
        suba    9,u
        negb
        tfr     d,y
        lda     12,u
        cmpa    2,u
        bhs     PAINT_FORWARD
        ldb     2,u
        subb    6,u
        pshs    d,y
PAINT_FORWARD
        bsr     PAINT_PUSH_FULL
        cmpb    3,u
        bls     PAINT_REMAINDER
        ldd     8,u
        suba    9,u
        negb
        tfr     d,y
        lda     3,u
        adda    6,u
        ldb     13,u
        pshs    d,y
        ; Native tail continues into the following block.

PAINT_REMAINDER
        lda     13,u
        cmpa    3,u
        bhs     PAINT_SEED
        adda    6,u
        sta     $c4
        bra     PAINT_SEGMENT
PAINT_FIRST_SPAN
        ; The full initial span, rather than point seeds, owns both directions.
        lda     7,u
        sta     9,u
        bsr     PAINT_PUSH_FULL
        neg     9,u
        bsr     PAINT_PUSH_FULL
        ; Native tail continues into the following block.

PAINT_SEED
        cmps    10,u
        bhs     PAINT_DONE
        puls    d
        std     2,u
        sta     $c4
        puls    d
        std     8,u
        sta     $c6
        cmpa    #191
        bhi     PAINT_SEED

PAINT_SEGMENT
        jsr     GFX_READ
        cmpb    1,u
        bne     PAINT_LEFT
        lda     $c4
        adda    6,u
        bcs     PAINT_SEED
        sta     $c4
        cmpa    3,u
        bls     PAINT_SEGMENT
        bra     PAINT_SEED
; Append a full-span record beneath the continuation, without a private stack.
; X is a dead raster scratch register; the BASIC cursor is retained in 4,U.
PAINT_PUSH_FULL
        puls    x
        ldd     8,u
        adda    9,u
        tfr     d,y
        ldd     12,u
        pshs    d,y
        jmp     ,x
PAINT_DONE
        ldd     #80
        std     $bd
PAINT_RETURN
        tfr     u,s
        ldx     4,u
        leas    16,s
        clra
        rts
