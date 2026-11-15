; Original Eightbit Studio executive. Apache-2.0. Interface/hardware facts only.
; All services run as guest instructions. No copied ROM, game or emulator code.
        org $E000
BOOT:   orcc #$50
        lds #$CC00
        ldx #$C800
        clra
INITRAM: sta ,x+
        cmpx #$CC00
        bne INITRAM
        lda #$9F
        sta $D002
        lda #$FF
        sta $D003
        lda #$81
        sta $D000
        lda #$80
        sta $D00B
        lda #$CE
        sta $D00C
        lda #$7F
        sta $D004
        lda #$30
        sta $D008
        lda #$75
        sta $D009
        ldd #$3075
        std $C83D
        ldd #$F850
        std $C82A
        ldx #0
HEADER: lda ,x+
        cmpa #$80
        bne HEADER
        leax 6,x
TITLE:  lda ,x+
        cmpa #$80
        bne TITLE
        leax 1,x
        jmp ,x
WAIT:
FRAMEWAIT: lda $D00D
        bita #$20
        beq FRAMEWAIT
        ldd $C83D
        sta $D008
        stb $D009
        ldd $C825
        addd #1
        std $C825
        jsr ZERO
        ldx #$F9F4
        lda #$D0
        tfr a,dp
        ldd #$0301
        rts
DPD0:   lda #$D0
        tfr a,dp
        rts
DPC8:   lda #$C8
        tfr a,dp
        rts
BUTTONS:
        lda $C80F
        sta $C810
        lda #14
        sta $D001
        lda #$99
        sta $D000
        lda #$81
        sta $D000
        clr $D003
        lda #$89
        sta $D000
        lda $D001
        coma
        sta $C80F
        lda #$81
        sta $D000
        lda #$FF
        sta $D003
        lda $C810
        coma
        anda $C80F
        sta $C811
        ldx #$C812
        ldb #1
BUTTONLOOP: bitb $C811
        beq BUTTONUP
        stb ,x+
        bra BUTTONNEXT
BUTTONUP: clr ,x+
BUTTONNEXT: lslb
        bne BUTTONLOOP
        lda $C811
        rts
JOYSTICKS: pshs d,x,y,u
        ldx #$C81B
        ldy #$C81F
        ldu #4
JOYLOOP: lda ,y+
        beq JOYZERO
        deca
        anda #6
        ora #$81
        sta $D000
        clr $D001
        lda $D000
        bita #$20
        bne JOYPOS
        lda #$FF
        sta $D001
        lda $D000
        bita #$20
        bne JOYZERO
        lda #$FF
        bra JOYSET
JOYPOS: lda #1
        bra JOYSET
JOYZERO: clra
JOYSET: sta ,x+
        leau -1,u
        cmpu #0
        bne JOYLOOP
        puls d,x,y,u,pc
SOUND:  ldx #$C800
        pshs b
        stb a,x
        puls b
        pshs d
        sta $D001
        lda #$99
        sta $D000
        lda #$81
        sta $D000
        stb $D001
        lda #$91
        sta $D000
        lda #$81
        sta $D000
        puls d
        ldb #1
        rts
I1F:    lda #$1F
        bra INTENSITY
I3F:    lda #$3F
        bra INTENSITY
I5F:    lda #$5F
        bra INTENSITY
I7F:    lda #$7F
INTENSITY: anda #$7F
        sta $C827
        pshs d
        sta $D001
        lda #$84
        sta $D000
        lda #$85
        sta $D000
        puls d
        ldd #$0501
        rts
ZERO:   pshs d
        lda #$CC
        sta $D00C
        clr $D001
        lda #$80
        sta $D000
        lda #$81
        sta $D000
        lda #$CE
        sta $D00C
        puls d
        ldd #$0301
        rts
MOVE:   pshs d
        sta $D001
        lda #$80
        sta $D000
        lda #$81
        sta $D000
        stb $D001
        lda #$CE
        bra MOTION
DRAW:   pshs d
        sta $D001
        lda #$80
        sta $D000
        lda #$81
        sta $D000
        stb $D001
        lda #$EE
MOTION: sta $D00C
        clr $D005
MOTIONWAIT: lda $D00D
        bita #$40
        beq MOTIONWAIT
        lda #$CE
        sta $D00C
        puls d,pc
