; Original C64 PAL acceptance: clear, print through CHROUT, return to BASIC.
.org $080D
    lda #147
    jsr $FFD2
    lda #6
    sta $D020
    lda #0
    sta $D021
    ldx #0
message_loop:
    lda message,x
    beq finished
    jsr $FFD2
    inx
    bne message_loop
finished:
    lda #7
    sta $D020
    rts
message:
    .byte 65,83,77,32,79,75,13,0
