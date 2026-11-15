.segment "EDITOR"
; Original screen-to-input capture. The public logical-row links support two
; physical rows (80 bytes), re-editing, shifted glyphs and quoted control codes.
; Used by BASIC's bounded direct editor and the real KERNAL CHRIN service.
editor_capture:
    lda #0
    beq capture_screen
editor_capture_input:
    lda #1
capture_screen:
    pha
    lda $0B
    pha
    lda $0F
    pha
    lda $4C
    pha
    lda $4D
    pha
    lda $D6
    pha
    lda $D3
    pha
    ldx $D6
@start:
    lda $D9,x
    bmi @found
    cpx #0
    beq @found
    dex
    jmp @start
@found:
    stx $4D
    lda #0
    sta $C8
    tsx
    lda $0107,x
    beq @begin
    sec
    lda $C9
    sbc $4D
    beq @column
    lda #40
@column:
    clc
    adc $CA
    sta $C8
@begin:
    lda #0
    sta $0B
    sta $0F
    sta $4C
@row:
    lda $4D
    sta $D6
    jsr cursor_address
    ldy #0
@cell:
    lda $C8
    beq @capture
    dec $C8
    jmp @next
@capture:
    ldx $0B
    cpx #80
    bcs @beyond
    lda ($D1),y
    jsr editor_decode
    sta $0200,x
    inc $0B
    jmp @next
@beyond:
    lda ($D1),y
    cmp #32
    beq @next
    lda #1
    sta $4C
@next:
    iny
    cpy #40
    bcc @cell
    inc $4D
    ldx $4D
    cpx #25
    bcs @finish
    lda $D9,x
    bpl @row
@finish:
    ldx $0B
@trim:
    cpx #0
    beq @length
    lda $01FF,x
    cmp #32
    bne @length
    dex
    jmp @trim
@length:
    stx $D5
    lda #0
    sta $0200,x
    lda $4C
    sta $D7
    pla
    sta $D3
    pla
    sta $D6
    pla
    sta $4D
    pla
    sta $4C
    pla
    sta $0F
    pla
    sta $0B
    pla
    jsr cursor_address
    ldx $D5
    ldy $D7
    rts
editor_decode:
    cmp #128
    bcc @normal
    pha
    lda $0F
    beq @unreverse
    pla
    cmp #160
    bcc @low_control
    cmp #192
    bcc @unreverse_direct
    cmp #224
    bcs @unreverse_direct
    sec
    sbc #64
    rts
@low_control:
    and #31
    rts
@unreverse:
    pla
@unreverse_direct:
    and #127
@normal:
    cmp #32
    bcc @letter
    cmp #64
    bcc @quote
    cmp #94
    beq @pi
    cmp #96
    bcs @block
    ora #128
    rts
@pi:
    lda #255
    rts
@block:
    clc
    adc #64
    rts
@letter:
    clc
    adc #64
@quote:
    cmp #34
    bne @done
    pha
    lda $0F
    eor #1
    sta $0F
    pla
@done:
    rts

editor_read:
    txa
    pha
    tya
    pha
    lda $D0
    cmp #1
    beq @read
    lda #2
    sta $D0
    lda #0
    sta $D5
    lda $D6
    sta $C9
    lda $D3
    sta $CA
@key:
    jsr getin_default
    beq @key
    cmp #3
    beq @key
    cmp #13
    beq @enter
    cmp #32
    bcc @control
    ldx $D5
    cpx #80
    bcs @key
    inc $D5
@control:
    jsr chrout_default
    jmp @key
@enter:
    jsr editor_capture_input
    lda #0
    sta $C8
    lda #1
    sta $D0
@read:
    ldx $C8
    cpx $D5
    bcc @character
    lda #0
    sta $D0
    lda #13
    bne @return
@character:
    lda $0200,x
    inc $C8
@return:
    sta $D7
    pla
    tay
    pla
    tax
    lda $D7
    clc
    rts

editor_span:
    jsr cursor_address
    sec
    lda #40
    sbc $D3
    sta $D7
    ldx $D6
    cpx #24
    bcs @address
    lda $DA,x
    bmi @address
    clc
    lda $D7
    adc #40
    sta $D7
@address:
    clc
    lda $D1
    adc $D3
    sta $D1
    lda $D2
    adc #0
    sta $D2
    clc
    lda $D1
    adc #1
    sta $AC
    lda $D2
    adc #0
    sta $AD
    clc
    lda $F3
    adc $D3
    sta $F3
    lda $F4
    adc #0
    sta $F4
    clc
    lda $F3
    adc #1
    sta $AE
    lda $F4
    adc #0
    sta $AF
    rts
editor_delete:
    jsr editor_span
    ldx $D7
    dex
    ldy #0
@byte:
    cpx #0
    beq @blank
    lda ($AC),y
    sta ($D1),y
    lda ($AE),y
    sta ($F3),y
    iny
    dex
    jmp @byte
@blank:
    lda #32
    sta ($D1),y
    lda $0286
    sta ($F3),y
    rts
editor_insert:
    jsr editor_span
    ldy $D7
    dey
    dey
    bmi @blank
@byte:
    lda ($D1),y
    sta ($AC),y
    lda ($F3),y
    sta ($AE),y
    dey
    bpl @byte
@blank:
    ldy #0
    lda #32
    sta ($D1),y
    lda $0286
    sta ($F3),y
    inc $D8
    rts
.segment "CODE"
