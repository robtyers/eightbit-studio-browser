.segment "CODE"
; Original FOR records occupy the real CPU stack: tag, two packed five-byte
; values, scalar pointer and the six-byte public programme/text context. No
; permanent RAM, host state or private page continuation contains loop values.
for_statement:
    jsr variable_name
    lda $45
    bpl :+
    jmp syntax_error            ; BASIC V2 FOR counters must be floating scalars
:
    jsr variable_lookup
    lda $47
    pha
    lda $48
    pha
    jsr skip_spaces
    cmp #$B2
    jne syntax_error
    jsr consume
    jsr expression
    pla
    sta $48
    pla
    sta $47
    jsr scalar_type
    jsr variable_store
    jsr skip_spaces
    cmp #$A4
    jne syntax_error
    jsr consume
    ; Re-entering FOR for an active counter replaces it and younger FORs,
    ; stopping at the current subroutine boundary.
    tsx
@old:
    cpx #$F8
    bcs @room
    lda $0101,x
    cmp #$81
    bne @room
    cpx #$E6
    jcs next_error
    lda $010C,x
    cmp $48
    bne @older
    lda $010D,x
    cmp $47
    beq @replace
@older:
    txa
    clc
    adc #19
    tax
    jmp @old
@replace:
    txa
    clc
    adc #19
    tax
    txs
@room:
    tsx
    cpx #$D0
    jcc memory_error
    lda $3A
    pha
    lda $39
    pha
    lda cur+1
    pha
    lda cur
    pha
    lda #0                      ; filled with the parsed body position below
    pha
    pha
    lda $47
    pha
    lda $48
    pha
    jsr expression
    jsr require_numeric
    jsr save_packed_fac
    jsr skip_spaces
    cmp #$A9
    bne @default
    jsr consume
    jsr expression
    jsr require_numeric
    jmp @step
@default:
    lda #1
    jsr byte_number
@step:
    jsr save_packed_fac
    jsr check_end
    lda #$81
    pha
    tsx
    lda txt
    sta $010E,x
    lda txt+1
    sta $010F,x
    jmp dispatch

save_packed_fac:
    pla
    sta $26
    pla
    sta $27
    lda $61
    pha
    lda $62
    and #$7F
    sta delta
    lda $66
    and #$80
    ora delta
    pha
    lda $63
    pha
    lda $64
    pha
    lda $65
    pha
    lda $27
    pha
    lda $26
    pha
    rts

next_statement:
    jsr skip_spaces
    cmp #'A'
    bcc @implicit
    cmp #('Z'+1)
    bcs @implicit
    jsr variable_name
    jsr variable_lookup
    jmp @search
@implicit:
    lda #0
    sta $47
    sta $48
@search:
    tsx
@frame:
    cpx #$F8
    jcs next_error
    lda $0101,x
    cmp #$81
    jne next_error
    cpx #$E6
    jcs next_error
    lda $48
    beq @found
    cmp $010C,x
    bne @older
    lda $47
    cmp $010D,x
    beq @found
@older:
    txa
    clc
    adc #19
    tax
    jmp @frame
@found:
    txs                         ; discard younger loops when NEXT names an outer one
    lda $010C,x
    sta $48
    lda $010D,x
    sta $47
    jsr scalar_type
    jsr variable_load
    tsx
    jsr loop_arg
    lda #0
    jsr numeric_call
    jsr variable_store
    tsx
    txa
    clc
    adc #5
    tax
    jsr loop_arg
    lda #8
    jsr numeric_call
    sta delta
    tsx
    lda $0105,x
    bmi @negative
    lda delta
    and #1
    jmp @test
@negative:
    lda delta
    and #4
@test:
    bne @finished
    lda $010E,x
    sta txt
    lda $010F,x
    sta txt+1
    lda $0110,x
    sta $45
    lda $0111,x
    sta $46
    lda $0112,x
    sta $39
    lda $0113,x
    sta $3A
    lda #$81
    sta mode
    jmp resume_position
@finished:
    txa
    clc
    adc #19
    tax
    txs
    jsr skip_spaces
    cmp #','
    bne @done
    jsr consume
    jmp next_statement
@done:
    jsr check_end
    jmp dispatch

; X is the saved loop SP (or SP+5 for the limit); caller JSR bytes are below
; that frame, so indexed reads never include transient return addresses.
loop_arg:
    lda $0106,x
    sta $69
    lda $0105,x
    pha
    ora #$80
    sta $6A
    pla
    and #$80
    beq :+
    lda #$FF
:
    sta $6E
    lda $0104,x
    sta $6B
    lda $0103,x
    sta $6C
    lda $0102,x
    sta $6D
    rts
next_error:
    ldx #<next_text
    ldy #>next_text
    jmp error
type_error:
    ldx #<type_text
    ldy #>type_text
    jmp error
next_text: .byte "?NEXT WITHOUT FOR ERROR",0
type_text: .byte "?TYPE MISMATCH ERROR",0

scalar_type:
    copy_pointer $47, ptr
    decrement ptr
    decrement ptr
    ldy #0
    lda (ptr),y
    sta $45
    iny
    lda (ptr),y
    sta $46
    rts

.segment "PAGE0"
