; Original guest range reductions and convergent series. Intermediate values
; are five six-byte FAC slots on the real CPU stack, with a saved frame index.
; Constants are mathematical data generated independently as exact rationals.
trans_enter:
    tsx
    cpx #$A0
    bcs @room
    pla
    pla
    ldx #4
    rts
@room:
    pla
    sta $26
    pla
    sta $27
    lda $0C
    pha
    tsx
    txa
    sec
    sbc #32
    tax
    txs
    stx $0C
    lda $27
    pha
    lda $26
    pha
    rts
trans_leave:
    stx $0B
    lda $0C
    clc
    adc #32
    tax
    txs
    pla
    sta $0C
    ldx $0B
    rts
trans_done:
    ldx #0
    jmp trans_leave
trans_calculate:
    jsr math_entry
    cpx #0
    jne trans_leave
    rts
trans_fac:
    clc
    adc $0C
    tax
    ldy #0
@byte:
    lda $0101,x
    sta fexp,y
    inx
    iny
    cpy #6
    bcc @byte
    rts
trans_arg:
    clc
    adc $0C
    tax
    ldy #0
@byte:
    lda $0101,x
    sta aexp,y
    inx
    iny
    cpy #6
    bcc @byte
    rts
trans_store:
    clc
    adc $0C
    tax
    ldy #0
@byte:
    lda fexp,y
    sta $0101,x
    inx
    iny
    cpy #6
    bcc @byte
    rts
constant_arg:
    asl
    sta num
    asl
    clc
    adc num
    tax
    ldy #0
@byte:
    lda math_constants,x
    sta aexp,y
    inx
    iny
    cpy #6
    bcc @byte
    rts
integer_arg:
    sta am
    lda #0
    sta am+1
    sta am+2
    sta am+3
    sta asign
    lda am
    beq @zero
    lda #136
    sta aexp
@normalise:
    lda am
    bmi @done
    asl am
    dec aexp
    jmp @normalise
@zero:
    sta aexp
@done:
    rts
arg_to_fac:
    ldx #5
@byte:
    lda aexp,x
    sta fexp,x
    dex
    bpl @byte
    rts
trans_one:
    lda #0
    jsr constant_arg
    jmp arg_to_fac

logarithm_value:
    lda fexp
    jeq quantity
    lda fsign
    jne quantity
    jsr trans_enter
    lda #0
    jsr trans_store
    lda fexp
    sec
    sbc #129
    sta num
    lda #0
    sbc #0
    sta num+1
    jsr signed_to_float
    lda #24
    jsr trans_store
    lda #0
    jsr trans_fac
    lda #129
    sta fexp
    lda #0
    jsr trans_store
    lda #0
    jsr constant_arg
    lda #0
    jsr trans_calculate
    lda #6
    jsr trans_store
    lda #0
    jsr trans_fac
    lda #0
    jsr constant_arg
    lda #1
    jsr trans_calculate
    lda #6
    jsr trans_arg
    lda #3
    jsr trans_calculate
    lda #0
    jsr trans_store
    lda #12
    jsr trans_store
    lda #18
    jsr trans_store
    lda #0
    jsr trans_arg
    lda #2
    jsr trans_calculate
    lda #6
    jsr trans_store
    lda #3
    sta $0B
@term:
    lda #12
    jsr trans_fac
    lda #6
    jsr trans_arg
    lda #2
    jsr trans_calculate
    lda #12
    jsr trans_store
    lda $0B
    jsr integer_arg
    lda #3
    jsr trans_calculate
    lda #18
    jsr trans_arg
    lda #0
    jsr trans_calculate
    lda #18
    jsr trans_store
    inc $0B
    inc $0B
    lda $0B
    cmp #49
    bcc @term
    lda fexp
    beq :+
    inc fexp                    ; 2*atanh(z), with canonical zero preserved
:
    lda #18
    jsr trans_store
    lda #24
    jsr trans_fac
    lda #2
    jsr constant_arg
    lda #2
    jsr trans_calculate
    lda #18
    jsr trans_arg
    lda #0
    jsr trans_calculate
    jmp trans_done

