; Original bounded array storage. Apache-2.0.
; Layout facts verified by opt-in synthetic black-box probes, 15 October 2026.
; U owns a local frame; nested subscript expressions cannot overwrite its state.
ARRAY_FIND
        pshs    u
        leas    -16,s
        tfr     s,u
        stu     12,u
        stb     2,u
        clr     3,u
        ldd     VAR_NAME
        std     ,u
ARRAY_ARGUMENT
        leax    1,x
        jsr     EX_ADDRESS
        tsta
        bmi     NATIVE_ISLAND_ARRAYS_INC_0
        tst     2,u
        beq     ARRAY_SUBSCRIPT
        addd    #1
ARRAY_SUBSCRIPT
        pshs    d
        inc     3,u
        lda     3,u
        cmpa    #125
        bhi     NATIVE_ISLAND_ARRAYS_INC_1
        ldd     STREND
        jsr     NATIVE_STACK_ROOM
        jsr     EX_COMMA
        beq     ARRAY_ARGUMENT
        jsr     MULTI_CLOSE
        stx     14,u
        ldy     ARYTAB
ARRAY_SEARCH
        cmpy    STREND
        beq     ARRAY_CREATE
        bhi     NATIVE_ISLAND_ARRAYS_INC_1
        jsr     ARRAY_RECORD
        bcs     NATIVE_ISLAND_ARRAYS_INC_1
        ldd     ,u
        cmpd    ,y
        beq     ARRAY_EXISTING
        tfr     x,y
        bra     ARRAY_SEARCH
NATIVE_ISLAND_ARRAYS_INC_0
PARSE_FAILURE_1
NATIVE_ISLAND_ARRAYS_INC_1
ARRAY_VALIDATE_BAD
        jmp     NATIVE_FAIL
ARRAY_EXISTING
        tst     2,u
        bne     PARSE_FAILURE_1
        lda     3,u
        cmpa    4,y
        bne     PARSE_FAILURE_1
        bra     ARRAY_ELEMENT
ARRAY_CREATE
        sty     4,u
        clra
        ldb     3,u
        lslb
        rola
        addd    #5
        std     6,u
        tfr     s,x
        ldd     #5
ARRAY_CREATE_EXTENT
        pshs    x
        tst     2,u
        bne     ARRAY_EXPLICIT_EXTENT
        ldx     ,x
        cmpx    #11
        bhs     ARRAY_VALIDATE_BAD
        ldx     #11
        bra     ARRAY_SIZE_PRODUCT
ARRAY_EXPLICIT_EXTENT
        ldx     ,x
ARRAY_SIZE_PRODUCT
        jsr     ARRAY_MULTIPLY
        puls    x
        bcs     PARSE_FAILURE_1
        leax    2,x
        cmpx    12,u
        blo     ARRAY_CREATE_EXTENT
        addd    6,u
        bcs     PARSE_FAILURE_1
        std     10,u
        addd    4,u
        bcs     NATIVE_ISLAND_ARRAYS_INC_1
        std     8,u
        jsr     NATIVE_STACK_ROOM
        ldy     4,u
ARRAY_ZERO
        clr     ,y+
        cmpy    8,u
        blo     ARRAY_ZERO
        sty     STREND
        ldy     4,u
        ldd     ,u
        std     ,y
        ldd     10,u
        std     2,y
        lda     3,u
        sta     4,y
        leay    5,y
        tfr     s,x
ARRAY_WRITE_EXTENT
        ldd     ,x++
        tst     2,u
        bne     ARRAY_WRITE_DIMENSION
        ldd     #11
ARRAY_WRITE_DIMENSION
        std     ,y++
        cmpx    12,u
        blo     ARRAY_WRITE_EXTENT
        tst     2,u
        bne     ARRAY_DIMENSION_DONE
        ldy     4,u
ARRAY_ELEMENT
        leay    5,y
        tfr     s,x
        clra
        clrb
