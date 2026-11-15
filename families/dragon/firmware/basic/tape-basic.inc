; Original bounded tokenised-BASIC cassette loading. Apache-2.0.
; Data blocks are staged in the public cassette buffer before their extent is
; checked against the live stack limit. Invalid/partial files leave NEW state.
FILE_BASIC_DATA
        clra
        ldb     $7d
        addd    6,u
        bcs     FILE_BASIC_BAD
        pshs    d
        addd    #128
        bcs     FILE_BASIC_BAD
        cmpd    STACKTOP
        bhs     FILE_BASIC_BAD
        ldy     6,u
        puls    d
        std     6,u
        ldx     #$01da
        ldb     $7d
        beq     FILE_BASIC_NEXT
        jsr     COPY_BYTES
FILE_BASIC_NEXT
        jmp     FILE_SKIP
FILE_BASIC_BAD
        jmp     FILE_BAD_BLOCK
FILE_BASIC_FINISH
        ; Rebuild links from bounded line bodies, never follow input pointers.
        ldd     6,u
        subd    #2
        bcs     FILE_BASIC_BAD
        std     REC_LIMIT
        ldx     TXTTAB
        cmpx    REC_LIMIT
        bhi     FILE_BASIC_BAD
        ldb     #1
        jsr     PROGRAM_WALK
        bcs     FILE_BASIC_BAD
        ldd     6,u
        jsr     VARIABLE_ENDS
        lda     #$ff
        sta     EXEC_MODE
FILE_BASIC_RETURN
        rts

