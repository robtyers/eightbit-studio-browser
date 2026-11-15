; Independently authored radix-four integer square root of an exact 64-bit
; radicand. Thirty-two restoring steps and the integer remainder establish
; nearest significand rounding without a host float or polynomial dependency.
square_root:
    lda fexp
    jeq zero_value
    lda fsign
    jne quantity
    lda fexp
    clc
    adc #129
    ror
    sta sign                    ; floor((biased exponent + 129) / 2)
    ldx #0
@mantissa:
    lda fm,x
    sta $4B,x
    inx
    cpx #4
    bcc @mantissa
    lda #0
    ldx #17
@clear:
    sta $4F,x
    dex
    bpl @clear
    lda fexp
    and #1
    beq @start
    lsr $4B
    ror $4C
    ror $4D
    ror $4E
    ror $4F
    ror $50
    ror $51
    ror $52
@start:
    lda #32
    sta $0B
@digit:
    jsr sqrt_shift
    jsr sqrt_shift
    lda #0
    sta $5C
    ldx #3
@trial:
    lda $53,x
    sta $5D,x
    dex
    bpl @trial
    asl $60
    rol $5F
    rol $5E
    rol $5D
    rol $5C
    asl $60
    rol $5F
    rol $5E
    rol $5D
    rol $5C
    inc $60
    ldx #0
@compare:
    lda $57,x
    cmp $5C,x
    bne @ordered
    inx
    cpx #5
    bcc @compare
@ordered:
    bcc @zero_digit
    ldx #4
    sec
@subtract:
    lda $57,x
    sbc $5C,x
    sta $57,x
    dex
    bpl @subtract
    sec
    bcs @root_bit
@zero_digit:
    clc
@root_bit:
    rol $56
    rol $55
    rol $54
    rol $53
    dec $0B
    bne @digit
    lda $57
    bne @round
    ldx #0
@remainder:
    lda $58,x
    cmp $53,x
    bne @round_order
    inx
    cpx #4
    bcc @remainder
    jmp @result
@round_order:
    bcc @result
@round:
    inc $56
    bne @result
    inc $55
    bne @result
    inc $54
    bne @result
    inc $53
    bne @result
    lda #$80
    sta $53
    inc sign
@result:
    ldx #3
@copy:
    lda $53,x
    sta fm,x
    dex
    bpl @copy
    lda sign
    sta fexp
    lda #0
    sta fsign
    tax
    rts
sqrt_shift:
    asl $52
    rol $51
    rol $50
    rol $4F
    rol $4E
    rol $4D
    rol $4C
    rol $4B
    rol $5B
    rol $5A
    rol $59
    rol $58
    rol $57
    rts