; Counted lists use only the published LIST byte; temporary state is on S.
VCOUNT: lda ,x+
VA:     sta $C823
VL:     ldd ,x++
VLINE:  jsr DRAW
        tst $C823
        beq VDONE
        dec $C823
        bra VL
VDONE:  tst $C824
        beq VRETURN
        jsr ZERO
VRETURN: rts
VONE:   leax 2,x
        bra VLINE
VCS:    lda ,x+
        sta $C823
        ldb ,x+
        stb $D004
        bra VL
VAB:    stb $D004
        bra VA
VB:     stb $D004
        bra VL
; Duffy lists move to their first pair, then draw the remaining count.
MCOUNT: lda ,x+
MA:     sta $C823
ML:     ldd ,x++
        jsr MOVE
        tst $C823
        beq VDONE
        dec $C823
        bra VL
MCS:    lda ,x+
        sta $C823
        ldb ,x+
        stb $D004
        bra ML
MAB:    stb $D004
        bra MA
MB:     stb $D004
        bra ML
MD:     leax 2,x
        jsr MOVE
        tst $C823
        beq VDONE
        dec $C823
        bra VL
; Packet lists accept only the published 00/FF/01 mode bytes.
; A private stack counter bounds 256 vectors; X stays on the terminator.
PL:     cmpx #$FFFF
        beq BADPACKET
        ldb ,x+
PB:     stb $D004
PACKET: pshs d
        ldd #256
        std ,s
PNEXT:  lda ,x
        cmpa #1
        beq PEND
        tsta
        beq PMODE
        cmpa #$FF
        bne BADPACKET
PMODE:  cmpx #$FFFC
        bhi BADPACKET
        ldd ,s
        beq BADPACKET
        subd #1
        std ,s
        lda ,x+
        pshs a
        ldd ,x++
        tst ,s+
        beq PBLANK
        jsr DRAW
        bra PNEXT
PBLANK: jsr MOVE
        bra PNEXT
PEND:   leas 2,s
        jmp VDONE
BADPACKET: fcb $02
; Patterned motion uses the board's rotating T2-clocked shift register.
; Reload DASH per vector. Keep T2/scale untouched and save the caller's ACR on S.
PDRAW:  pshs d
        sta $D001
        lda #$80
        sta $D000
        lda #$81
        sta $D000
        stb $D001
        lda $D00B
        pshs a
        anda #$E3
        ora #$10
        sta $D00B
        lda $C829
        sta $D00A
        lda #$0E
        sta $D00C
        clr $D005
PDWAIT: lda $D00D
        bita #$40
        beq PDWAIT
        lda #$CE
        sta $D00C
        puls a
        sta $D00B
        puls d,pc
DA:     sta $C823
DL:     cmpx #$FFFD
        bhi BADPATTERN
        ldd ,x++
        jsr PDRAW
        tst $C823
        beq DEND
        dec $C823
        bra DL
DEND:   jmp VDONE
; Unlike PACKET, DASHPK uses 02 for solid and FF for patterned strokes.
DPACKET: tst $C823
        bne BADPATTERN
        ldd #256
        pshs d
DPNEXT: cmpx #$FFFF
        beq BADPATTERN
        lda ,x
        cmpa #1
        beq DPEND
        tsta
        beq DPMODE
        cmpa #2
        beq DPMODE
        cmpa #$FF
        bne BADPATTERN
DPMODE: cmpx #$FFFC
        bhi BADPATTERN
        ldd ,s
        beq BADPATTERN
        subd #1
        std ,s
        lda ,x+
        pshs a
        ldd ,x++
        tst ,s+
        beq DPBLANK
        bpl DPSOLID
        jsr PDRAW
        bra DPNEXT
DPSOLID: jsr DRAW
        bra DPNEXT
DPBLANK: jsr MOVE
        bra DPNEXT
DPEND:  leax 1,x
        leas 2,s
        jmp VDONE
BADPATTERN: fcb $02
; Original text glyphs are drawn through the same VIA motion primitives.
; 5 columns x 7 rows, one blank column; width/8 is the cell X velocity.
; Signed height is the row Y velocity. The cell timer is 16 cycles.
; Text-list headers and strings are validated before this message is drawn.
; S holds the list count and format; X scans without advancing the caller's U.
TLSIZE: lda #1
        bra TLSTART
