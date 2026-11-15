; Original Eightbit Studio executive conformance demonstration. Apache-2.0.
; Optional example: the default starter and its cartridge bytes remain unchanged.
        org 0
        fcc "g GCE 2026"
        fcb $80
        fdb 0
        fcb $F8,$50,$20,$C0
        fcc "EXECUTIVE"
        fcb $80,0
start:
frame:  jsr $F192
        jsr $F2A9
        ldu #title
        jsr $F373
        jsr $F354
        lda #$30
        sta $D004
        ldd #$E0E0
        jsr $F312
        ldx #outline
        jsr $F3CE
        jsr $F354
        ldx #strokes
        jsr $F3B5
        lbra frame
title:  fcb -16,80,40,-20
        fcc "ABI"
        fcb $80
outline: fcb 3
        fcb 24,24,-24,24,-24,-24,24,-24
strokes: fcb 3,48
        fcb -24,-24,0,48,24,0,0,-48
