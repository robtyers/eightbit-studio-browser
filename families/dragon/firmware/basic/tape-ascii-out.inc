; Original ASCII programme output. LIST supplies original detokenised text;
; its bytes stream through the existing cassette buffer, never a second copy
; of the programme. Negative FORMAT_BUFFER selects this temporary output sink.
; The public cassette count $79 stays separate from temporary RAM formatting.
FILE_SAVE_ASCII
        clr     $79
        lda     #$ff
        sta     FORMAT_BUFFER
        lda     #13
        jsr     CHROUT
        ldx     #EMPTY_TEXT
        jsr     CMD_LIST
        clr     FORMAT_BUFFER
        ldb     $79
        beq     FILE_ASCII_EOF
        bsr     FILE_ASCII_BLOCK
FILE_ASCII_EOF
        bsr     FILE_ASCII_PAUSE
        jmp     FILE_SAVE_EOF
FILE_ASCII_BYTE
        pshs    d,x
        ldx     #$01da
        ldb     $79
        abx
        sta     ,x
        incb
        stb     $79
        cmpb    #255
        bne     FILE_ASCII_RETURN
        bsr     FILE_ASCII_BLOCK
FILE_ASCII_RETURN
        puls    d,x,pc
; ASCII files are gapped. The original finite delay precedes the leader so a
; reader can finish its line processing before the next physical data block.
FILE_ASCII_BLOCK
        lda     #1
        std     $7c
        ldd     #$01da
        std     $7e
        bsr     FILE_ASCII_PAUSE
        jsr     TAPE_PREP
        jsr     TAPE_BLOCK_OUT
        bcs     FILE_ASCII_FAILED
        clr     $79
        rts
FILE_ASCII_FAILED
        clr     FORMAT_BUFFER
NATIVE_TAIL_4
        jsr     MOTOROFF
        jmp     NATIVE_FAIL
FILE_ASCII_PAUSE
        pshs    y
        ldy     #55930
FILE_ASCII_GAP
        leay    -1,y
        bne     FILE_ASCII_GAP
        puls    y,pc

