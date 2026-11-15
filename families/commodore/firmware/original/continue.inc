.segment "PAGE0"
; A suspended continuation is an explicit guest stack record. Ordinary direct
; commands execute below it; RUN/CLR/edit/error invalidate it through OLDTXT.
stop_program:
    sta mode
    lda $3A
    cmp #$FF
    bne @programme
    lda mode
    jeq gate_recovery
    jmp break_error
@programme:
    copy_pointer $39, $3B
    copy_pointer txt, $3D
    lda $3A
    pha
    lda $39
    pha
    lda cur+1
    pha
    lda cur
    pha
    lda txt+1
    pha
    lda txt
    pha
    lda #$95
    pha
    lda mode
    beq @prompt
    jsr ensure_newline
    ldx #<break_text
    ldy #>break_text
    jsr message
    ldx #<in_text
    ldy #>in_text
    jsr message
    copy_pointer $39, num
    jsr print_unsigned
    lda #13
    jsr $FFD2
@prompt:
    jmp prompt_entry
continue_statement:
    jsr check_end
    lda $3E
    jeq continue_error
    tsx
@record:
    cpx #$F2
    jcs continue_error
    lda $0101,x
    cmp #$95
    beq @found
    cmp #$8D
    beq @call
    cmp #$81
    jne continue_error
    cpx #$E6
    jcs continue_error
    txa
    clc
    adc #19
    tax
    jmp @record
@call:
    txa
    clc
    adc #7
    tax
    jmp @record
@found:
    txs
    pla
    sta mode
    lda #0
    sta $3E
    sta $9D
    jmp return_context
continue_error:
    ldx #<continue_text
    ldy #>continue_text
    jmp error
continue_text: .byte "?CAN'T CONTINUE ERROR",0
