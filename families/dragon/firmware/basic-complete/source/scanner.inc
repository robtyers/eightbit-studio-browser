; Original implementation of the published $9F/$A5 text-scanner RAM interface.
; The load operand occupies the required $A6/$A7 pointer. Apache-2.0.
SCANNER_TEMPLATE
        ; This twelve-byte template is copied to $009F: external targets must
        ; remain absolute, and the load operand must stay at $00A6/$00A7.
        jsr     NEXT_INCREMENT
        bra     SCANNER_LOAD
        nop
SCANNER_LOAD
        lda     $ffff
        jmp     CHAR_FLAGS
SCANNER_TEMPLATE_END
NEXT_INCREMENT
        inc     TXT_PTR+1
        bne     NEXT_INC_END
        inc     TXT_PTR
NEXT_INC_END
        rts
CHAR_FLAGS
        cmpa    #32
        beq     SCANNER_SKIP_SPACE
        cmpa    #':
        beq     CHAR_TERMINATOR
        tsta
        beq     CHAR_TERMINATOR
        cmpa    #'0
        blo     CHAR_OTHER
        cmpa    #'9
        bhi     CHAR_OTHER
        andcc   #$fa
        rts
CHAR_OTHER
        orcc    #1
        andcc   #$fb
        rts
CHAR_TERMINATOR
        orcc    #5
        rts
SCANNER_SKIP_SPACE
        jmp     $009f
