; Original tokenised-BASIC completion after preflighted direct cassette input.
; The public block cursor is authoritative; incoming links remain untrusted.
FILE_BASIC_FINISH
        ; Rebuild links from bounded line bodies, never follow input pointers.
        ldd     $7e
        subd    #2
        bcs     FILE_BASIC_BAD
        std     REC_LIMIT
        ldx     TXTTAB
        cmpx    REC_LIMIT
        bhi     FILE_BASIC_BAD
        ldb     #1
        jsr     PROGRAM_WALK
        bcs     FILE_BASIC_BAD
        ldd     $7e
        jsr     VARIABLE_ENDS
        lda     #$ff
        sta     EXEC_MODE
FILE_BASIC_RETURN
        rts
FILE_BASIC_BAD
        jmp     FILE_BAD_BLOCK
