; Original native GET/PUT. Apache-2.0. Rectangle transfers use guest RAM only.
; U=S+8, 24 transient stack bytes: rectangle -8..-1; operation0, packed1,
; token cursor2, array4/end6, screen row8, screen mask10, rows11,
; array mask12, bit width13, byte column14, byte width15. Width zero means 256.
; Bounds are checked per transfer to preserve observed bounded partial writes.
CMD_GET
        clrb
        bra     SPRITE_BEGIN
CMD_PUT
        ldb     #1
SPRITE_BEGIN
        leas    -24,s
        leau    8,s
        stb     0,u
        jsr     GFX_RECTANGLE
        jsr     VAR_FIND_BARE
        stx     2,u
        ldy     ARYTAB
SPRITE_FIND
        cmpy    STREND
        lbhs    NATIVE_RANGE_ERROR
        jsr     ARRAY_RECORD
        lbcs    NATIVE_RANGE_ERROR
        std     4,u
        ldd     VAR_NAME
        cmpd    ,y
        beq     SPRITE_FOUND
        tfr     x,y
        bra     SPRITE_FIND
SPRITE_FOUND
        tst     1,y
        lbmi    NATIVE_RANGE_ERROR
        stx     6,u
        ldd     4,u
        leax    d,y
        stx     4,u
        ldx     2,u
        clr     1,u
        jsr     EX_OPTIONAL_COMMA
        bne     SPRITE_REQUIRE_END
        tst     0,u
        bne     SPRITE_OPERATION
        lda     #'G
        jsr     EX_EXPECT
        bra     SPRITE_PACKED
SPRITE_OPERATION
        jsr     EX_SPACE
        leay    SPRITE_OPERATIONS,pcr
SPRITE_OPERATION_NEXT
        cmpa    ,y++
        beq     SPRITE_OPERATION_FOUND
        cmpy    #SPRITE_OPERATIONS_END
        blo     SPRITE_OPERATION_NEXT
        jmp     NATIVE_FAIL
SPRITE_OPERATION_FOUND
        lda     -1,y
        sta     0,u
        leax    1,x
SPRITE_PACKED
        inc     1,u
        bra     SPRITE_ARGUMENTS
SPRITE_REQUIRE_END
        jsr     CMD_REQUIRE_END
SPRITE_ARGUMENTS
        stx     2,u
        leax    -8,u
        lbsr    SPRITE_ORDER
        leax    -6,u
        lbsr    SPRITE_ORDER
        ; GET's unpacked byte form publishes the aligned logical start;
        ; packed GET and every PUT retain the selected rectangle start.
        ldd     -8,u
        tst     0,u
        bne     SPRITE_CURSOR
        tst     1,u
        bne     SPRITE_CURSOR
        andb    #$f8
SPRITE_CURSOR
        std     $c3
        ldd     -6,u
        std     $c5
        jsr     GFX_ADDRESS
        ; The address/mask helper already describes the selected first byte.
        ; Retain its byte address while deriving the transfer's bit count.
        sty     8,u
        lda     $86
        ldb     $b6
        bitb    #1
        beq     SPRITE_SURFACE_MASK
        anda    #$aa
SPRITE_SURFACE_MASK
        sta     10,u
        lda     #$80
        sta     12,u
        lda     $b6
        cmpa    #4
        beq     SPRITE_X_BITS
        bita    #1
        bne     SPRITE_X_BITS
        lsr     -7,u
        lsr     -3,u
SPRITE_X_BITS
        cmpa    #2
        bhs     SPRITE_Y_ROWS
        lsr     -5,u
        lsr     -1,u
SPRITE_Y_ROWS
        ldb     -1,u
        subb    -5,u
        incb
        stb     11,u
        ldb     -3,u
        subb    -7,u
        incb
        stb     13,u
        ldb     -7,u
        lsrb
        lsrb
        lsrb
        stb     14,u
        ldb     -3,u
        lsrb
        lsrb
        lsrb
        subb    14,u
        incb
        stb     15,u
        tst     1,u
        beq     SPRITE_ADDRESS
        tst     0,u
        bne     SPRITE_ADDRESS
        ldx     4,u
        clra
        clrb
        std     -2,x
SPRITE_ADDRESS
        ldx     4,u

SPRITE_ROW
        ldy     8,u
        tst     1,u
        bne     SPRITE_BITS
        ldb     15,u
SPRITE_BYTES_COPY
        cmpx    6,u
        bhs     SPRITE_CAPACITY_ERROR
        tst     0,u
        bne     SPRITE_BYTES_PUT
        lda     ,y+
        sta     ,x+
        bra     SPRITE_BYTES_NEXT
SPRITE_BYTES_PUT
        lda     ,x+
        sta     ,y+
SPRITE_BYTES_NEXT
        decb
        bne     SPRITE_BYTES_COPY

SPRITE_ROW_CHECK
        cmpx    6,u
        bhs     SPRITE_CAPACITY_ERROR
SPRITE_ROW_DONE
        clra
        ldb     $b9
        addd    8,u
        std     8,u
        dec     11,u
        bne     SPRITE_ROW
        ldx     2,u
        leas    24,s
        clra
        rts
SPRITE_BITS
        ldb     13,u
        pshs    b
        ldb     10,u
SPRITE_BIT
        cmpx    6,u
        bhs     SPRITE_CAPACITY_ERROR
        tst     0,u
        bne     SPRITE_PUT_BIT
        tst     12,u
        bpl     SPRITE_BIT_EXISTING
        clr     ,x
SPRITE_BIT_EXISTING
        bitb    ,y
        beq     SPRITE_BIT_ADVANCE
        lda     12,u
        ora     ,x
        sta     ,x
        bra     SPRITE_BIT_ADVANCE
SPRITE_PUT_BIT
        lda     ,x
        anda    12,u
        nega
        lda     0,u
        bcc     SPRITE_PUT_DESTINATION
        lsra
        lsra
SPRITE_PUT_DESTINATION
        bitb    ,y
        beq     SPRITE_PUT_TRUTH
        lsra
SPRITE_PUT_TRUTH
        lsra
        tfr     b,a
        bcc     SPRITE_PUT_CLEAR
        ora     ,y
        bra     SPRITE_PUT_STORE
SPRITE_PUT_CLEAR
        coma
        anda    ,y
SPRITE_PUT_STORE
        sta     ,y
SPRITE_BIT_ADVANCE
        lsr     12,u
        bne     SPRITE_BIT_DESTINATION
        ror     12,u
        leax    1,x
SPRITE_BIT_DESTINATION
        lsrb
        bne     SPRITE_BIT_SOURCE
        rorb
        leay    1,y
SPRITE_BIT_SOURCE
        dec     ,s
        bne     SPRITE_BIT
        leas    1,s
        bra     SPRITE_ROW_CHECK

SPRITE_CAPACITY_ERROR
        jmp     NATIVE_RANGE_ERROR
SPRITE_ORDER
        lda     1,x
        cmpa    5,x
        bls     SPRITE_ORDER_END
        ldb     5,x
        sta     5,x
        stb     1,x
SPRITE_ORDER_END
        rts

SPRITE_OPERATIONS
        fcb     $ac,12,$ad,3,$c8,8,$c9,14,$c0,5
SPRITE_OPERATIONS_END