TLPOS:  clra
TLSTART: pshs a
        ldd #64
        pshs d
TLNEXT: cmpu #$FFFF
        lbeq BADLIST
        lda ,u
        lbeq TLEND
        ldd ,s
        lbeq BADLIST
        subd #1
        std ,s
        tfr u,x
        ldb #2
        tst 2,s
        beq TLHEADER
        ldb #4
TLHEADER: cmpx #$FFFF
        lbeq BADLIST
        leax 1,x
        decb
        bne TLHEADER
        ldd #256
        pshs d
TLSCAN: cmpx #$FFFF
        lbeq BADLIST
        lda ,x+
        cmpa #$80
        beq TLVALID
        cmpa #$20
        lblo BADLIST
        cmpa #$6F
        lbhi BADLIST
        ldd ,s
        lbeq BADLIST
        subd #1
        std ,s
        bra TLSCAN
TLVALID: leas 2,s
        tst 2,s
        beq TLPDRAW
        jsr THW
        bra TLNEXT
TLPDRAW: jsr TYX
        bra TLNEXT
TLEND:  leau 1,u
        leas 3,s
        clra
        ldb #1
        ldx #$FBB4
        rts
BADLIST: fcb $02
THW:    ldd ,u++
        std $C82A
TYX:    ldd ,u++
TD:     pshs d
        jsr ZERO
        lda #$7F
        sta $D004
        puls d
        jsr MOVE
TEXT:   stu $C82C
        pshs y
        lda $D006
        pshs a
        ldd #256
        pshs d
        lda #15
        sta $D004
TCHAR:  lda ,u+
        cmpa #$80
        lbeq TEND
        cmpa #$20
        lblo BADTEXT
        cmpa #$6F
        lbhi BADTEXT
        pshs a
        ldd 1,s
        lbeq BADTEXT
        subd #1
        std 1,s
        puls a
        suba #$20
        ldb #7
        mul
        ldx #GLYPHS
        leax d,x
        ldy #7
TROW:   lda ,x+
        ldb #5
        pshs d
TCELL:  lsl ,s
        bcc TBLANK
        clra
        bsr TWIDTH
        jsr DRAW
        bra TNEXT
TBLANK: clra
        bsr TWIDTH
        jsr MOVE
TNEXT:  dec 1,s
        bne TCELL
        leas 2,s
        bsr TWIDTH
        pshs b
        lslb
        lslb
        addb ,s+
        negb
        lda $C82A
        jsr MOVE
        leay -1,y
        bne TROW
        ldy #7
TUP:    lda $C82A
        nega
        bvc TUPONE
        ldd #$4000
        jsr MOVE
        lda #$40
TUPONE: clrb
        jsr MOVE
        leay -1,y
        bne TUP
        bsr TWIDTH
        pshs b
        lslb
        addb ,s+
        lslb
        clra
        jsr MOVE
        lbra TCHAR
TWIDTH: ldb $C82B
        asrb
        asrb
        asrb
        rts
TEND:   leas 2,s
        puls a
        sta $D004
        puls y
        jsr ZERO
        ldx #$FBB4
        rts
