; Original Apache-2.0 guest numeric algorithms, independently authored.
; FAC/ARG layout is the public interface measured in the stage-3 corpus.
; Magnitudes use 32 significant binary bits, eight guard bits and sticky tails.
.setcpu "6502"
.include "branches.inc"
.export math_entry
.import warm_entry
.import device_absent, file_not_open, clrchn_default
.export open_default, close_default, chkin_default, chkout_default

fexp = $61
fm = $62
fsign = $66
aexp = $69
am = $6A
asign = $6E
sign = $6F
elo = $4D
ehi = $4E
f = $50                         ; five-byte aligned magnitude, big-endian
g = $55
count = $5F
status = $60
num = $14
temp = $54

.segment "PAGE2"
.word warm_entry, warm_entry
.res $FC, $EA
math_entry:
    cld
    cmp #0
    jeq add_values
    cmp #1
    jeq subtract_values
    cmp #2
    jeq multiply_values
    cmp #3
    jeq divide_values
    cmp #4
    jeq parse_number
    cmp #5
    jeq format_number
    cmp #6
    jeq signed_to_float
    cmp #7
    jeq float_to_signed
    cmp #8
    jeq compare_values
    cmp #9
    jeq logical_and
    cmp #10
    jeq logical_or
    cmp #11
    jeq logical_not
    cmp #12
    jeq sign_value
    cmp #13
    jeq absolute_value
    cmp #14
    jeq floor_value
    cmp #15
    jeq unsigned_word
    cmp #16
    jeq parse_string_number
    cmp #17
    jeq square_root
    cmp #18
    jeq random_value
    cmp #19
    jeq logarithm_value
    cmp #20
    jeq exponential_value
    cmp #21
    jeq cosine_value
    cmp #22
    jeq sine_value
    cmp #23
    jeq tangent_value
    cmp #24
    jeq arctangent_value
    cmp #25
    jeq power_value
    cmp #26
    jeq pi_value
    ldx #5                      ; explicitly unimplemented operation
    rts

sign_value:
    lda fexp
    jeq zero_value
    lda #1
    sta num
    lda #0
    sta num+1
    lda fsign
    bpl :+
    lda #$FF
    sta num
    sta num+1
:
    jmp signed_to_float
absolute_value:
    lda #0
    sta fsign
    tax
    rts
floor_value:
    lda fexp
    jeq zero_value
    cmp #129
    bcs @fraction
    lda fsign
    jeq zero_value
    lda #$FF
    sta num
    sta num+1
    jmp signed_to_float
@fraction:
    cmp #160
    bcs @done
    lda #160
    sec
    sbc fexp
    sta count
    lda #0
    sta status
    ldx #3
    lda #1
@bit:
    sta elo
    eor #$FF
    and fm,x
    sta temp
    lda fm,x
    and elo
    ora status
    sta status
    lda temp
    sta fm,x
    lda elo
    asl
    bne :+
    dex
    lda #1
:
    dec count
    bne @bit
    lda fsign
    beq @done
    lda status
    beq @done
    lda #129
    sta aexp
    lda #$80
    sta am
    lda #0
    sta am+1
    sta am+2
    sta am+3
    sta asign
    jmp subtract_values
@done:
    ldx #0
    rts
unsigned_word:
    lda fexp
    beq @zero
    lda fsign
    jne quantity
    lda fexp
    cmp #145
    jcs quantity
    cmp #129
    bcc @zero
    sta count
    lda fm
    sta num+1
    lda fm+1
    sta num
    lda #144
    sec
    sbc count
    tax
    beq @done
@shift:
    lsr num+1
    ror num
    dex
    bne @shift
@done:
    ldx #0
    rts
@zero:
    lda #0
    sta num
    sta num+1
    tax
    rts

; Classify ordered FAC/ARG values without subtracting (and risking overflow).
; Return A=1 greater, 2 equal, 4 less; leave both numeric operands intact.
compare_values:
    lda fexp
    bne @left_nonzero
    lda aexp
    beq @equal
    lda asign
    bmi @greater
    bpl @less
@left_nonzero:
    lda aexp
    bne @both_nonzero
    lda fsign
    bmi @less
    bpl @greater
@both_nonzero:
    lda fsign
    eor asign
    bpl @magnitude
    lda fsign
    bmi @less
    bpl @greater