exponential_value:
    lda fexp
    cmp #136
    bcc @range
    lda fsign
    jne zero_value
    jmp overflow
@range:
    jsr trans_enter
    lda #0
    jsr trans_store
    lda #2
    jsr constant_arg
    lda #3
    jsr trans_calculate
    lda #7
    jsr trans_calculate
    ldx $0C
    lda num
    sta $011F,x
    lda num+1
    sta $0120,x
    bmi @lower
    lda num
    cmp #128
    jcs @overflow
    jmp @reduce
@lower:
    lda num
    cmp #127                    ; -129 is the final possibly nonzero interval
    jcc @zero
@reduce:
    lda #6
    jsr trans_calculate
    lda #2
    jsr constant_arg
    lda #2
    jsr trans_calculate
    lda #6
    jsr trans_store
    lda #0
    jsr trans_fac
    lda #6
    jsr trans_arg
    lda #1
    jsr trans_calculate
    lda #0
    jsr trans_store
    jsr trans_one
    lda #12
    jsr trans_store
    lda #18
    jsr trans_store
    lda #1
    sta $0B
@term:
    lda #12
    jsr trans_fac
    lda #0
    jsr trans_arg
    lda #2
    jsr trans_calculate
    lda $0B
    jsr integer_arg
    lda #3
    jsr trans_calculate
    lda #12
    jsr trans_store
    lda #18
    jsr trans_arg
    lda #0
    jsr trans_calculate
    lda #18
    jsr trans_store
    inc $0B
    lda $0B
    cmp #20
    bcc @term
    ldx $0C
    clc
    lda fexp
    adc $011F,x
    sta fexp
    lda #0
    adc $0120,x
    bmi @zero
    bne @overflow
    lda fexp
    beq @zero
    jmp trans_done
@zero:
    jsr zero_value
    jmp trans_done
@overflow:
    ldx #1
    jmp trans_leave

cosine_value:
    lda #0
    beq trig_value
sine_value:
    lda #1
trig_value:
    sta num
    jsr trans_enter
    ldx $0C
    lda num
    sta $0120,x
    lda #0
    jsr trans_store
    lda #6
    jsr constant_arg
    lda #3
    jsr trans_calculate
    lda #14
    jsr trans_calculate
    lda #6
    jsr constant_arg
    lda #2
    jsr trans_calculate
    lda #6
    jsr trans_store
    lda #0
    jsr trans_fac
    lda #6
    jsr trans_arg
    lda #1
    jsr trans_calculate
    lda #0
    jsr trans_store
    lda #3
    jsr constant_arg
    lda #8
    jsr trans_calculate
    cmp #1
    bne @reduced
    lda #6
    jsr constant_arg
    lda #1
    jsr trans_calculate
    lda #0
    jsr trans_store
@reduced:
    lda #0
    jsr trans_arg
    lda #2
    jsr trans_calculate
    lda #6
    jsr trans_store
    ldx $0C
    lda $0120,x
    beq @cos
    lda #0
    jsr trans_fac
    lda #3
    jmp @initial
@cos:
    jsr trans_one
    lda #2
@initial:
    sta $0B
    lda #12
    jsr trans_store
    lda #18
    jsr trans_store
@term:
    lda #12
    jsr trans_fac
    lda fsign
    eor #$FF
    sta fsign
    lda #6
    jsr trans_arg
    lda #2
    jsr trans_calculate
    lda $0B
    jsr integer_arg
    lda #3
    jsr trans_calculate
    lda $0B
    sec
    sbc #1
    jsr integer_arg
    lda #3
    jsr trans_calculate
    lda #12
    jsr trans_store
    lda #18
    jsr trans_arg
    lda #0
    jsr trans_calculate
    lda #18
    jsr trans_store
    inc $0B
    inc $0B
    lda $0B
    cmp #35
    bcc @term
    jmp trans_done

tangent_value:
    jsr trans_enter
    lda #0
    jsr trans_store
    lda #22
    jsr trans_calculate
    lda #6
    jsr trans_store
    lda #0
    jsr trans_fac
    lda #21
    jsr trans_calculate
    lda #12
    jsr trans_store
    lda #6
    jsr trans_fac
    lda #12
    jsr trans_arg
    lda #3
    jsr trans_calculate
    jmp trans_done

