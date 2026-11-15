; Original tokenised BASIC cassette output. Apache-2.0.
; FILE_BEGIN owns the name/cursor frame; reads use only the guest bus.
FILE_SAVE
        stx     0,u
        tst     -8,u
        bne     FILE_SAVE_NAME
        lda     #32
        sta     -8,u
FILE_SAVE_NAME
        leax    -8,u
        ldy     #$01da
        ldb     #8
        jsr     COPY_BYTES
        ldb     #7
FILE_SAVE_HEADER
        clr     ,y+
        decb
        bne     FILE_SAVE_HEADER
        ldx     0,u
        lda     8,u
        cmpa    #3
        beq     FILE_SAVE_MACHINE
        jsr     VALIDATE_PROGRAM
        bcs     NATIVE_SHARED12_TAPE_SAVE_INC_0
        ldd     TXTTAB
        std     6,u
        ldd     VARTAB
        subd    6,u
        std     4,u
        ldx     0,u
        jsr     EX_OPTIONAL_COMMA
        bne     FILE_SAVE_PARSED
        jsr     EX_ALPHA
        cmpa    #'A
        bne     NATIVE_SHARED12_TAPE_SAVE_INC_0
        leax    1,x
        ldd     #$ffff
        std     $01e3
        bra     FILE_SAVE_PARSED
NATIVE_SHARED12_TAPE_SAVE_INC_0
        jmp     NATIVE_FAIL
FILE_SAVE_MACHINE
        jsr     EVAL_NEXT_ADDRESS
        std     6,u
        std     $01e7
        jsr     EVAL_NEXT_ADDRESS
        std     4,u
        subd    6,u
        bcs     NATIVE_SHARED12_TAPE_SAVE_INC_0
        addd    #1
        std     4,u
        jsr     EVAL_NEXT_ADDRESS
        std     $01e5
        lda     #2
        sta     $01e2
FILE_SAVE_PARSED
        jsr     CMD_REQUIRE_END
        stx     0,u
        ldd     #128
        std     $90
        ldd     #$01da
        std     $7e
        ldd     #15
        std     $7c
        bsr     FILE_WRITE_BLOCK
        tst     $01e3
        lbne    FILE_SAVE_ASCII
        ldd     6,u
        std     $7e
FILE_SAVE_DATA
        ldd     4,u
        beq     FILE_SAVE_FULL
        tsta
        beq     FILE_SAVE_LENGTH
FILE_SAVE_FULL
        ldb     #255
FILE_SAVE_LENGTH
        lda     #1
        std     $7c
        bsr     FILE_WRITE_BLOCK
        clra
        ldb     $7d
        addd    $7e
        std     $7e
        ldd     4,u
        subb    $7d
        sbca    #0
        std     4,u
        bne     FILE_SAVE_DATA
FILE_SAVE_EOF
        ldd     #$ff00
        std     $7c
        bsr     FILE_WRITE_BLOCK
        jmp     FILE_STOP
FILE_WRITE_BLOCK
        jsr     TAPE_PREP
        jsr     TAPE_BLOCK_OUT
        lbcs    FILE_IO_ERROR
        rts