ARRAY_OFFSET
        ; The validated full extent product bounds every mixed-radix prefix.
        ; Per-subscript comparisons remain essential, including a G marker.
        pshs    x
        ldx     ,y++
        bsr     ARRAY_MULTIPLY
        puls    x
        pshs    d
        ldd     ,x++
        cmpd    -2,y
        lbhs    NATIVE_FAIL
        addd    ,s++
        cmpx    12,u
        blo     ARRAY_OFFSET
        ; The validated full extent product bounds every mixed-radix prefix.
        ; Per-subscript comparisons remain essential, including a G marker.
        ldx     #5
        bsr     ARRAY_MULTIPLY
        leay    d,y
ARRAY_DIMENSION_DONE
        ldd     ,u
        std     VAR_NAME
        sty     VAR_PTR
        ldx     14,u
        tfr     u,s
        leas    16,s
        puls    u
        jmp     VAR_SET_TYPE



; Checked unsigned product shares the complete native product with graphics.
; The high word alone determines overflow; preserve the caller's Y/X/U.
ARRAY_MULTIPLY
        ; Unit extents are common in maximal-dimensional validated arrays.
        cmpx    #1
        beq     ARRAY_MULTIPLY_UNIT
        pshs    y
        jsr     PRODUCT_16
        cmpy    #0
        beq     ARRAY_MULTIPLY_DONE
        orcc    #1
ARRAY_MULTIPLY_DONE
        puls    y,pc
ARRAY_MULTIPLY_UNIT
        rts



CMD_DIM
        ldb     #1
        jsr     VAR_FIND_MODE
        jsr     ST_CHECK
        jsr     EX_OPTIONAL_COMMA
        bne     ARRAY_DIM_DONE
        bra     CMD_DIM
ARRAY_DIM_DONE
        clra
        rts

; One validator serves lookup and collection. Y is the record, X returns
; its exclusive end, and D the header length. No caller workspace is borrowed;
; every extent and the complete five-byte element product must fit the record.
ARRAY_RECORD
        pshs    y,u
        ldd     STREND
        subd    ,s
        cmpd    #7
        blo     ARRAY_RECORD_BAD
        ; The dimension-derived header check below also proves length >= 7.
        ldd     2,y
        leau    d,y
        cmpu    STREND
        bhi     ARRAY_RECORD_BAD
        cmpu    ,s
        bls     ARRAY_RECORD_BAD
        lda     4,y
        beq     ARRAY_RECORD_BAD
        cmpa    #125
        bhi     ARRAY_RECORD_BAD
        ldb     #2
        mul
        addd    #5
        cmpd    2,y
        bhi     ARRAY_RECORD_BAD
        leax    5,y
        leay    d,y
        pshs    y
        ; Start with the element width, so the checked product includes it.
        ; U holds the record end; only the header end needs a stack slot.
        ldd     #5
ARRAY_RECORD_EXTENT
        pshs    x
        ldx     ,x
        beq     ARRAY_RECORD_GRAPHICS
        bsr     ARRAY_MULTIPLY
        puls    x
        bcs     ARRAY_RECORD_FRAME_BAD
        leax    2,x
        cmpx    ,s
        blo     ARRAY_RECORD_EXTENT
        ; A wrapped sum is below the header end, so it cannot equal the
        ; validated record end U. The following equality also rejects carry.
        addd    ,s
        pshs    u
        cmpd    ,s++
        bne     ARRAY_RECORD_FRAME_BAD
ARRAY_RECORD_VALID
        ldd     ,s
        subd    2,s
        tfr     u,x
        leas    2,s
        puls    y,u,pc
ARRAY_RECORD_POP
        leas    2,s
ARRAY_RECORD_FRAME_BAD
        leas    2,s
ARRAY_RECORD_BAD
        coma
ARRAY_RECORD_RETURN
        puls    y,u,pc


; GET's G form marks the last extent of a numeric array zero.
; Its retained byte length must still be a positive multiple of the other
; extent times five. No graphics bytes are interpreted as string descriptors.
ARRAY_RECORD_GRAPHICS
        puls    x
        pshs    d
        leax    2,x
        cmpx    2,s
        bne     ARRAY_RECORD_POP
        ldy     4,s
        tst     1,y
        bmi     ARRAY_RECORD_POP
        tfr     u,d
        subd    2,s
ARRAY_GRAPHICS_EXTENT
        subd    ,s
        bhi     ARRAY_GRAPHICS_EXTENT
        bne     ARRAY_RECORD_POP
        leas    2,s
        bra     ARRAY_RECORD_VALID

