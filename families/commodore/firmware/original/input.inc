.segment "PAGE0"
get_statement:
    lda $3A
    cmp #$FF
    jeq direct_input_error
    jsr skip_spaces
    cmp #'#'
    jeq file_get_statement
get_variables:
    lda #$40
    sta $11
    jsr variable_name
    jsr variable_lookup
    jsr $FFE4
    pha
    lda $45
    bmi @numeric
    lda $46
    bpl @numeric
    pla
    beq @empty
    jsr byte_number
    jsr string_character
    jmp @store
@empty:
    sta $61
    sta $62
    sta $63
    jsr string_temporary
    jmp @store
@numeric:
    pla
    beq @zero
    cmp #'0'
    jcc syntax_error
    cmp #('9'+1)
    jcs syntax_error
    sec
    sbc #'0'
@zero:
    jsr byte_number
@store:
    jsr variable_store
    jsr skip_spaces
    cmp #','
    bne @done
    jsr consume
    jmp get_variables
@done:
    jsr check_end
    jsr $FFCC
    jmp dispatch

input_statement:
    lda $3A
    cmp #$FF
    jeq direct_input_error
    jsr skip_spaces
    cmp #34
    bne @variables
    jsr string_literal
    jsr string_output
    jsr skip_spaces
    cmp #';'
    jne syntax_error
    jsr consume
@variables:
    lda #0
input_start:
    pha                         ; explicit normal/file input identity
    lda #0
    sta $11
    sta $13
    tsx
    lda $0101,x
    bne :+
    inc $13
:
    copy_pointer txt, $43
input_prompt:
    tsx
    lda $0101,x
    bne @variable
    lda #'?'
    jsr $FFD2
    lda #32
    jsr $FFD2
    jsr input_line
@variable:
    jsr variable_name
    jsr variable_lookup
    tsx
    lda $0101,x
    beq :+
    jsr input_field
:
    lda txt
    pha
    lda txt+1
    pha
    copy_pointer $35, txt
    lda $45
    bmi @numeric
    lda $46
    bpl @numeric
    jsr skip_spaces
    jsr input_string
    jmp @value
@numeric:
    jsr input_number
@value:
    jsr skip_spaces
    cmp #','
    beq @delimiter
    cmp #0
    jne input_bad_field
@delimiter:
    sta $12
    jsr consume
    copy_pointer txt, $35
    pla
    sta txt+1
    pla
    sta txt
    jsr variable_store
    jsr skip_spaces
    cmp #','
    bne @end
    jsr consume
    tsx
    lda $0101,x
    bne @variable
    lda $12
    bne @variable
    lda #'?'
    jsr $FFD2
    jmp input_prompt
@end:
    jsr check_end
    tsx
    lda $0101,x
    bne @done
    lda $12
    beq @done
    ldx #<extra_input_text
    ldy #>extra_input_text
    jsr message
@done:
    pla
    pha
    lda #0
    sta $13
    pla
    beq :+
    jsr $FFCC
:
    jmp dispatch

input_number:
    lda #0
    sta $0D
    jsr skip_spaces
    cmp #','
    beq @zero
    cmp #0
    beq @zero
    cmp #'-'
    beq @negative
    cmp #'+'
    bne @positive
    jsr consume
@positive:
    lda #0
    pha
    jmp @parse
@negative:
    jsr consume
    lda #$FF
    pha
@parse:
    lda #4
    jsr gate_math
    cpx #0
    beq @accepted
    pla                         ; sign and this parser's real JSR return
    pla
    pla
    jmp input_bad_field
@accepted:
    pla
    sta $66
    rts
@zero:
    lda #0
    jmp byte_number

input_line:
    lda #0
    sta inlen
@key:
    jsr $FFCF
    cmp #13
    beq @enter
    ldx inlen
    cpx #80
    bcs @key
    sta $0200,x
    inc inlen
    jmp @key
@enter:
    jsr $FFD2
    ldx inlen
    lda #0
    sta $0200,x
    sta $35
    lda #2
    sta $36
    rts
input_bad_field:
    pla                         ; saved statement text, before retrying its list
    sta txt+1
    pla
    sta txt
input_redo:
    tsx
    lda $0101,x
    jne bad_input_error
    lda #$19
    sta $16
    copy_pointer $43, txt
    ldx #<redo_input_text
    ldy #>redo_input_text
    jsr message
    jmp input_prompt
direct_input_error:
    ldx #<direct_input_text
    ldy #>direct_input_text
    jmp error
direct_input_text: .byte "?ILLEGAL DIRECT ERROR",0
redo_input_text: .byte "?REDO FROM START",13,0
extra_input_text: .byte "?EXTRA IGNORED",13,0

input_field:
    lda #0
    sta inlen
    sta mode
@key:
    jsr $FFCF
    cmp #34
    bne :+
    pha
    lda mode
    eor #1
    sta mode
    pla
:
    ldx mode
    bne @save
    cmp #13
    beq @end
    cmp #','
    beq @end
@save:
    ldx inlen
    cpx #80
    jcs bad_input_error
    sta $0200,x
    inc inlen
    jmp @key
@end:
    cmp #13
    bne @finish
    lda #32                    ; keyboard file input leaves output on this row
    jsr $FFD2
@finish:
    ldx inlen
    lda #0
    sta $0200,x
    sta $35
    lda #2
    sta $36
    rts
bad_input_error:
    ldx #<bad_input_text
    ldy #>bad_input_text
    jmp error
bad_input_text: .byte "?BAD DATA ERROR",0
.segment "PAGE0"
