.segment "CODE"
; Original guest conditional and subroutine control, no host language state.
if_statement:
    jsr expression
    lda $0D
    bpl :+
    jsr string_release          ; IF uses the returned value's zero/length state
:
    jsr skip_spaces
    cmp #$A7                    ; THEN
    beq @then
    cmp #$89                    ; published IF expression GOTO form
    jne syntax_error
    lda $61
    jeq skip_line
    jsr consume
    jmp goto_statement

@then:
    jsr consume
    lda $61                     ; any nonzero floating value is true
    jeq skip_line
    jsr skip_spaces
    cmp #'0'
    jcc dispatch
    cmp #('9'+1)
    jcs dispatch
    jmp goto_statement

on_statement:
    jsr byte_value
    sta inidx
    jsr skip_spaces
    cmp #$89
    beq @kind
    cmp #$8D
    jne syntax_error
@kind:
    sta mode
    jsr consume
    lda #0
    sta $49
    lda #$FF
    sta $4A                     ; no selected line (line numbers are below $FA00)
@line:
    jsr skip_spaces
    jsr decimal_line
    lda inidx
    beq @separator
    dec inidx
    bne @separator
    copy_pointer num, $49
@separator:
    jsr skip_spaces
    cmp #','
    bne @end
    jsr consume
    jmp @line
@end:
    jsr check_end
    lda $4A
    cmp #$FF
    jeq dispatch
    copy_pointer $49, num
    lda mode
    cmp #$8D
    jeq gosub_target
    jmp goto_target

gosub_statement:
    jsr skip_spaces
    jsr decimal_line
    jsr check_end
gosub_target:
    tsx
    cpx #$D0                    ; retain room for numeric workspace and interrupts
    jcc memory_error
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
    lda #$8D
    pha
    ; Validate before resolving the target. The saved caller stays on the real
    ; CPU stack, independent of scratch pointers changed by the validated search.
    jsr validate_program
    jsr find_line
    jcc undefined_error
    lda #0
    sta $9D
    jmp start_line
return_statement:
    jsr check_end
    tsx
@search:
    cpx #$F8
    jcs return_error
    lda $0101,x
    cmp #$8D
    beq @frame
    cmp #$81
    jne return_error
    cpx #$E6
    jcs return_error
    txa
    clc
    adc #19                     ; RETURN unwinds active FOR records in its callee
    tax
    jmp @search
@frame:
    cpx #$F2
    jcs return_error
    txs
    pla
    cmp #$8D
    jne return_error
    lda #$8D
    sta mode
return_context:
    pla
    sta txt
    pla
    sta txt+1
    pla
    sta $45
    pla
    sta $46
    pla
    sta $39
    pla
    sta $3A
resume_position:
    lda $3A
    cmp #$FF
    bne @programme
    lda $39
    cmp #$FF
    jne bad_program
    lda txt+1
    cmp #2
    jne bad_program
    lda txt
    cmp #80                     ; input-length scratch may be used by arithmetic
    jcc @position
    jne bad_program
    jmp @position
@programme:
    copy_pointer $39, num
    jsr validate_program
    jsr find_line
    jcc bad_program
    lda cur
    cmp $45
    jne bad_program
    lda cur+1
    cmp $46
    jne bad_program
    clc
    lda cur
    adc #4
    sta ptr
    lda cur+1
    adc #0
    sta ptr+1
    lda txt+1
    cmp ptr+1
    jcc bad_program
    bne @end_bound
    lda txt
    cmp ptr
    jcc bad_program
@end_bound:
    ldy #0
    lda (cur),y
    sta ptr
    iny
    lda (cur),y
    sta ptr+1
    lda txt+1
    cmp ptr+1
    jcc @position
    jne bad_program
    lda txt
    cmp ptr
    jcs bad_program
@position:
    lda mode
    cmp #$95
    beq @resume
    jsr peek
    jeq @resume
    cmp #':'
    jne bad_program
@resume:
    jmp dispatch
return_error:
    ldx #<return_text
    ldy #>return_text
    jmp error
return_text: .byte "?RETURN WITHOUT GOSUB ERROR",0

.segment "PAGE0"
