.segment "PAGE0"
; Original BASIC channel syntax; only the original keyboard/screen channels
; can open. Unattached peripherals retain KERNAL's real error/carry path.
file_defaults:
    lda #0
    sta $B7
    sta $BB
    sta $BC
    sta $B9
    lda #1
    sta $BA
    rts
file_name:
    jsr expression
    jsr require_string
    lda $61
    ldx $62
    ldy $63
    jsr $FFBD
    jmp string_release
open_statement:
    jsr file_defaults
    jsr byte_value
    sta $B8
    jsr skip_spaces
    cmp #','
    bne @open
    jsr consume
    jsr byte_value
    sta $BA
    jsr skip_spaces
    cmp #','
    bne @open
    jsr consume
    jsr byte_value
    sta $B9
    jsr skip_spaces
    cmp #','
    bne @open
    jsr consume
    jsr file_name
@open:
    jsr check_end
    jsr $FFC0
    jcs file_error
    jmp dispatch
close_statement:
    jsr byte_value
    pha
    jsr check_end
    pla
    jsr $FFC3
    jcs file_error
    jmp dispatch
file_output:
    jsr byte_value
    ldx num
    jsr $FFC9
    jcs file_error
    rts
file_print_statement:
    jsr file_output
    jsr skip_spaces
    cmp #','
    bne file_print_body
    jsr consume
file_print_body:
    jmp print_statement
command_statement:
    jsr file_output
    jsr skip_spaces
    cmp #','
    bne @end
    jsr consume
    jmp file_print_body
@end:
    jsr check_end
    jmp dispatch
file_input_channel:
    jsr byte_value
    jsr comma
    ldx num
    jsr $FFC6
    jcs file_error
    rts
file_input_statement:
    lda $3A
    cmp #$FF
    jeq direct_input_error
    jsr file_input_channel
    lda #$FF
    jmp input_start
file_get_statement:
    jsr consume
    jsr file_input_channel
    jmp get_variables
load_save_statement:
    jsr file_defaults
    jsr skip_spaces
    beq @call
    cmp #':'
    beq @call
    jsr file_name
    jsr skip_spaces
    cmp #','
    bne @call
    jsr consume
    jsr byte_value
    sta $BA
    jsr skip_spaces
    cmp #','
    bne @call
    jsr consume
    jsr byte_value
    sta $B9
@call:
    jsr check_end
    pla
    cmp #$94
    beq @save
    cmp #$95
    beq @verify
    lda #0
    jmp @load
@verify:
    lda #1
@load:
    ldx $2B
    ldy $2C
    jsr $FFD5
    jcs file_error
    jmp dispatch
@save:
    lda #$2B
    ldx $2D
    ldy $2E
    jsr $FFD8
    jcs file_error
    jmp dispatch
file_error:
    pha
    jsr $FFCC
    pla
    cmp #1
    jcc bad_program
    cmp #10
    jcs bad_program
    asl
    tax
    lda file_errors-2,x
    tay
    lda file_errors-1,x
    tax
    tya
    pha
    txa
    tay
    pla
    tax
    jmp error
.segment "RODATA"
file_errors:
.word files_full_text, file_open_text, file_closed_text, file_missing_text
.word device_text, file_input_text, file_output_text, file_name_text, file_device_text
files_full_text: .byte "?TOO MANY FILES ERROR",0
file_open_text: .byte "?FILE OPEN ERROR",0
file_closed_text: .byte "?FILE NOT OPEN ERROR",0
file_missing_text: .byte "?FILE NOT FOUND ERROR",0
device_text: .byte "?DEVICE NOT PRESENT ERROR",0
file_input_text: .byte "?NOT INPUT FILE ERROR",0
file_output_text: .byte "?NOT OUTPUT FILE ERROR",0
file_name_text: .byte "?MISSING FILE NAME ERROR",0
file_device_text: .byte "?ILLEGAL DEVICE NUMBER ERROR",0
.segment "PAGE0"
