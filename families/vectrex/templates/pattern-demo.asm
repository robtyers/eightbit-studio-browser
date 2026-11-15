; Original Eightbit Studio patterned-vector demonstration. Apache-2.0.
; The default starter and other optional examples stay unchanged.
        org 0
        fcc "g GCE 2026"
        fcb $80
        fdb 0
        fcb $F8,$50,$20,$C0
        fcc "PATTERNS"
        fcb $80,0
start:  lda #1
        sta $C824
        lda #$AA
        sta $C829
frame:  jsr $F192
        jsr $F2A9
        lda #$FF
        sta $D004
        ldx #pairs
        lda #1
        jsr $F434
        ldx #pairs
        lda #1
        sta $C823
        jsr $F437
        ldx #packet
        jsr $F46E
        bra frame
pairs:  fcb 0,24,24,0
packet: fcb 0,-12,-12
        fcb 2,0,24,$FF,24,0,2,0,-24,$FF,-24,0
        fcb 1