@magnitude:
    lda fexp
    cmp aexp
    bne @ordered
    ldx #0
@mantissa:
    lda fm,x
    cmp am,x
    bne @ordered
    inx
    cpx #4
    bcc @mantissa
@equal:
    lda #2
    ldx #0
    rts
@ordered:
    lda #1
    bcs :+
    lda #4
:
    ldx fsign
    bpl @return
    eor #5                     ; reverse only the two unequal order bits
@return:
    ldx #0
    rts
@less:
    lda #4
    bne @return
@greater:
    lda #1
    bne @return

logical_and:
    lda #0
    beq logical_binary
logical_or:
    lda #1
logical_binary:
    sta sign
    jsr float_to_signed
    cpx #0
    bne @done
    lda num
    pha
    lda num+1
    pha
    ldx #5
@right:
    lda $69,x
    sta $61,x
    dex
    bpl @right
    jsr float_to_signed
    cpx #0
    bne @failed
    lda sign
    beq @and
    pla
    ora num+1
    sta num+1
    pla
    ora num
    sta num
    jmp signed_to_float
@and:
    pla
    and num+1
    sta num+1
    pla
    and num
    sta num
    jmp signed_to_float
@failed:
    pla                         ; status X survives both real operand pops
    pla
@done:
    rts
logical_not:
    jsr float_to_signed
    cpx #0
    bne :+
    lda num
    eor #$FF
    sta num
    lda num+1
    eor #$FF
    sta num+1
    jmp signed_to_float
:
    rts

zero_value:
    lda #0
    ldx #5
@zero:
    sta fexp,x
    dex
    bpl @zero
    sta $70
    ldx #0
    rts
overflow:
    ldx #1
    rts
divide_zero:
    ldx #2
    rts
quantity:
    ldx #3
    rts

; E is a signed, temporarily wider biased exponent. Underflow is zero; the
; retained magnitude is rounded upwards at an exact half (qualified separately).
round_result:
    lda ehi
    bmi zero_value
    jne overflow
    lda f+4
    bpl @copy
    inc f+3
    bne @copy
    inc f+2
    bne @copy
    inc f+1
    bne @copy
    inc f
    bne @copy
    lda #$80
    sta f
    inc elo
    jeq overflow
@copy:
    lda elo
    jeq zero_value              ; a carry may round the boundary up to minimum
    sta fexp
    ldx #3
@mantissa:
    lda f,x
    sta fm,x
    dex
    bpl @mantissa
    lda sign
    sta fsign
    lda #0
    sta $70
    tax
    rts

subtract_values:
    lda asign
    eor #$FF
    sta asign
add_values:
    lda aexp
    bne @left
    ldx #0
    rts
@left:
    lda fexp
    bne @order
    lda aexp
    sta fexp
    lda asign
    sta fsign
    ldx #3
@arg:
    lda am,x
    sta fm,x
    dex
    bpl @arg
    ldx #0
    rts
@order:
    cmp aexp
    bcs @align
    ldx #5
@swap:
    lda fexp,x
    pha
    lda aexp,x
    sta fexp,x
    pla
    sta aexp,x
    dex
    bpl @swap
@align:
    lda fexp
    sta elo
    lda #0
    sta ehi
    sta f+4
    sta g+4
    ldx #3
@copy:
    lda fm,x
    sta f,x
    lda am,x
    sta g,x
    dex
    bpl @copy
    sec
    lda fexp
    sbc aexp
    tax
    beq @signs
    cmp #40
    bcc @shift
    lda #0
    ldx #4
@discard:
    sta g,x
    dex
    bpl @discard
    jmp @signs
@shift:
    lsr g
    ror g+1
    ror g+2
    ror g+3
    ror g+4
    bcc :+
    lda g+4
    ora #1                      ; jam every discarded nonzero bit into sticky
    sta g+4
:
    dex
    bne @shift
@signs:
    lda fsign
    sta sign
    cmp asign
    bne @difference
    clc
    ldx #4
@sum:
    lda f,x
    adc g,x
    sta f,x
    dex
    bpl @sum
    bcc @rounded
    ror f
    ror f+1
    ror f+2
    ror f+3
    ror f+4
    inc elo
    bne @rounded
    inc ehi
@rounded:
    jmp round_result
@difference:
    ldx #0
@compare:
    lda f,x
    cmp g,x
    bcc @reverse
    bne @subtract
    inx
    cpx #5
    bcc @compare
    jmp zero_value
