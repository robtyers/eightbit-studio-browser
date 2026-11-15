; B is a reviewed output device; U is an original guest output routine.
; Restore the prior device on both normal return and language errors.
PRINT_SELECTED
        lda     DEVNUM
        pshs    a
        stb     DEVNUM
        jsr     EX_SPACE
        jsr     ,u
        puls    a
        sta     DEVNUM
        rts
; Return the active output column in A while preserving the TAB count in B.
PRINT_COLUMN
        lda     DEVNUM
        cmpa    #$fe
        beq     PRINT_COLUMN_PRINTER
        lda     $6c
        rts
PRINT_COLUMN_PRINTER
        lda     $9c
        rts

