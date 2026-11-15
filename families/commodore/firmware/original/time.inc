.segment "CODE"
; Original public ST/TI/TI$ services, using real guest KERNAL clock calls.
special_variable:
    lda $45
    cmp #'S'
    bne @time
    lda $46
    cmp #'T'
    bne @ordinary
    lda $90
    jsr byte_number
    sec
    rts
@time:
    cmp #'T'
    bne @ordinary
    lda $46
    cmp #'I'
    beq @ticks
    cmp #('I'+$80)
    bne @ordinary
    jsr time_string
    sec
    rts
@ticks:
    jsr $FFDE
    sta $64
    stx $63
    sty $62
    lda #0
    sta $65
    sta $66
    sta $0D
    lda #152
    sta $61
    lda $62
    ora $63
    ora $64
    beq @zero
@normalise:
    lda $62
    bmi @done
    asl $64
    rol $63
    rol $62
    dec $61
    jmp @normalise
@zero:
    lda #0
    sta $61
@done:
    sec
    rts
@ordinary:
    clc
    rts

time_string:
    lda #6
    jsr string_allocate
    jsr $FFDE
    sta $50
    stx $51
    sty $52
    lda #60
    sta $56
    lda #0
    sta $57
    jsr clock_divide
    lda #<3600
    sta $56
    lda #>3600
    sta $57
    jsr clock_divide
    ldy #0
    lda $50
    jsr clock_digits
    copy_pointer $54, $50
    lda #0
    sta $52
    sta $57
    lda #60
    sta $56
    jsr clock_divide
    lda $50
    jsr clock_digits
    lda $54
    jsr clock_digits
    lda #6
    sta $61
    copy_pointer ptr, $62
    jmp string_temporary
clock_digits:
    ldx #0
@tens:
    cmp #10
    bcc @units
    sec
    sbc #10
    inx
    jmp @tens
@units:
    pha
    txa
    clc
    adc #'0'
    sta (ptr),y
    iny
    pla
    clc
    adc #'0'
    sta (ptr),y
    iny
    rts
clock_divide:
    lda #0
    sta $54
    sta $55
    lda #24
    sta $58
@bit:
    asl $50
    rol $51
    rol $52
    rol $54
    rol $55
    lda $55
    cmp $57
    bcc @next
    bne @subtract
    lda $54
    cmp $56
    bcc @next
@subtract:
    sec
    lda $54
    sbc $56
    sta $54
    lda $55
    sbc $57
    sta $55
    inc $50
@next:
    dec $58
    bne @bit
    rts

time_assignment:
    jsr skip_spaces
    cmp #$B2
    jne syntax_error
    jsr consume
    jsr expression
    jsr require_string
    jsr check_end
    jsr string_refresh
    lda $61
    cmp #6
    jne quantity_error
    copy_pointer $62, ptr
    lda #0
    sta $50
    sta $51
    sta $52
    ldy #0
@pair:
    lda (ptr),y
    jsr clock_digit
    sta $56
    asl
    asl
    clc
    adc $56
    asl
    sta $56
    iny
    lda (ptr),y
    jsr clock_digit
    clc
    adc $56
@accumulate:
    pha
    jsr clock_multiply60
    pla
    clc
    adc $50
    sta $50
    lda $51
    adc #0
    sta $51
    lda $52
    adc #0
    sta $52
    iny
    cpy #6
    bcc @pair
    jsr clock_multiply60
    lda $50
    ldx $51
    ldy $52
    jsr $FFDB
    jsr string_release
    jmp dispatch
clock_digit:
    cmp #'0'
    jcc quantity_error
    cmp #('9'+1)
    jcs quantity_error
    sec
    sbc #'0'
    rts
clock_multiply60:
    ldx #2
@copy:
    lda $50,x
    sta $53,x
    dex
    bpl @copy
    ldx #4
@times16:
    asl $50
    rol $51
    rol $52
    dex
    bne @times16
    sec
    lda $50
    sbc $53
    sta $50
    lda $51
    sbc $54
    sta $51
    lda $52
    sbc $55
    sta $52
    asl $50
    rol $51
    rol $52
    asl $50
    rol $51
    rol $52
    rts
.segment "PAGE0"
