; Original Eightbit Studio Vectrex Vectrex demonstration, Apache-2.0.
; BIOS entry addresses and hardware facts: see GUIDE.html references and provenance.
; A single source file. No firmware bytes or third-party implementation code.
        org     0
        fcc     "g GCE 2026"
        fcb     $80
        fdb     $FD0D
        fcb     $F8,$50,$20,$C0
        fcc     "VECTOR STUDIO"
        fcb     $80,0

X_POS:  equ     $C880
Y_POS:  equ     $C881
TICK:   equ     $C882

start:  clr     X_POS
        clr     Y_POS
        clr     TICK
        lda     #1
        sta     $C81F
        lda     #3
        sta     $C820
        clr     $C821
        clr     $C822
frame:  jsr     $F192           ; Wait_Recal: begin a stable display frame.
        jsr     $F1BA           ; Read_Btns.
        jsr     $F1F8           ; Joy_Digital.
        lda     $C81B
        beq     vertical
        bmi     left
        inc     X_POS
        bra     vertical
left:   dec     X_POS
vertical:
        lda     $C81C
        beq     draw
        bmi     down
        inc     Y_POS
        bra     draw
down:   dec     Y_POS
draw:   inc     TICK
        jsr     $F2A9           ; Intensity_7F.
        jsr     $F354           ; Reset0Ref.
        lda     #$60
        sta     $D004           ; Vector scale.
        lda     Y_POS
        ldb     X_POS
        jsr     $F312           ; Moveto_d.
        ldd     #$1818
        jsr     $F3DF           ; Draw_Line_d: four sides of a diamond.
        ldd     #$E818
        jsr     $F3DF
        ldd     #$E8E8
        jsr     $F3DF
        ldd     #$18E8
        jsr     $F3DF
        jsr     $F354
        lda     TICK
        anda    #$3F
        suba    #$20
        ldb     #$C0
        jsr     $F312
        ldd     #$0040
        jsr     $F3DF           ; An independently moving line.
        lda     #0
        ldb     #$A0
        jsr     $F256           ; Sound_Byte: channel A fine period.
        lda     #1
        ldb     #1
        jsr     $F256
        lda     #7
        ldb     #$3E            ; Tone A enabled; other tone/noise channels off.
        jsr     $F256
        ldb     #0
        lda     $C80F
        bita    #1
        beq     volume
        ldb     #8
volume: lda     #8
        jsr     $F256           ; Button 1 controls channel A volume.
        lbra    frame
