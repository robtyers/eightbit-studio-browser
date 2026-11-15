; Observed TAB(n) pads to n from the current column, including across rows.
; It emits nothing when n is less than the current column.
PRINT_TAB
        leax    1,x
        jsr     GFX_BYTE
        jsr     MULTI_CLOSE
        lda     DEVNUM
        inca
        beq     PRINT_TAB_END
        lbsr     PRINT_COLUMN
        pshs    a
        subb    ,s+
        bls     PRINT_TAB_END
PRINT_TAB_SPACE
        lda     #32
        jsr     CHROUT
        decb
        bne     PRINT_TAB_SPACE
PRINT_TAB_END
        jmp     CMD_PRINT_END