@reverse:
    lda asign
    sta sign
    ldx #4
@exchange:
    lda f,x
    pha
    lda g,x
    sta f,x
    pla
    sta g,x
    dex
    bpl @exchange
@subtract:
    sec
    ldx #4
@borrow:
    lda f,x
    sbc g,x
    sta f,x
    dex
    bpl @borrow
@normalise:
    lda f
    bmi @rounded
    asl f+4
    rol f+3
    rol f+2
    rol f+1
    rol f
    lda elo
    bne :+
    dec ehi
:
    dec elo
    jmp @normalise

; Exact 32 x 32 -> 64 bit unsigned product, little-endian scratch arrays.
multiply_values:
    lda fexp
    jeq zero_value
    lda aexp
    jeq zero_value
    lda fsign
    eor asign
    sta sign
    clc
    lda fexp
    adc aexp
    sta elo
    lda #0
    adc #0
    sta ehi
    sec
    lda elo
    sbc #129
    sta elo
    lda ehi
    sbc #0
    sta ehi
    ldx #0
    lda #0
@clear:
    sta $50,x
    inx
    cpx #16
    bcc @clear
    ldx #3
    ldy #0
@multiplicand:
    lda fm,x
    sta $58,y
    iny
    dex
    bpl @multiplicand
    lda #32
    sta status
@bit:
    lda am+3
    and #1
    beq @shifts
    clc
    ldx #0
    ldy #8
@add:
    lda $50,x
    adc $58,x
    sta $50,x
    inx
    dey
    bne @add
@shifts:
    clc
    ldx #0
    ldy #8
@left:
    rol $58,x
    inx
    dey
    bne @left
    lsr am
    ror am+1
    ror am+2
    ror am+3
    dec status
    bne @bit
    lda $57
    bmi @upper
    clc
    ldx #0
    ldy #8
@normalise:
    rol $50,x
    inx
    dey
    bne @normalise
    jmp @extract
@upper:
    inc elo
    bne @extract
    inc ehi
@extract:
    lda $50
    ora $51
    ora $52
    beq :+
    lda $53
    ora #1
    sta $53
:
    ; Extract through the CPU stack because the source/destination overlap.
    ldx #3
@save:
    lda $50,x
    pha
    inx
    cpx #8
    bcc @save
    ldx #0
@take:
    pla
    sta f,x
    inx
    cpx #5
    bcc @take
    jmp round_result

; Restoring division retains 40 quotient bits and jams the exact remainder.
divide_values:
    lda aexp
    jeq divide_zero
    lda fexp
    jeq zero_value
    lda fsign
    eor asign
    sta sign
    sec
    lda fexp
    sbc aexp
    sta elo
    lda #0
    sbc #0
    sta ehi
    clc
    lda elo
    adc #129
    sta elo
    lda ehi
    adc #0
    sta ehi
    lda #0
    sta $50
    sta $55
    ldx #3
@copy:
    lda fm,x
    sta $51,x
    lda am,x
    sta $56,x
    dex
    bpl @copy
    lda #0
    ldx #4
@quotient:
    sta $5A,x
    dex
    bpl @quotient
    jsr compare_remainder
    bcs @ready
    jsr shift_remainder
    lda elo
    bne :+
    dec ehi
:
    dec elo
@ready:
    lda #40
    sta count
@bit:
    jsr compare_remainder
    bcc @zero
    sec
    ldx #4
@subtract:
    lda $50,x
    sbc $55,x
    sta $50,x
    dex
    bpl @subtract
    sec
@zero:
    rol $5E
    rol $5D
    rol $5C
    rol $5B
    rol $5A
    dec count
    beq @result
    jsr shift_remainder
    jmp @bit
@result:
    lda $50
    ora $51
    ora $52
    ora $53
    ora $54
    beq :+
    lda $5E
    ora #1
    sta $5E
:
    ldx #4
@extract:
    lda $5A,x
    sta f,x
    dex
    bpl @extract
    jmp round_result
compare_remainder:
    ldx #0
@byte:
    lda $50,x
    cmp $55,x
    bne @done
    inx
    cpx #5
    bcc @byte
    sec
@done:
    rts
shift_remainder:
    asl $54
    rol $53
    rol $52
    rol $51
    rol $50
    rts

