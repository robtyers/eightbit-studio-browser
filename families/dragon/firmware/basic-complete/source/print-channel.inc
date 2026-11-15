; B is a reviewed output device; U is an original guest output routine.
; Restore the prior device on normal return; errors expose the live selection.
PRINT_SELECTED
        lda     DEVNUM
        pshs    a
        stb     DEVNUM
        jsr     EX_SPACE
        jsr     ,u
        puls    a
        sta     DEVNUM
        rts
; PRINT uses screen columns only for device zero; cassette has no column,
; while every other nonzero selector uses the published printer column.
; This column rule is intentionally distinct from B54A's byte sink routing.
PRINT_COLUMN
        lda     DEVNUM
        beq     PRINT_COLUMN_SCREEN
        inca
        beq     PRINT_COLUMN_SAVE
PRINT_COLUMN_PRINTER
        lda     $9c
        bra     PRINT_COLUMN_SAVE
PRINT_COLUMN_SCREEN
        lda     CURSOR+1
        anda    #31
PRINT_COLUMN_SAVE
        sta     $6c
        rts