arctangent_value:
    jsr trans_enter
    ldx $0C
    lda fsign
    and #$80
    sta $0120,x
    lda #0
    sta fsign
    lda #0
    jsr constant_arg
    lda #8
    jsr trans_calculate
    cmp #1
    bne @unit
    lda #0
    jsr trans_store
    jsr trans_one
    lda #0
    jsr trans_arg
    lda #3
    jsr trans_calculate
    ldx $0C
    inc $0120,x
@unit:
    lda #0
    jsr trans_store
    ; Reducing all x>.5 keeps the alternating series strictly inside .5.
    lda #1
    jsr integer_arg
    dec aexp
    lda #8
    jsr trans_calculate
    cmp #1
    bne @series
    lda #0
    jsr constant_arg
    lda #0
    jsr trans_calculate
    lda #6
    jsr trans_store
    lda #0
    jsr trans_fac
    lda #0
    jsr constant_arg
    lda #1
    jsr trans_calculate
    lda #6
    jsr trans_arg
    lda #3
    jsr trans_calculate
    lda #0
    jsr trans_store
    ldx $0C
    lda $0120,x
    ora #2
    sta $0120,x
@series:
    lda #0
    jsr trans_fac
    lda #12
    jsr trans_store
    lda #18
    jsr trans_store
    lda #0
    jsr trans_arg
    lda #2
    jsr trans_calculate
    lda #6
    jsr trans_store
    lda #3
    sta $0B
@term:
    lda #12
    jsr trans_fac
    lda fsign
    eor #$FF
    sta fsign
    lda #6
    jsr trans_arg
    lda #2
    jsr trans_calculate
    lda #12
    jsr trans_store
    lda $0B
    jsr integer_arg
    lda #3
    jsr trans_calculate
    lda #18
    jsr trans_arg
    lda #0
    jsr trans_calculate
    lda #18
    jsr trans_store
    inc $0B
    inc $0B
    lda $0B
    cmp #49
    bcc @term
    ldx $0C
    lda $0120,x
    and #2
    beq @inverse
    lda #5
    jsr constant_arg
    lda #0
    jsr trans_calculate
@inverse:
    ldx $0C
    lda $0120,x
    and #1
    beq @sign
    lda #18
    jsr trans_store
    lda #4
    jsr constant_arg
    jsr arg_to_fac
    lda #18
    jsr trans_arg
    lda #1
    jsr trans_calculate
@sign:
    ldx $0C
    lda $0120,x
    and #$80
    beq @done
    lda fsign
    eor #$FF
    sta fsign
@done:
    jmp trans_done

power_value:
    jsr trans_enter
    lda #0
    jsr trans_store
    jsr arg_to_fac
    lda #6
    jsr trans_store
    lda fexp
    bne @nonzero_exponent
    jsr trans_one
    jmp trans_done
@nonzero_exponent:
    lda #0
    jsr trans_fac
    lda fexp
    bne @base
    lda #6
    jsr trans_fac
    lda fsign
    bne @divide
    jsr zero_value
    jmp trans_done
@divide:
    ldx #2
    jmp trans_leave
@base:
    jsr power_integer
    lda #0
    jsr trans_fac
    ldx $0C
    lda #0
    sta $0120,x
    lda fsign
    beq @positive
    lda #6
    jsr trans_fac
    lda #14
    jsr trans_calculate
    lda #6
    jsr trans_arg
    lda #8
    jsr trans_calculate
    cmp #2
    bne @quantity
    lda fexp
    cmp #161
    bcs @positive
    lda #160
    sec
    sbc fexp
    sta count
    ldx #3
@mantissa:
    lda fm,x
    sta f,x
    dex
    bpl @mantissa
@parity:
    lda count
    beq @odd
    lsr f
    ror f+1
    ror f+2
    ror f+3
    dec count
    jmp @parity
@odd:
    lda f+3
    and #1
    beq @positive
    ldx $0C
    lda #$80
    sta $0120,x
