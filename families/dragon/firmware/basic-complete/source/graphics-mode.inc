; Original graphics workspace and mode management. B7 is an exclusive end,
; established by public interface observations, despite some table captions.
GFX_INIT
        ldd     #$0300
        std     $b2
        clr     $b6
        lda     #$06
        sta     $bc
        std     $ba
        lsla
        std     $b7
        lda     #16
        sta     $b9
GFX_RUNTIME_RESET
        ldd     #128
        std     $c7
        ldb     #96
        std     $c9
        clra
        clrb
        std     $e3
        std     $e5
        std     $e7
        stb     $e9
        ; Observed public PLAY defaults: O3, V15 DAC levels $BA/$42, L4, T2.
        ldd     #$02ba
        std     $de
        ldd     #$4204
        std     $e0
        lda     #2
        sta     $e2
        rts
LOCAL_FAILURE_11
        jmp     NATIVE_RANGE_ERROR
GFX_NEXT_BYTE
        jsr     MULTI_COMMA
GFX_BYTE
        jsr     EX_BODY
        jmp     MULTI_BYTE
; The delimiter probe and terminal guards retain B. Keep an omitted colour
; in B with Z clear; successful conversion returns Z set and the new colour.
GFX_OPTIONAL_COLOUR
        jsr     EX_OPTIONAL_COMMA
        bne     GFX_COLOUR_RETURN
GFX_PARSE_COLOUR
        bsr     GFX_BYTE


GFX_COLOUR
GFX_COLOUR_ZERO
        cmpb    #8
        bhi     LOCAL_FAILURE_11
        lda     $b6
        bita    #1
        beq     GFX_MONO_COLOUR
        decb
        andb    #3
        clra
        rts
GFX_MONO_COLOUR
        andb    #1
        negb
        andb    #3
GFX_COLOUR_END
        clra
GFX_COLOUR_RETURN
        rts
GFX_PARSE_COLOUR_ZERO equ GFX_PARSE_COLOUR

CMD_COLOR
        bsr     GFX_PARSE_COLOUR
        pshs    b
        bsr     GFX_OPTIONAL_COLOUR
        bne     GFX_COLOR_END
        stb     $b3
GFX_COLOR_END
        puls    b
        stb     $b2
        clra
        rts
CMD_PMODE
        leas    -4,s
        ldd     $ba
        std     2,s
        jsr     EX_COMMA
        beq     GFX_MODE_CURRENT
        bsr     GFX_BYTE
        cmpb    #4
        bhi     LOCAL_FAILURE_11
        bra     GFX_MODE_NUMBER
GFX_MODE_CURRENT
        ldb     $b6
GFX_MODE_NUMBER
        stb     ,s
        jsr     EX_OPTIONAL_COMMA
        bne     GFX_MODE_VALIDATE
        bsr     GFX_BYTE
        tstb
        beq     LOCAL_FAILURE_11
        cmpb    #8
        bhi     LOCAL_FAILURE_11
        decb
        jsr     GFX_PAGE_ADDRESS
        std     2,s
GFX_MODE_VALIDATE
        ldb     ,s
        ldu     #GFX_MODES
        lda     b,u
        clrb
        ; Table contains the page count in 256-byte units.
        addd    2,s
        cmpd    TXTTAB
        bhs     LOCAL_FAILURE_11
        std     $b7
        ldd     2,s
        std     $ba
        lda     ,s
        sta     $b6
        ldb     #16
        bita    #1
        bne     GFX_MODE_WIDE
        cmpa    #4
        bne     GFX_MODE_STRIDE
GFX_MODE_WIDE
        ldb     #32
GFX_MODE_STRIDE
        stb     $b9
        leas    4,s
        clra
        rts
GFX_MODES
        fcb     6,12,12,24,24
CMD_PCLS
        ldb     $b3
        jsr     EX_END
        beq     GFX_CLEAR_COLOUR
        jsr     GFX_BYTE
        jsr     GFX_COLOUR_ZERO
GFX_CLEAR_PARSED
GFX_CLEAR_COLOUR
        jsr     CMD_REQUIRE_END
        lda     #$55
        mul
GFX_CLEAR_READY
        ldy     $ba
GFX_CLEAR_BYTE
        cmpy    $b7
        bhs     GFX_CLEAR_END
        stb     ,y+
        bra     GFX_CLEAR_BYTE
GFX_CLEAR_END
        rts