signed_to_float:
    lda #0
    sta sign
    lda num+1
    bpl @positive
    lda #$FF
    sta sign
    sec
    lda #0
    sbc num
    sta num
    lda #0
    sbc num+1
    sta num+1
@positive:
    lda num
    ora num+1
    jeq zero_value
    lda num+1
    sta f
    lda num
    sta f+1
    lda #0
    sta f+2
    sta f+3
    sta f+4
    sta ehi
    lda #144
    sta elo
@normalise:
    lda f
    bmi @done
    asl f+4
    rol f+3
    rol f+2
    rol f+1
    rol f
    dec elo
    jmp @normalise
@done:
    jmp round_result
float_to_signed:
    lda fexp
    jeq @zero
    cmp #129
    jcc @small
    cmp #145
    jcs quantity
    sta count
    lda fm+2
    ora fm+3
    sta status                  ; fractional sticky bits for negative floor
    lda fm
    sta num+1
    lda fm+1
    sta num
    lda #144
    sec
    sbc count
    tax
    beq @range
@right:
    lsr num+1
    ror num
    bcc :+
    lda #1
    sta status
:
    dex
    bne @right
@range:
    lda num+1
    bpl @sign
    ldx fsign
    jeq quantity
    cmp #$80
    jne quantity
    lda num
    jne quantity
    lda fm+2
    ora fm+3
    jne quantity                 ; negative values below -32768 are not admissible
@sign:
    lda fsign
    beq @success
    lda status
    beq @negate
    inc num
    bne @negate
    inc num+1
@negate:
    sec
    lda #0
    sbc num
    sta num
    lda #0
    sbc num+1
    sta num+1
@success:
    ldx #0
    rts
@small:
    lda fsign
    beq @zero
    lda #$FF
    sta num
    sta num+1
    ldx #0
    rts
@zero:
    lda #0
    sta num
    sta num+1
    tax
    rts

; Decimal conversion uses two bounded little-endian integers on the real guest
; stack. N/D * 2^E is exact until the single final 32-bit rounding step. Eighty
; mantissa input digits and the supported exponent range fit forty bytes.
; Output needs only thirty-two bytes per integer, leaving ten stack digit cells.
npt = $22
dpt = $26
base = $4B
saved_sp = $4C
width = $51
loops = $52
digits = $53
exp_large = $55
fraction_count = $56
significant = $57
point = $58
exp_value = $59
exp_negative = $5C
digit_count = $5D
out_index = $5E
position = $5F

big_enter:
    lda #80
    ldx width
    cpx #12
    bne :+
    lda #24
:
    sta $4A
    tsx
    txa
    sec
    sbc $4A
    bcc @full
    cmp #80                     ; retain IRQ/callback room and low-stack output
    bcs @space
@full:
    ldx #4
    clc
    rts
@space:
    pla                         ; move only this helper's real JSR return
    sta $28
    pla
    sta $29
    tsx
    stx saved_sp
    txa
    sec
    sbc $4A
    tax
    txs
    inx
    stx base
    stx npt
    txa
    clc
    adc width
    sta dpt
    lda #1
    sta npt+1
    sta dpt+1
    lda #0
    ldy $4A
@clear:
    sta $0100,x
    inx
    dey
    bne @clear
    lda base
    clc
    adc #64
    sta digits
    lda $29
    pha
    lda $28
    pha
    ldx #0
    sec
    rts
big_leave:
    stx status
    ldx saved_sp
    txs
    ldx status
    rts
big_compare:
    ldy width
    dey
@byte:
    lda (npt),y
    cmp (dpt),y
    bne @return
    dey
    bpl @byte
    sec
@return:
    rts
big_subtract:
    ldy #0
    ldx width
    sec
@byte:
    lda (npt),y
    sbc (dpt),y
    sta (npt),y
    iny
    dex
    bne @byte
    rts
big_shift_n:
    ldx width
    ldy #0
    clc
@byte:
    lda (npt),y
    rol a
    sta (npt),y
    iny
    dex
    bne @byte
    rts
big_shift_d:
    ldx width
    ldy #0
    clc
@byte:
    lda (dpt),y
    rol a
    sta (dpt),y
    iny
    dex
    bne @byte
    rts
big_right_d:
    ldy width
    dey
    clc
@byte:
    lda (dpt),y
    ror a
    sta (dpt),y
    dey
    bpl @byte
    rts
