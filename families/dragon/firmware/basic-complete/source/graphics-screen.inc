; Original SAM/VDG selection. Only digital pins are written; no host rendering call.
CMD_SCREEN
        jsr     GFX_BYTE
        pshs    b
        ldb     $c1
        pshs    b
        jsr     EX_OPTIONAL_COMMA
        bne     SCREEN_ARGUMENTS
        jsr     GFX_BYTE
        tstb
        beq     SCREEN_COLOUR_READY
        ldb     #8
SCREEN_COLOUR_READY
        stb     ,s
SCREEN_ARGUMENTS
        jsr     CMD_REQUIRE_END
        puls    b
        stb     $c1
        puls    a
        pshs    x
        tsta
        beq     SCREEN_TEXT
        pshs    b
        lda     $b6
        adda    #3
        tfr     a,b
        lsla
        lsla
        lsla
        lsla
        ora     #$80
        ora     ,s+
        pshs    a
        cmpb    #7
        bne     SCREEN_GRAPHICS_SAM
        decb
SCREEN_GRAPHICS_SAM
        ldy     $ba
        bra     SCREEN_SAM
PARSE_FAILURE_12
        jmp     NATIVE_RANGE_ERROR
SCREEN_TEXT
        pshs    b
        clrb
        ldy     #$0400
SCREEN_SAM
        ldx     #$ffc0
        lda     #3
        bsr     SCREEN_SAM_BITS
        tfr     y,d
        lsra
        tfr     a,b
        lda     #7
        bsr     SCREEN_SAM_BITS
        puls    a
        ldb     $ff22
        andb    #7
        pshs    a
        orb     ,s+
        stb     $ff22
        jmp     NATIVE_TAIL_1


NATIVE_ISLAND_GRAPHICS_SCREEN_INC_0
        jmp     NATIVE_FAIL
SCREEN_SAM_BITS
        lsrb
        bcc     SCREEN_SAM_CLEAR
        sta     1,x
        bra     SCREEN_SAM_NEXT
SCREEN_SAM_CLEAR
        sta     ,x
SCREEN_SAM_NEXT
        leax    2,x
        deca
        bne     SCREEN_SAM_BITS
        rts

CMD_PCLEAR
        jsr     GFX_BYTE
        tstb
        beq     PARSE_FAILURE_12
        cmpb    #8
        bhi     PARSE_FAILURE_12
        ; The statement-end guard preserves the page count in B.
        jsr     CMD_REQUIRE_END
        jsr     GFX_PAGE_ADDRESS
        addd    #1
        cmpd    $b7
        bls     PARSE_FAILURE_12
        pshs    d,x
        jsr     VALIDATE_PROGRAM
        bcs     NATIVE_ISLAND_GRAPHICS_SCREEN_INC_0
        ldd     ,s
        subd    TXTTAB
        std     REC_DELTA
        addd    VARTAB
        std     REC_END
        addd    #128
        bcs     NATIVE_ISLAND_GRAPHICS_SCREEN_INC_0
        cmpd    STACKTOP
        bhs     NATIVE_ISLAND_GRAPHICS_SCREEN_INC_0
        ldd     REC_DELTA
        bmi     PCLEAR_DOWN
        ldx     VARTAB
        ldy     REC_END
PCLEAR_COPY_HIGH
        cmpx    TXTTAB
        beq     PCLEAR_COPIED
        lda     ,-x
        sta     ,-y
        bra     PCLEAR_COPY_HIGH
PCLEAR_DOWN
        ldx     TXTTAB
        ldy     ,s
PCLEAR_COPY_LOW
        jsr     COPY_PROGRAM_TAIL

PCLEAR_COPIED
        ldx     2,s
        cmpx    TXTTAB
        blo     PCLEAR_CURSOR
        cmpx    VARTAB
        bhs     PCLEAR_CURSOR
        ldd     REC_DELTA
        leax    d,x
PCLEAR_CURSOR
        stx     2,s
        ldd     ,s
        std     TXTTAB
        jsr     STORE_REINDEX
        ldd     EXEC_NEXT
        beq     PCLEAR_CLEARED
        addd    REC_DELTA
        std     EXEC_NEXT
PCLEAR_CLEARED
        jsr     CMD_CLEAR_VARIABLES
        leas    2,s
        puls    x,pc