BADTEXT: fcb $02
GLYPHS:
        fcb $00,$00,$00,$00,$00,$00,$00 ; $20
        fcb $20,$20,$20,$20,$20,$00,$20 ; $21
        fcb $50,$50,$50,$00,$00,$00,$00 ; $22
        fcb $50,$50,$F8,$50,$F8,$50,$50 ; $23
        fcb $20,$78,$A0,$70,$28,$F0,$20 ; $24
        fcb $C8,$D0,$10,$20,$40,$58,$98 ; $25
        fcb $60,$90,$A0,$40,$A8,$90,$68 ; $26
        fcb $20,$20,$40,$00,$00,$00,$00 ; $27
        fcb $10,$20,$40,$40,$40,$20,$10 ; $28
        fcb $40,$20,$10,$10,$10,$20,$40 ; $29
        fcb $00,$A8,$70,$F8,$70,$A8,$00 ; $2A
        fcb $00,$20,$20,$F8,$20,$20,$00 ; $2B
        fcb $00,$00,$00,$00,$20,$20,$40 ; $2C
        fcb $00,$00,$00,$F8,$00,$00,$00 ; $2D
        fcb $00,$00,$00,$00,$00,$20,$20 ; $2E
        fcb $08,$10,$10,$20,$40,$40,$80 ; $2F
        fcb $70,$88,$98,$A8,$C8,$88,$70 ; $30
        fcb $20,$60,$20,$20,$20,$20,$70 ; $31
        fcb $70,$88,$08,$10,$20,$40,$F8 ; $32
        fcb $F0,$08,$08,$70,$08,$08,$F0 ; $33
        fcb $10,$30,$50,$90,$F8,$10,$10 ; $34
        fcb $F8,$80,$80,$F0,$08,$08,$F0 ; $35
        fcb $70,$80,$80,$F0,$88,$88,$70 ; $36
        fcb $F8,$08,$10,$20,$40,$40,$40 ; $37
        fcb $70,$88,$88,$70,$88,$88,$70 ; $38
        fcb $70,$88,$88,$78,$08,$08,$70 ; $39
        fcb $00,$20,$20,$00,$20,$20,$00 ; $3A
        fcb $00,$20,$20,$00,$20,$20,$40 ; $3B
        fcb $08,$10,$20,$40,$20,$10,$08 ; $3C
        fcb $00,$00,$F8,$00,$F8,$00,$00 ; $3D
        fcb $80,$40,$20,$10,$20,$40,$80 ; $3E
        fcb $70,$88,$08,$10,$20,$00,$20 ; $3F
        fcb $70,$88,$B8,$A8,$B8,$80,$78 ; $40
        fcb $20,$50,$88,$88,$F8,$88,$88 ; $41
        fcb $F0,$88,$88,$F0,$88,$88,$F0 ; $42
        fcb $78,$80,$80,$80,$80,$80,$78 ; $43
        fcb $E0,$90,$88,$88,$88,$90,$E0 ; $44
        fcb $F8,$80,$80,$F0,$80,$80,$F8 ; $45
        fcb $F8,$80,$80,$F0,$80,$80,$80 ; $46
        fcb $78,$80,$80,$B8,$88,$88,$78 ; $47
        fcb $88,$88,$88,$F8,$88,$88,$88 ; $48
        fcb $70,$20,$20,$20,$20,$20,$70 ; $49
        fcb $38,$10,$10,$10,$10,$90,$60 ; $4A
        fcb $88,$90,$A0,$C0,$A0,$90,$88 ; $4B
        fcb $80,$80,$80,$80,$80,$80,$F8 ; $4C
        fcb $88,$D8,$A8,$A8,$88,$88,$88 ; $4D
        fcb $88,$C8,$C8,$A8,$98,$98,$88 ; $4E
        fcb $70,$88,$88,$88,$88,$88,$70 ; $4F
        fcb $F0,$88,$88,$F0,$80,$80,$80 ; $50
        fcb $70,$88,$88,$88,$A8,$90,$68 ; $51
        fcb $F0,$88,$88,$F0,$A0,$90,$88 ; $52
        fcb $78,$80,$80,$70,$08,$08,$F0 ; $53
        fcb $F8,$20,$20,$20,$20,$20,$20 ; $54
        fcb $88,$88,$88,$88,$88,$88,$70 ; $55
        fcb $88,$88,$88,$88,$88,$50,$20 ; $56
        fcb $88,$88,$88,$A8,$A8,$D8,$88 ; $57
        fcb $88,$88,$50,$20,$50,$88,$88 ; $58
        fcb $88,$88,$50,$20,$20,$20,$20 ; $59
        fcb $F8,$08,$10,$20,$40,$80,$F8 ; $5A
        fcb $70,$40,$40,$40,$40,$40,$70 ; $5B
        fcb $80,$40,$40,$20,$10,$10,$08 ; $5C
        fcb $70,$10,$10,$10,$10,$10,$70 ; $5D
        fcb $20,$50,$88,$00,$00,$00,$00 ; $5E
        fcb $00,$00,$00,$00,$00,$00,$F8 ; $5F
        fcb $40,$20,$10,$00,$00,$00,$00 ; $60
        fcb $00,$00,$70,$08,$78,$88,$78 ; $61
        fcb $80,$80,$B0,$C8,$88,$88,$F0 ; $62
        fcb $00,$00,$78,$80,$80,$80,$78 ; $63
        fcb $08,$08,$68,$98,$88,$88,$78 ; $64
        fcb $00,$00,$70,$88,$F8,$80,$78 ; $65
        fcb $30,$48,$40,$E0,$40,$40,$40 ; $66
        fcb $00,$78,$88,$88,$78,$08,$F0 ; $67
        fcb $80,$80,$B0,$C8,$88,$88,$88 ; $68
        fcb $20,$00,$60,$20,$20,$20,$70 ; $69
        fcb $10,$00,$30,$10,$10,$90,$60 ; $6A
        fcb $80,$80,$90,$A0,$C0,$A0,$90 ; $6B
        fcb $60,$20,$20,$20,$20,$20,$70 ; $6C
        fcb $00,$00,$D0,$A8,$A8,$A8,$A8 ; $6D
        fcb $00,$00,$B0,$C8,$88,$88,$88 ; $6E
        fcb $00,$00,$70,$88,$88,$88,$70 ; $6F
        fill $02,$F000-*
        jmp BOOT
        fill $02,$F192-*
        jmp WAIT
        fill $02,$F1AA-*
        jmp DPD0
        fill $02,$F1AF-*
        jmp DPC8
        fill $02,$F1BA-*
        jmp BUTTONS
        fill $02,$F1F8-*
        jmp JOYSTICKS
        fill $02,$F256-*
        jmp SOUND
        fill $02,$F29D-*
        jmp I1F
        fill $02,$F2A1-*
        jmp I3F
        fill $02,$F2A5-*
        jmp I5F
        fill $02,$F2A9-*
        lda #$7F
        jmp INTENSITY
        fill $02,$F312-*
        jmp MOVE
        fill $02,$F354-*
        jmp ZERO
        fill $02,$F373-*
        bra STUB_THW
        fill $02,$F378-*
        bra STUB_TYX
        fill $02,$F37A-*
        bra STUB_TD
        fill $02,$F385-*
        jmp TLSIZE
        fill $02,$F38C-*
        jmp TLPOS
        fill $02,$F39A-*