big_swap_pointers:
    lda npt
    pha
    lda dpt
    sta npt
    pla
    sta dpt
    rts
big_multiply_n5:
    lda #0
    sta $29
    ldy #0
@byte:
    lda #0
    sta $2A
    lda (npt),y
    sta $28
    asl a
    rol $2A
    asl a
    rol $2A
    clc
    adc $28
    bcc :+
    inc $2A
:
    clc
    adc $29
    bcc :+
    inc $2A
:
    sta (npt),y
    lda $2A
    sta $29
    iny
    cpy width
    bcc @byte
    rts
big_multiply_n10:
    jsr big_multiply_n5
    jmp big_shift_n
big_multiply_d5:
    jsr big_swap_pointers
    jsr big_multiply_n5
    jmp big_swap_pointers
big_multiply_d10:
    jsr big_swap_pointers
    jsr big_multiply_n10
    jmp big_swap_pointers
math_peek:
    lda $10
    bpl @byte
    lda $7A
    cmp $11
    bne @byte
    lda $7B
    cmp $12
    bne @byte
    lda #0
    rts
@byte:
    ldy #0
    lda ($7A),y
    rts
math_consume:
    inc $7A
    bne :+
    inc $7B
:
    jmp math_peek

; Ordinary short decimals need neither an eighty-byte reservation nor a weaker
; IRQ margin. Exponents/long mantissas retain the full exact-conversion buffers.
decimal_width:
    ldy #0
    ldx #0
@byte:
    lda ($7A),y
    cmp #'.'
    beq @next
    cmp #'0'
    bcc @end
    cmp #('9'+1)
    bcs @end
    inx
    cpx #10
    bcs @large
@next:
    iny
    cpy #12
    bcc @byte
@large:
    lda #40
    sta width
    rts
@end:
    cmp #'E'
    beq @large
    lda #12
    sta width
    rts
parse_number:
    lda #0
    sta $10
    jsr decimal_width
    jmp decimal_entry
parse_string_number:
    lda #$80
    sta $10
    lda #40
    sta width
decimal_entry:
    jsr big_enter
    bcs :+
    rts
:
    lda #0
    sta exp_large
    sta fraction_count
    sta significant
    sta point
    sta exp_negative
    sta exp_value
    sta exp_value+1
    sta digit_count
    jsr math_peek
@next:
    cmp #'0'
    jcc @not_digit
    cmp #('9'+1)
    jcs @not_digit
    sec
    sbc #'0'
    sta temp
    inc digit_count
    lda digit_count
    cmp #81
    jcs decimal_unimplemented
    lda point
    beq :+
    inc fraction_count
:
    lda temp
    bne @significant
    lda significant
    beq @accumulate
@significant:
    inc significant
@accumulate:
    jsr big_multiply_n10
    ldy #0
    lda (npt),y
    clc
    adc temp
    sta (npt),y
    bcc @consume
    ldx width
    dex
    iny
@carry:
    lda (npt),y
    adc #0
    sta (npt),y
    bcc @consume
    iny
    dex
    bne @carry
@consume:
    jsr math_consume
    jmp @next
@not_digit:
    cmp #'.'
    bne @exponent
    lda point
    beq @point
    lda $10
    bpl :+
    jmp @magnitude
:
    jmp decimal_syntax
@point:
    inc point
    jsr math_consume
    jmp @next
@exponent:
    ldx digit_count
    jeq decimal_syntax
    cmp #'E'
    jne @literal_end
    jsr math_consume
    cmp #$AA
    beq @positive
    cmp #'+'
    beq @positive
    cmp #$AB
    beq @negative
    cmp #'-'
    bne @require_digit
@negative:
    inc exp_negative
@positive:
    jsr math_consume
@require_digit:
    cmp #'0'
    jcc decimal_syntax
    cmp #('9'+1)
    jcs decimal_syntax
@exp_digit:
    sec
    sbc #'0'
    sta temp
    lda exp_large
    bne @exp_consume
    lda exp_value
    sta num
    lda exp_value+1
    sta num+1
    asl exp_value
    rol exp_value+1
    asl exp_value
    rol exp_value+1
    clc
    lda exp_value
    adc num
    sta exp_value
    lda exp_value+1
    adc num+1
    sta exp_value+1
    asl exp_value
    rol exp_value+1
    clc
    lda exp_value
    adc temp
    sta exp_value
    lda exp_value+1
    adc #0
    sta exp_value+1
    cmp #4
    bcs @huge
    bne @exp_consume
