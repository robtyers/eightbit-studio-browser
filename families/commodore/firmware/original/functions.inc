.segment "PAGE0"
; Original DEF/FN records use the public scalar table; arguments, the old formal
; value and expression continuations remain on the actual guest CPU stack.
function_name:
    jsr variable_name
    lda $45
    bmi @bad
    lda $46
    bmi @bad
    lda $45
    ora #$80
    sta $45
    jsr variable_bounds
    jmp scalar_lookup
@bad:
    jmp syntax_error
function_definition:
    lda $3A
    cmp #$FF
    jeq direct_input_error
    jsr skip_spaces
    cmp #$A5
    jne syntax_error
    jsr consume
    jsr function_name
    lda $47
    pha
    lda $48
    pha
    jsr skip_spaces
    cmp #'('
    jne syntax_error
    jsr consume
    jsr variable_name
    lda $45
    ora $46
    bmi @bad
    jsr variable_lookup
    jsr skip_spaces
    cmp #')'
    jne syntax_error
    jsr consume
    jsr skip_spaces
    cmp #$B2
    jne syntax_error
    jsr consume
    copy_pointer $47, num
    pla
    sta $48
    pla
    sta $47
    ldy #0
    lda txt
    sta ($47),y
    iny
    lda txt+1
    sta ($47),y
    iny
    lda num
    sta ($47),y
    iny
    lda num+1
    sta ($47),y
    iny
    lda #0
    sta ($47),y
    jmp data_statement          ; skip the unevaluated expression to colon/NUL
@bad:
    jmp syntax_error

function_evaluate:
    jsr consume
    jsr function_name
    ldy #0
    lda ($47),y
    pha
    iny
    lda ($47),y
    jeq undefined_function
    pha
    iny
    lda ($47),y
    pha
    iny
    lda ($47),y
    pha
    jsr skip_spaces
    cmp #'('
    jne syntax_error
    jsr consume
    jsr expression
    jsr require_numeric
    jsr skip_spaces
    cmp #')'
    jne syntax_error
    jsr consume
    pla
    sta $48
    pla
    sta $47
    pla
    sta $4A
    pla
    sta $49
    ldy #0
@old_value:
    lda ($47),y
    pha
    iny
    cpy #5
    bcc @old_value
    jsr float_store
    lda $47
    pha
    lda $48
    pha
    lda txt
    pha
    lda txt+1
    pha
    copy_pointer $49, txt
    jsr expression
    jsr require_numeric
    jsr check_end
    pla
    sta txt+1
    pla
    sta txt
    pla
    sta $48
    pla
    sta $47
    ldy #4
@restore:
    pla
    sta ($47),y
    dey
    bpl @restore
    rts
undefined_function:
    ldx #<function_error_text
    ldy #>function_error_text
    jmp error
function_error_text: .byte "?UNDEF'D FUNCTION ERROR",0