@positive:
    lda #0
    jsr trans_fac
    lda #0
    sta fsign
    lda #19
    jsr trans_calculate
    lda #6
    jsr trans_arg
    lda #2
    jsr trans_calculate
    lda #20
    jsr trans_calculate
    ldx $0C
    lda $0120,x
    beq @done
    lda #$FF
    sta fsign
@done:
    jmp trans_done
@quantity:
    ldx #3
    jmp trans_leave

; Exact integral powers avoid LOG/EXP cancellation and retain binary32 rounding
; at each ordinary multiply/divide. Larger/non-integral exponents use the
; separately bounded general route above.
power_integer:
    lda #6
    jsr trans_fac
    lda fexp
    cmp #145
    jcs @general
    lda #14
    jsr trans_calculate
    lda #6
    jsr trans_arg
    lda #8
    jsr trans_calculate
    cmp #2
    jne @general
    jsr float_to_signed
    cpx #0
    jne @general
    ldx $0C
    lda #0
    sta $0120,x
    lda num+1
    bpl @positive
    inc $0120,x
    sec
    lda #0
    sbc num
    sta num
    lda #0
    sbc num+1
    sta num+1
@positive:
    lda num
    sta $0119,x
    lda num+1
    sta $011A,x
    lda $0120,x
    beq @initial
    jsr trans_one
    lda #0
    jsr trans_arg
    lda #3
    jsr trans_calculate
    lda #0
    jsr trans_store
@initial:
    jsr trans_one
    lda #12
    jsr trans_store
@bit:
    ldx $0C
    lda $0119,x
    and #1
    beq @shift
    lda #12
    jsr trans_fac
    lda #0
    jsr trans_arg
    lda #2
    jsr trans_calculate
    lda #12
    jsr trans_store
@shift:
    ldx $0C
    lsr $011A,x
    ror $0119,x
    lda $0119,x
    ora $011A,x
    beq @result
    lda #0
    jsr trans_fac
    lda #0
    jsr trans_arg
    lda #2
    jsr trans_calculate
    lda #0
    jsr trans_store
    jmp @bit
@result:
    lda #12
    jsr trans_fac
    jmp trans_done
@general:
    rts

; Original deterministic xorshift state in the published random-seed workspace.
; RND(0) samples guest clock/timer registers; negative arguments reseed from FAC.
random_value:
    lda fexp
    beq @clock
    lda fsign
    beq @advance
    ldx #3
@seed:
    lda fm,x
    sta $8C,x
    dex
    bpl @seed
    lda fexp
    eor $8F
    sta $8F
    jmp @advance
@clock:
    lda $DC04
    sta $8C
    lda $DC05
    sta $8D
    lda $A1
    sta $8E
    lda $A2
    sta $8F
@advance:
    lda $8C
    ora $8D
    ora $8E
    ora $8F
    bne @nonzero
    lda #$65
    sta $8C
    lda #$10
    sta $8F
@nonzero:
    ldx #3
@copy:
    lda $8C,x
    sta fm,x
    dex
    bpl @copy
    ldy #13
@left13:
    asl fm+3
    rol fm+2
    rol fm+1
    rol fm
    dey
    bne @left13
    jsr random_mix
    ldy #17
@right17:
    lsr fm
    ror fm+1
    ror fm+2
    ror fm+3
    dey
    bne @right17
    jsr random_mix
    ldy #5
@left5:
    asl fm+3
    rol fm+2
    rol fm+1
    rol fm
    dey
    bne @left5
    jsr random_mix
    lda #128
    sta fexp
    lda #0
    sta fsign
@normalise:
    lda fm
    bmi @done
    asl fm+3
    rol fm+2
    rol fm+1
    rol fm
    dec fexp
    bne @normalise
@done:
    lda fexp
    sta $8B
    ldx #0
    rts
random_mix:
    ldx #3
@byte:
    lda fm,x
    eor $8C,x
    sta $8C,x
    sta fm,x
    dex
    bpl @byte
    rts

pi_value:
    lda #3
    jsr constant_arg
    jsr arg_to_fac
    ldx #0
    rts
.include "math-constants.inc"