STUB_THW: jmp THW
STUB_TYX: jmp TYX
STUB_TD: jmp TD
; Jump banks fit only in unpublished gaps; do not occupy missing services.
STUB_MCOUNT: jmp MCOUNT
STUB_MB: jmp MB
STUB_MCS: jmp MCS
        fill $02,$F3AD-*
        bra STUB_MCOUNT
        fill $02,$F3B1-*
        bra STUB_MB
        fill $02,$F3B5-*
        bra STUB_MCS
        fill $02,$F3B7-*
        bra STUB_MAB
        fill $02,$F3B9-*
        bra STUB_MA
        fill $02,$F3BC-*
        bra STUB_ML
        fill $02,$F3BE-*
        bra STUB_MD
        fill $02,$F3CE-*
        bra STUB_VCOUNT
        fill $02,$F3D2-*
        bra STUB_VB
        fill $02,$F3D6-*
        bra STUB_VCS
        fill $02,$F3D8-*
        bra STUB_VAB
        fill $02,$F3DA-*
        bra STUB_VA
        fill $02,$F3DD-*
        bra STUB_VL
        fill $02,$F3DF-*
        bra STUB_VONE
        fill $02,$F3E1-*
STUB_MAB: jmp MAB
STUB_MA: jmp MA
STUB_ML: jmp ML
STUB_MD: jmp MD
STUB_VCOUNT: jmp VCOUNT
STUB_VB: jmp VB
STUB_VCS: jmp VCS
STUB_VAB: jmp VAB
STUB_VA: jmp VA
STUB_VL: jmp VL
STUB_VONE: jmp VONE
        fill $02,$F404-*
        ldb #$FF
        bra STUB_PB
        ldb #$7F
        bra STUB_PB
        bra STUB_PL
        bra STUB_PB
        bra STUB_PACKET
STUB_PL: jmp PL
STUB_PB: jmp PB
STUB_PACKET: jmp PACKET
        fill $02,$F434-*
        jmp DA
        bra STUB_DL
        fill $02,$F43B-*
STUB_DL: jmp DL
        fill $02,$F46E-*
        jmp DPACKET
        fill $02,$F495-*
        jmp TEXT
        fill $02,$FFF2-*
        fdb $EFFF,$EFFF,$EFFF,$EFFF,$EFFF,$EFFF,BOOT
