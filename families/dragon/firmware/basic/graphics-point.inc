; Original plotting in BASIC's 256 by 192 coordinate space.
GFX_PAIR
        bsr     GFX_COORDINATES
        jmp     MULTI_CLOSE
GFX_COORDINATES
        jsr     EX_OPEN
        ; Public coordinate conversion clips an unsigned word X and byte Y
        ; to the logical canvas. Negative/overflowing numeric inputs fail.
        jsr     EVAL_ADDRESS
        tsta
        beq     GFX_X_CLAMPED
        ldb     #255
GFX_X_CLAMPED
        pshs    b
        jsr     GFX_NEXT_BYTE
        cmpb    #191
        bls     GFX_Y_CLAMPED
        ldb     #191
GFX_Y_CLAMPED
        stb     $c6
        clr     $c5
        puls    b
        stb     $c4
        clr     $c3
        rts



CMD_PRESET
        clr     $c2
        bra     GFX_PLOT
CMD_PSET
        lda     #1
        sta     $c2
GFX_PLOT
        bsr     GFX_COORDINATES
        ldb     $b2
        tst     $c2
        bne     GFX_PLOT_COLOUR
        ldb     $b3
GFX_PLOT_COLOUR
        pshs    b
        jsr     EX_OPTIONAL_COMMA
        bne     GFX_PLOT_READY
        jsr     GFX_PARSE_COLOUR
        stb     ,s
GFX_PLOT_READY
        jsr     MULTI_CLOSE
        jsr     CMD_REQUIRE_END
        puls    b
        stb     $b4
GFX_PIXEL
        pshs    x
        bsr     GFX_ADDRESS
        lda     $86
        coma
        anda    ,y
        ora     $b5
        sta     ,y
NATIVE_TAIL_1
        puls    x
        clra
        rts
EX_PPOINT
        bsr     GFX_PAIR
        pshs    x
        bsr     GFX_READ
        lda     $b6
        bita    #1
        bne     GFX_POINT_PLUS
        tstb
        beq     GFX_POINT_NUMBER
        clrb
GFX_POINT_PLUS
        incb
GFX_POINT_NUMBER
        clra
NATIVE_TAIL_8
        jsr     INT_TO_FAC
        puls    x,pc
; Internal colour: zero/three in monochrome, zero through three in colour.
GFX_READ
        bsr     GFX_ADDRESS
        ldb     ,y
        andb    $86
        lda     $b6
        bita    #1
        bne     GFX_POINT_COLOUR
        tstb
        beq     GFX_READ_DONE
        ldb     #3
GFX_READ_DONE
        rts
GFX_POINT_COLOUR
        lda     $86
GFX_POINT_SHIFT
        bita    #1
        bne     GFX_READ_DONE
        lsra
        lsrb
        bra     GFX_POINT_SHIFT
; Y byte address; $86 mask and $B5 packed colour, preserving guest registers.
GFX_ADDRESS
        pshs    d,x
        ldb     $c4
        lda     $b6
        cmpa    #4
        beq     GFX_X_PHYSICAL
        lsrb
GFX_X_PHYSICAL
        clr     $bd
        stb     $be
        ldb     $c6
        cmpa    #2
        bhs     GFX_Y_PHYSICAL
        lsrb
GFX_Y_PHYSICAL
        clr     $bf
        stb     $c0
        lda     $b9
        mul
        addd    $ba
        tfr     d,y
        ldb     $be
        lda     $b6
        bita    #1
        bne     GFX_ADDRESS_COLOUR
        andb    #7
        ldx     #GFX_BIT_MASKS
        lda     b,x
        sta     $86
        bra     GFX_PACK_PIXEL
GFX_ADDRESS_COLOUR
        andb    #3
        ldx     #GFX_COLOUR_MASKS
        lda     b,x
        sta     $86
GFX_PACK_PIXEL
        lda     #$55
        ldb     $b4
        mul
        andb    $86
        stb     $b5
        ldb     $be
        lda     $b6
        bita    #1
        bne     GFX_ADDRESS_BYTE
        lsrb
GFX_ADDRESS_BYTE
        lsrb
        lsrb
        clra
        leay    d,y
        puls    d,x,pc
GFX_BIT_MASKS
        fcb     $80,$40,$20,$10,8,4,2,1
GFX_COLOUR_MASKS
        fcb     $c0,$30,$0c,3