@huge:
    inc exp_large
@exp_consume:
    jsr math_consume
    cmp #'0'
    bcc @literal_end
    cmp #('9'+1)
    bcc @exp_digit
@literal_end:
    ldx $10
    bmi @magnitude
    cmp #'.'
    jeq decimal_syntax
    cmp #'A'
    bcc @magnitude
    cmp #('Z'+1)
    jcc decimal_syntax
@magnitude:
    lda significant
    jeq decimal_zero
    lda exp_large
    beq @scale
    lda exp_negative
    jne decimal_zero
    jmp decimal_overflow
@scale:
    lda exp_value
    sta elo
    lda exp_value+1
    sta ehi
    lda exp_negative
    beq @fraction
    sec
    lda #0
    sbc elo
    sta elo
    lda #0
    sbc ehi
    sta ehi
@fraction:
    sec
    lda elo
    sbc fraction_count
    sta elo
    lda ehi
    sbc #0
    sta ehi
    clc
    lda elo
    adc significant
    sta exp_value
    lda ehi
    adc #0
    sta exp_value+1
    bmi @small
    jne decimal_overflow
    lda exp_value
    cmp #40
    jcs decimal_overflow
    jmp @power
@small:
    cmp #$FF
    jne decimal_zero
    lda exp_value
    cmp #$D9                    ; decimal order below -39 is certainly zero
    jcc decimal_zero
@power:
    ldy #0
    lda #1
    sta (dpt),y
    lda ehi
    bmi @denominator
    lda elo
    sta loops
    beq @normalise
@positive_power:
    jsr big_multiply_n5
    dec loops
    bne @positive_power
    jmp @normalise
@denominator:
    sec
    lda #0
    sbc elo
    sta loops
@negative_power:
    jsr big_multiply_d5
    dec loops
    bne @negative_power
@normalise:
    jsr big_compare
    bcs @above
    jsr big_shift_n
    lda elo
    bne :+
    dec ehi
:
    dec elo
    jmp @normalise
@above:
    jsr big_shift_d
    inc elo
    bne :+
    inc ehi
:
    jsr big_compare
    bcs @above
    jsr big_right_d
    lda elo
    bne :+
    dec ehi
:
    dec elo
    lda #0
    ldx #4
@quotient_zero:
    sta $61,x
    dex
    bpl @quotient_zero
    lda #40
    sta loops
@bit:
    jsr big_compare
    bcc @quotient_bit
    jsr big_subtract
    sec
@quotient_bit:
    rol $65
    rol $64
    rol $63
    rol $62
    rol $61
    dec loops
    beq @rounded
    jsr big_shift_n
    jmp @bit
@rounded:
    lda #0
    ldy #0
    ldx width
@sticky:
    ora (npt),y
    iny
    dex
    bne @sticky
    cmp #0
    beq :+
    lda $65
    ora #1
    sta $65
:
    lda #0
    sta sign
    clc
    lda elo
    adc #129
    sta elo
    lda ehi
    adc #0
    sta ehi
    ldx #4
@extract:
    lda $61,x
    sta f,x
    dex
    bpl @extract
    jsr round_result
    jmp big_leave
decimal_zero:
    jsr zero_value
    jmp big_leave
decimal_overflow:
    ldx #1
    jmp big_leave
decimal_syntax:
    ldx #6
    jmp big_leave
decimal_unimplemented:
    ldx #5
    jmp big_leave

; Produce the PRINT representation into bounded low guest stack workspace.
; Page-zero code invokes CHROUT after this gate returns, so user callbacks never
; inherit the private numeric page. FAC itself is preserved by formatting.
format_number:
    lda #32
    sta width
    jsr big_enter
    bcs :+
    rts
:
    lda #0
    sta out_index
    lda fsign
    sta f
    lda fexp
    jne @value
    lda #' '
    jsr number_character
    lda #'0'
    jsr number_character
    jmp number_end
@value:
    lda #0
    sta $4F
    ldy #0
    ldx #3
@mantissa:
    lda fm,x
    sta (npt),y
    iny
    dex
    bpl @mantissa
    ldy #0
    lda #1
    sta (dpt),y
    sec
    lda fexp
    sbc #160
    sta elo
    lda #0
    sbc #0
    sta ehi
    bmi @small
    lda elo
    sta loops
    beq @decimal
