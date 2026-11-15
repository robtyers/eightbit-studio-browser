; Original Apache-2.0 guest BASIC firmware.
.setcpu "6502"
.export banner_entry
.import warm_entry
.segment "PAGE0"
.word warm_entry, warm_entry
.res $FC, $EA

.segment "PAGE1"
.word warm_entry, warm_entry
.res $FC, $EA
banner_entry:
    ldx #0
@next:
    lda banner,x
    beq @done
    jsr $FFD2
    inx
    bne @next
@done:
    rts
banner:
.byte "EIGHTBIT C64 ORIGINAL",13
.byte "GUEST BASIC V2 - DEVELOPMENT",13
.byte "PAL 64K / ORIGINAL FIRMWARE",13,13,0
