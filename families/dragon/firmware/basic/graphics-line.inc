; Original integer Bresenham line, bounded to BASIC's 256 by 192 canvas.
CMD_LINE
        jsr     EX_SPACE
        cmpa    #$89
        lbeq    CMD_LINE_INPUT
        leas    -24,s
        leau    8,s
        cmpa    #$c4
        beq     LINE_FROM_CURSOR
        bsr     GFX_RECTANGLE
        bra     LINE_ARGUMENTS
NATIVE_SHARED12_GRAPHICS_LINE_INC_0
        jmp     LOCAL_FAILURE_5
LINE_FROM_CURSOR
        ldd     $c7
        std     -8,u
        ldd     $c9
LINE_FIRST_Y
        std     -6,u
        bsr     GFX_RECTANGLE_TAIL
LINE_ARGUMENTS
        jsr     EX_SPACE
        cmpa    #$ac
        beq     LINE_FOREGROUND
        cmpa    #$ad
        bne     NATIVE_SHARED12_GRAPHICS_LINE_INC_0
        ldb     $b3
        bra     LINE_COLOUR
LINE_FOREGROUND
        ldb     $b2
LINE_COLOUR
        stb     $b4
        leax    1,x
        jsr     EX_COMMA
        bne     LINE_NO_BOX
        jsr     EX_NEXT_SPACE
        cmpa    #'B
        bne     NATIVE_SHARED12_GRAPHICS_LINE_INC_0
        jsr     EX_NEXT_SPACE
        cmpa    #'F
        beq     LINE_FILLED_BOX
        jsr     CMD_REQUIRE_END
        jsr     GFX_BOX
        bra     LINE_CURSOR
LINE_FILLED_BOX
        leax    1,x
        jsr     CMD_REQUIRE_END
        jsr     GFX_BOX_FILL
        bra     LINE_CURSOR
LINE_NO_BOX
        jsr     CMD_REQUIRE_END
        jsr     GFX_LINE
LINE_CURSOR
        ldd     -4,u
        std     $c7
        ldd     -2,u
        std     $c9
        leas    24,s
        rts

; Shared strict second endpoint and following comma for LINE, GET and PUT.
GFX_RECTANGLE
        jsr     GFX_PAIR
        ldd     $c3
        std     -8,u
        ldd     $c5
        std     -6,u
GFX_RECTANGLE_TAIL
        lda     #$c4
        jsr     EX_EXPECT
        jsr     GFX_PAIR
        ldd     $c3
        std     -4,u
        ldd     $c5
        std     -2,u
        jmp     MULTI_COMMA

