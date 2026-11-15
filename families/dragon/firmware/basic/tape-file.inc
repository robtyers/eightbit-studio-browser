; Original named cassette-file search and skip. Apache-2.0.
; The name/cursor/block budget live on the native stack. Search/skip blocks use
; $01DA-$02D8; selected binary blocks stream into their public RAM destination.
; U is eight bytes into the 17-byte native frame: name -8, cursor 0,
; block budget 2, offset/remaining 4, destination 6, operation 8.
CMD_SKIPF
        clrb
FILE_BEGIN
        leas    -17,s
        leau    8,s
        stb     8,u
        leay    -8,u
        ldb     #8
        lda     #32
FILE_EMPTY_NAME
        sta     ,y+
        decb
        bne     FILE_EMPTY_NAME
        clr     -8,u
        jsr     EX_END
        beq     FILE_NAME_READY
        jsr     EX_COMMA
        beq     FILE_NAME_READY
        jsr     EVAL_NATIVE
        tst     $06
        lbeq    NATIVE_FAIL
        ldb     FAC
        cmpb    #8
        jsr     NATIVE_CHECK_HI
        pshs    x
        leay    -8,u
        ldx     STR_PTR
        tstb
        beq     FILE_NAME_COPIED
FILE_NAME_COPY
        jsr     COPY_BYTES
FILE_NAME_COPIED
        puls    x
FILE_NAME_READY
        clra
        clrb
        std     4,u
        lda     8,u
        cmpa    #1
        bne     FILE_ARGUMENTS_READY
        jsr     EX_OPTIONAL_COMMA
        bne     FILE_ARGUMENTS_READY
        jsr     EVAL_ADDRESS
FILE_OFFSET_POSITIVE
FILE_OFFSET_READY
        std     4,u
FILE_ARGUMENTS_READY
        lda     8,u
        cmpa    #3
        lbhs    FILE_SAVE
        jsr     CMD_REQUIRE_END
        stx     0,u
        ldd     #$ffff
        std     2,u
        jsr     MOTORON
FILE_SEARCH
        bsr     FILE_READ_BLOCK
        tst     $7c
        bne     FILE_SEARCH
        ldb     $7d
        cmpb    #15
        bne     FILE_BAD_BLOCK
        tst     -8,u
        beq     FILE_FOUND
        ldx     #$01da
        leay    -8,u
        ldb     #8
FILE_NAME_COMPARE
        jsr     COMPARE_BYTES
        bne     FILE_SEARCH
FILE_FOUND
        tst     8,u
        beq     FILE_SKIP
        jsr     FILE_LOAD_HEADER
FILE_SKIP
        bsr     FILE_READ_BLOCK
        lda     $7c
        cmpa    #1
        beq     FILE_DATA
        cmpa    #$ff
        bne     FILE_BAD_BLOCK
        tst     $7d
        bne     FILE_BAD_BLOCK
FILE_STOP
        lda     8,u
        cmpa    #-3
        bne     FILE_STOP_TOKENISED
        ; A source line needs CR. Discard an unfinished EOF tail, matching the
        ; observed loader rather than silently storing an extra programme line.
        lda     #$ff
        sta     EXEC_MODE
        bra     FILE_STOP_MOTOR
FILE_STOP_TOKENISED
        cmpa    #-2
        bne     FILE_STOP_MOTOR
        jsr     FILE_BASIC_FINISH
FILE_STOP_MOTOR
        jsr     MOTOROFF
        ldx     0,u
        leas    17,s
        clra
        rts
FILE_DATA
        lda     8,u
        cmpa    #-3
        lbeq    FILE_ASCII_DATA
        cmpa    #-2
        lbeq    FILE_BASIC_DATA
        tsta
        beq     FILE_SKIP
        clra
        ldb     $7d
        addd    6,u
        std     6,u
        bra     FILE_SKIP
FILE_READ_BLOCK
        ldd     2,u
        subd    #1
        beq     FILE_LIMIT
        std     2,u
        lda     8,u
        cmpa    #-1
        beq     FILE_READ_BINARY
        ldd     #$01da
        bra     FILE_READ_DESTINATION
FILE_READ_BINARY
        ldd     6,u
FILE_READ_DESTINATION
        std     $7e
        jsr     TAPE_BLOCK_IN
        bcs     FILE_IO_ERROR
        rts
FILE_BAD_BLOCK
        lda     #2
        bra     FILE_STATUS_ERROR
FILE_LIMIT
        lda     #3
FILE_STATUS_ERROR
        sta     $81
FILE_IO_ERROR
        lda     8,u
        cmpa    #-2
        bgt     FILE_IO_STOP
        jsr     NEW_PROGRAM
FILE_IO_STOP
        jmp     NATIVE_TAIL_4


