; Original Eightbit Studio text-list demonstration. Apache-2.0.
; Optional example; earlier cartridges remain unchanged.
        org 0
        fcc "g GCE 2026"
        fcb $80
        fdb 0
        fcb $F8,$50,$20,$C0
        fcc "TEXT LISTS"
        fcb $80,0
start:
frame:  jsr $F192
        jsr $F2A9
        ldu #sized
        jsr $F385
        ldu #positioned
        jsr $F38C
        bra frame
sized:  fcb -8,40,24,-28
        fcc "A"
        fcb $80
        fcb -8,40,24,12
        fcc "B"
        fcb $80,0
positioned: fcb -24,-28
        fcc "C"
        fcb $80
        fcb -24,12
        fcc "D"
        fcb $80,0
