; Original Eightbit Studio packet-list demonstration. Apache-2.0.
; Optional example; the default starter and executive-demo stay unchanged.
        org 0
        fcc "g GCE 2026"
        fcb $80
        fdb 0
        fcb $F8,$50,$20,$C0
        fcc "PACKETS"
        fcb $80,0
start:  lda #1
        sta $C824
frame:  jsr $F192
        jsr $F2A9
        ldx #packet
        jsr $F404
        ldx #packet
        jsr $F408
        ldx #scaled
        jsr $F40C
        ldx #packet
        ldb #31
        jsr $F40E
        ldx #packet
        jsr $F410
        bra frame
scaled: fcb 63
packet: fcb 0,-12,-12
        fcb $FF,0,24,$FF,24,0,$FF,0,-24,$FF,-24,0
        fcb 1
