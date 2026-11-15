.segment "PAGE0"
; Original DATA traversal uses the public DATPTR/DATLIN fields. The actual READ
; continuation is held on the CPU stack while TXT/CUR temporarily visit DATA.
restore_data:
    sec
    lda $2B
    sbc #1
    sta $41
    lda $2C
    sbc #0
    sta $42
    lda #0
    sta $3F
    sta $40
    rts
read_statement:
    lda #$98
    sta $11
    jsr variable_name
    jsr variable_lookup
    lda #0
    sta $0D
    lda $45
    bmi :+
    lda $46
    bpl :+
    lda #$FF
    sta $0D
:
    lda $47
    pha
    lda $48
    pha
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
    jsr next_data
    copy_pointer txt, $41
    pla
    sta txt
    pla
    sta txt+1
    pla
    sta cur
    pla
    sta cur+1
    pla
    sta $39
    pla
    sta $3A
    pla
    sta $48
    pla
    sta $47
    jsr variable_store
    jsr skip_spaces
    cmp #','
    bne @done
    jsr consume
    jmp read_statement
@done:
    jsr check_end
    jmp dispatch

next_data:
    jsr validate_program
    lda $40
    ora $3F
    bne @existing
    ; DATLIN=0 can also describe a real DATA line zero. The reset DATPTR is
    ; distinguished by its exact TXTTAB-1 address, never by an arbitrary byte.
    clc
    lda $41
    adc #1
    sta ptr
    lda $42
    adc #0
    cmp $2C
    bne @existing
    lda ptr
    cmp $2B
    bne @existing
    copy_pointer $2B, cur
    jmp data_line
@existing:
    copy_pointer $3F, num
    jsr find_line
    jcc bad_program
    copy_pointer $41, txt
    ; Reject a poisoned DATPTR before scanning any unrelated guest memory.
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
    bne :+
    lda txt
    cmp ptr
    jcc bad_program
:
    ldy #0
    lda (cur),y
    sta ptr
    iny
    lda (cur),y
    sta ptr+1
    lda txt+1
    cmp ptr+1
    jcc @delimiter
    jne bad_program
    lda txt
    cmp ptr
    jcs bad_program
@delimiter:
    jsr peek
    cmp #','
    bne data_search
    jsr consume
    jmp data_value
data_line:
    ldy #1
    lda (cur),y
    jeq data_exhausted
    ldy #2
    lda (cur),y
    sta $3F
    iny
    lda (cur),y
    sta $40
    clc
    lda cur
    adc #4
    sta txt
    lda cur+1
    adc #0
    sta txt+1
data_search:
    lda #0
    sta mode
@byte:
    jsr peek
    beq @next_line
    cmp #34
    bne @token
    lda mode
    eor #1
    sta mode
    jmp @consume
@token:
    ldx mode
    bne @consume
    cmp #$8F
    beq @next_line
    cmp #$83
    beq @found
@consume:
    jsr consume
    jmp @byte
@next_line:
    ldy #0
    lda (cur),y
    tax
    iny
    lda (cur),y
    sta cur+1
    stx cur
    jmp data_line
@found:
    jsr consume
data_value:
    copy_pointer $3F, $39
    jsr skip_spaces
    ldx $0D
    bmi data_string
    cmp #','
    beq @empty
    cmp #':'
    beq @empty
    cmp #0
    beq @empty
    cmp #'-'
    beq @negative
    cmp #'+'
    bne @number
    jsr consume
    jmp @number
@negative:
    jsr consume
    lda #1
    pha
    jmp @parse
@number:
    lda #0
    pha
@parse:
    lda #0
    sta $0D
    lda #4
    jsr numeric_call
    pla
    beq @end
    lda $61
    beq @end
    lda #$FF
    sta $66
@end:
    jsr skip_spaces
    beq @done
    cmp #','
    beq @done
    cmp #':'
    jne syntax_error
@done:
    rts
@empty:
    lda #0
    jmp byte_number
data_exhausted:
    ; Exhaustion belongs to the READ statement's caller line, which remains in
    ; the public execution fields until a DATA value has actually been found.
    ldx #<data_text
    ldy #>data_text
    jmp error
data_text: .byte "?OUT OF DATA ERROR",0

data_string:
    lda #0
    sta $10
    jmp string_data_entry
input_string:
    lda #$FF
    sta $10
string_data_entry:
    jsr peek
    cmp #34
    beq @quoted
    lda #0
    sta mode
    jmp @begin
@quoted:
    lda #1
    sta mode
    jsr consume
@begin:
    copy_pointer txt, $62
    ldx #0
@byte:
    jsr peek
    beq @end
    ldy mode
    beq @unquoted
    cmp #34
    beq @close
    jmp @take
@unquoted:
    cmp #','
    beq @trim
    cmp #':'
    bne @take
    lda $10
    beq @trim
@take:
    inx
    jeq long_error
    jsr consume
    jmp @byte
@close:
    jsr consume
    jsr skip_spaces
    beq @end
    cmp #','
    beq @end
    cmp #':'
    jne syntax_error
    jmp @end
@trim:
    txa
    tay
@space:
    cpy #0
    beq @length
    dey
    lda ($62),y
    cmp #32
    beq @space
    iny
@length:
    tya
    tax
@end:
    stx $61
    jmp string_temporary

.segment "PAGE0"