@left:
    jsr big_shift_n
    dec loops
    bne @left
    jmp @decimal
@small:
    sec
    lda #0
    sbc elo
    sta loops
@right:
    jsr big_shift_d
    dec loops
    bne @right
@decimal:
    jsr big_compare
    bcs @ten
    jsr big_multiply_n10
    dec $4F
    jmp @decimal
@ten:
    jsr big_multiply_d10
    inc $4F
    jsr big_compare
    bcs @ten
    jsr big_multiply_n10
    dec $4F
    lda #0
    sta digit_count
@digit:
    lda #0
    sta temp
@subtract:
    jsr big_compare
    bcc @save_digit
    jsr big_subtract
    inc temp
    jmp @subtract
@save_digit:
    clc
    lda digits
    adc digit_count
    tax
    lda temp
    sta $0100,x
    inc digit_count
    lda digit_count
    cmp #10
    bcs @round
    jsr big_multiply_n10
    jmp @digit
@round:
    clc
    lda digits
    adc #9
    tax
    lda $0100,x
    cmp #5
    bcc @trim
    ldy #9
@carry:
    dex
    inc $0100,x
    lda $0100,x
    cmp #10
    bcc @trim
    lda #0
    sta $0100,x
    dey
    bne @carry
    ldx digits
    lda #1
    sta $0100,x
    inc $4F
@trim:
    lda #9
    sta digit_count
@trailing:
    clc
    lda digits
    adc digit_count
    tax
    dex
    lda $0100,x
    bne @sign
    dec digit_count
    lda digit_count
    cmp #1
    bne @trailing
@sign:
    lda f
    bmi @negative
    lda #' '
    jmp @first
@negative:
    lda #'-'
@first:
    jsr number_character
    lda $4F
    bmi @fractional
    cmp #9
    jcs @scientific
    clc
    adc #1
    sta loops                   ; number of positions before the decimal point
    lda #0
    sta position
@plain:
    lda position
    cmp loops
    bne @plain_digit
    cmp digit_count
    jcs number_end
    lda #'.'
    jsr number_character
@plain_digit:
    lda position
    cmp digit_count
    bcc @significant_digit
    cmp loops
    jcs number_end
    lda #'0'
    jmp @plain_emit
@significant_digit:
    clc
    adc digits
    tax
    lda $0100,x
    ora #'0'
@plain_emit:
    jsr number_character
    inc position
    jmp @plain
@fractional:
    cmp #$FE
    jcc @scientific
    lda #'.'
    jsr number_character
    lda $4F
    cmp #$FE
    bne @fraction_digits
    lda #'0'
    jsr number_character
@fraction_digits:
    lda #0
    sta position
@next:
    lda position
    cmp digit_count
    jcs number_end
    clc
    adc digits
    tax
    lda $0100,x
    ora #'0'
    jsr number_character
    inc position
    jmp @next
@scientific:
    ldx digits
    lda $0100,x
    ora #'0'
    jsr number_character
    lda #1
    sta position
    lda digit_count
    cmp #1
    beq @exponent
    lda #'.'
    jsr number_character
@fraction:
    lda position
    cmp digit_count
    bcs @exponent
    clc
    adc digits
    tax
    lda $0100,x
    ora #'0'
    jsr number_character
    inc position
    jmp @fraction
@exponent:
    lda #'E'
    jsr number_character
    lda $4F
    bmi @negative_exp
    sta temp
    lda #'+'
    jmp @exp_sign
@negative_exp:
    sec
    lda #0
    sbc $4F
    sta temp
    lda #'-'
@exp_sign:
    jsr number_character
    ldy #0
@tens:
    lda temp
    cmp #10
    bcc @exp_digits
    sec
    sbc #10
    sta temp
    iny
    bne @tens
@exp_digits:
    tya
    ora #'0'
    jsr number_character
    lda temp
    ora #'0'
    jsr number_character
number_end:
    lda #' '
    jsr number_character
    ldx out_index
    lda #0
    sta $0100,x
    ldx #0
    jmp big_leave
number_character:
    ldx out_index
    sta $0100,x
    inc out_index
    rts

.include "square-root.inc"

.include "transcendentals.inc"

.include "channels.inc"
