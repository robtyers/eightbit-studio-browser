; Original guest expression/scalar layer. Numeric values are FAC/ARG or guest
; hardware-stack operands; the private host page records never contain values.
expression:
    jsr expression_and
@next:
    jsr skip_spaces
    cmp #$B0
    bne @done
    jsr save_fac
    jsr consume
    jsr expression_and
    jsr join_operands
    jsr require_both_numeric
    lda #10
    jsr numeric_call
    jmp @next
@done:
    rts
expression_and:
    jsr expression_not
@next:
    jsr skip_spaces
    cmp #$AF
    bne @done
    jsr save_fac
    jsr consume
    jsr expression_not
    jsr join_operands
    jsr require_both_numeric
    lda #9
    jsr numeric_call
    jmp @next
@done:
    rts
expression_not:
    tsx
    cpx #$90
    jcc formula_error
    jsr skip_spaces
    cmp #$A8
    bne expression_compare
    jsr consume
    jsr expression_not
    jsr require_numeric
    lda #11
    jmp numeric_call
expression_compare:
    jsr expression_sum
@next:
    jsr skip_spaces
    jsr relation_bit
    beq @done
    sta token
    jsr consume
    jsr skip_spaces
    jsr relation_bit
    beq @right
    sta delta
    and token
    jne syntax_error             ; repeated comparison sign is not a pair
    lda delta
    ora token
    sta token
    jsr consume
@right:
    lda token
    pha
    jsr save_fac
    jsr expression_sum
    jsr join_operands
    lda $0D
    ora $12
    bmi @strings
    lda #8
    jsr numeric_call
    jmp @ordered
@strings:
    jsr string_compare
@ordered:
    sta delta
    pla
    and delta
    jsr boolean_number
    jmp @next
@done:
    rts
relation_bit:
    cmp #$B1
    bne :+
    lda #1
    rts
:
    cmp #$B2
    bne :+
    lda #2
    rts
:
    cmp #$B3
    bne :+
    lda #4
    rts
:
    lda #0
    rts
boolean_number:
    php
    lda #0
    sta $0D
    plp
    beq @zero
    lda #$FF
@zero:
    sta num
    sta num+1
    lda #6
    jmp numeric_call

expression_sum:
    jsr expression_term
@next:
    jsr skip_spaces
    cmp #$AA
    beq @binary
    cmp #$AB
    beq @binary
@done:
    rts
@binary:
    pha
    jsr save_fac
    jsr consume
    jsr expression_term
    jsr join_operands
    pla
    cmp #$AA
    bne @minus
    lda #0
    jmp @calculate
@minus:
    lda #1
@calculate:
    pha
    lda $0D
    ora $12
    bmi @strings
    pla
    jsr numeric_call
    jmp @next
@strings:
    pla
    jne type_error
    jsr string_concat
    jmp @next
expression_term:
    jsr expression_power
@next:
    jsr skip_spaces
    cmp #$AC
    beq @binary
    cmp #$AD
    bne @done
@binary:
    jsr require_numeric
    pha
    jsr save_fac
    jsr consume
    jsr expression_power
    jsr require_numeric
    jsr join_operands
    pla
    cmp #$AC
    bne @divide
    lda #2
    jmp @calculate
@divide:
    lda #3
@calculate:
    jsr numeric_call
    jmp @next
@done:
    rts
expression_power:
    jsr expression_factor
@next:
    jsr skip_spaces
    cmp #$AE
    bne @done
    jsr require_numeric
    jsr save_fac
    jsr consume
    jsr expression_factor
    jsr require_numeric
    jsr join_operands
    lda #25
    jsr numeric_call
    jmp @next
@done:
    rts
expression_factor:
    jsr skip_spaces
    jmp ($030A)
factor_default:
    tsx
    cpx #$90
    jcc formula_error
    jsr skip_spaces
    cmp #$FF
    beq @pi
    cmp #$A5
    jeq function_evaluate
    cmp #34
    jeq string_literal
    cmp #$AA
    beq @positive
    cmp #$AB
    beq @negative
    cmp #$A8
    jeq expression_not
    cmp #'('
    beq @parenthesis
    cmp #$B4
    jcc @literal
    cmp #$C3
    jcc function_value
    cmp #$C2
    jeq function_value
    cmp #$C3
    jcc @literal
    cmp #$CB
    jcc function_value
@literal:
    cmp #'.'
    beq @number
    cmp #'0'
    bcc @other
    cmp #('9'+1)
    bcc @number
@other:
    cmp #'A'
    bcc @unsupported
    cmp #('Z'+1)
    bcs @unsupported
    jsr variable_name
    jsr special_variable
    bcs @done
    jsr variable_lookup
    jmp variable_load
@number:
    lda #0
    sta $0D
    lda #4
    jmp numeric_call
@pi:
    jsr consume
    lda #0
    sta $0D
    lda #26
    jmp numeric_call
@positive:
    jsr consume
    jmp expression_power
@negative:
    jsr consume
    jsr expression_power
    jsr require_numeric
    lda $61
    beq @done
    lda $66
    eor #$FF
    sta $66
@done:
    rts
@parenthesis:
    jsr consume
    jsr expression
    jsr skip_spaces
    cmp #')'
    jne syntax_error
    jsr consume
    rts
@unsupported:
    cmp #$80
    jcs unsupported_error
    jmp syntax_error

; A function argument is evaluated by the same guest parser as other operands.
; The function identity survives nested calls on the real processor stack.
function_value:
    pha
    jsr consume
    jsr skip_spaces
    cmp #'('
    jne syntax_error
    jsr consume
    jsr expression
    tsx
    lda $0101,x
    cmp #$C8
    jcs substring_arguments
    jsr skip_spaces
    cmp #')'
    jne syntax_error
    jsr consume
    pla
    cmp #$C3
    jcs string_function
    cmp #$B8
    beq @free
    pha
    jsr require_numeric
    pla
    cmp #$B4
    bne :+
    lda #12
    jmp numeric_call
:
    cmp #$B5
    bne :+
    lda #14
    jmp numeric_call
:
    cmp #$B6
    bne :+
    lda #13
    jmp numeric_call
:
    cmp #$B7
    beq @usr
    cmp #$B9
    beq @position
    cmp #$BA
    beq @sqrt
    cmp #$BB
    jcc @peek
    cmp #$C2
    jcs @peek
    sec
    sbc #$A9                    ; BB..C1 -> original services 18..24
    jmp numeric_call
@peek:
    cmp #$C2
    jne unsupported_error
    lda #15
    jsr numeric_call
    ldy #0
    lda (num),y
    jmp byte_number
@free:
    lda $0D
    bpl :+
    jsr string_release
:
    jsr string_collect
    lda #0
    sta $0D
    sec
    lda $33
    sbc $31
    sta num
    lda $34
    sbc $32
    sta num+1
    lda #6
    jmp numeric_call
@position:
    lda $D3
    jmp byte_number
@sqrt:
    lda #17
    jmp numeric_call
@usr:
    jsr call_usr
    cld
    rts
call_usr:
    jmp ($0311)
byte_number:
    sta num
    lda #0
    sta $0D
    sta num+1
    lda #6
    jmp numeric_call

; Real JSR return bytes are temporarily moved while six FAC bytes are pushed or
; popped. This is guest stack code, not a continuation/register override.
save_fac:
    tsx
    cpx #$90
    jcc formula_error
    pla
    sta $26
    pla
    sta $27
    lda $0D
    pha
    ldx #0
@push:
    lda $61,x
    pha
    inx
    cpx #6
    bcc @push
    lda $27
    pha
    lda $26
    pha
    rts
join_operands:
    ldx #5
@right:
    lda $61,x
    sta $69,x
    dex
    bpl @right
    pla
    sta $26
    pla
    sta $27
    ldx #5
@left:
    pla
    sta $61,x
    dex
    bpl @left
    pla
    sta $12
    lda $27
    pha
    lda $26
    pha
    rts
numeric_call:
    pha
    cmp #7
    beq :+
    lda #0
    sta $0E
:
    pla
    jsr gate_math
    cpx #0
    jne numeric_error
    rts
numeric_output:
    jsr require_numeric
    lda #5
    jsr numeric_call
    ldx #0
@character:
    lda $0100,x
    beq @done
    jsr $FFD2                    ; page zero is restored before user output hooks
    inx
    cpx #32
    bcc @character
    jmp bad_program
@done:
    rts
numeric_error:
    cpx #1
    jeq overflow_error
    cpx #2
    jeq division_error
    cpx #3
    jeq quantity_error
    cpx #4
    jeq formula_error
    cpx #5
    jeq unsupported_error
    jmp syntax_error

assignment:
    jsr variable_name
    lda $45
    cmp #'T'
    bne :+
    lda $46
    cmp #('I'+$80)
    jeq time_assignment
    cmp #'I'
    jeq syntax_error
:
    lda $45
    cmp #'S'
    bne :+
    lda $46
    cmp #'T'
    jeq syntax_error
:
    jsr variable_lookup
    lda $45
    pha
    lda $46
    pha
    lda $2F
    pha
    lda $30
    pha
    lda $47
    pha
    lda $48
    pha
    jsr skip_spaces
    cmp #$B2
    jne syntax_error
    jsr consume
    jsr expression
    jsr check_end
    pla
    sta $48
    pla
    sta $47
    pla
    sta $4A
    pla
    sta $49
    jsr adjust_array_destination
    pla
    sta $46
    pla
    sta $45
    jsr variable_store
    jmp dispatch

variable_name:
    jsr skip_spaces
    cmp #'A'
    jcc syntax_error
    cmp #('Z'+1)
    jcs syntax_error
    sta $45
    lda #0
    sta $46
    jsr consume
    jsr name_character
    bcc @suffix
    sta $46
    jsr consume
@remaining:
    jsr name_character
    bcc @suffix
    jsr consume
    jmp @remaining
@suffix:
    cmp #'$'
    bne @integer
    lda $46
    ora #$80
    sta $46
    jsr consume
    jmp @special
@integer:
    cmp #'%'
    bne @special
    lda $45
    ora #$80
    sta $45
    lda $46
    ora #$80
    sta $46
    jsr consume
@special:
@done:
    jsr skip_spaces
    rts
name_character:
    cmp #'A'
    bcc @digit
    cmp #('Z'+1)
    bcc @yes
@digit:
    cmp #'0'
    bcc @no
    cmp #('9'+1)
    bcc @yes
@no:
    clc
    rts
@yes:
    sec
    rts

; Validate public scalar/array/string boundaries before scanning or allocating.
variable_lookup:
    jsr variable_bounds
    jsr skip_spaces
    cmp #'('
    jne scalar_lookup
    lda #0
    jmp array_reference
variable_bounds:
    jsr validate_bounds
    clc
    lda $2B
    adc #2
    sta ptr
    lda $2C
    adc #0
    sta ptr+1
    lda $2E
    cmp ptr+1
    jcc bad_program
    bne :+
    lda $2D
    cmp ptr
    jcc bad_program
:
    ldx #$2D
@bounds:
    lda $00,x
    sta ptr
    lda $01,x
    sta ptr+1
    lda ptr+1
    cmp $03,x
    jcc @ordered
    jne bad_program
    lda ptr
    cmp $02,x
    jeq @ordered
    jcs bad_program
@ordered:
    inx
    inx
    cpx #$33
    bcc @bounds
    lda $34
    cmp $38
    jcc @string_bound
    jne bad_program
    lda $33
    cmp $37
    jeq @string_bound
    jcs bad_program
@string_bound:
    rts
scalar_lookup:
    copy_pointer $2D, ptr
@record:
    lda ptr
    cmp $2F
    bne @next
    lda ptr+1
    cmp $30
    jeq @allocate
@next:
    clc
    lda ptr
    adc #7
    sta nxt
    lda ptr+1
    adc #0
    sta nxt+1
    cmp $30
    jcc @inside
    jne bad_program
    lda nxt
    cmp $2F
    jeq @inside
    jcs bad_program
@inside:
    ldy #0
    lda (ptr),y
    cmp $45
    bne @advance
    iny
    lda (ptr),y
    cmp $46
    beq @found
@advance:
    copy_pointer nxt, ptr
    jmp @record
@allocate:
    lda #7
    sta num
    lda #0
    sta num+1
    jsr grow_heap
    copy_pointer $31, src
    copy_pointer nxt, dst
@move_arrays:
    lda src
    cmp $2F
    bne @move_byte
    lda src+1
    cmp $30
    beq @fit
@move_byte:
    decrement src
    decrement dst
    ldy #0
    lda (src),y
    sta (dst),y
    jmp @move_arrays
@fit:
    copy_pointer $2F, ptr
    copy_pointer nxt, $31
    clc
    lda $2F
    adc #7
    sta $2F
    lda $30
    adc #0
    sta $30
    ldy #0
    lda $45
    sta (ptr),y
    iny
    lda $46
    sta (ptr),y
    iny
    lda #0
@zero:
    sta (ptr),y
    iny
    cpy #7
    bcc @zero
@found:
    clc
    lda ptr
    adc #2
    sta $47
    lda ptr+1
    adc #0
    sta $48
    rts
variable_type:
    lda #0
    sta $0E
    lda $45
    bpl @done
    lda $46
    bpl @done
    lda #$80
    sta $0E
@done:
    lda $45
    rts
variable_load:
    jsr variable_type
    bmi @integer
    lda $46
    bmi @string
    lda #0
    sta $0D
    ldy #0
    lda ($47),y
    sta $61
    bne @nonzero
    lda #0
    ldx #5
@zero:
    sta $61,x
    dex
    bpl @zero
    rts
@nonzero:
    iny
    lda ($47),y
    pha
    ora #$80
    sta $62
    pla
    and #$80
    beq @positive
    lda #$FF
@positive:
    sta $66
    ldy #2
@mantissa:
    lda ($47),y
    sta $61,y
    iny
    cpy #5
    bcc @mantissa
    rts
@integer:
    lda #0
    sta $0D
    ldy #0
    lda ($47),y
    sta num+1
    iny
    lda ($47),y
    sta num
    lda #6
    jmp numeric_call
@string:
    ldy #0
    lda ($47),y
    sta $61
    iny
    lda ($47),y
    sta $62
    iny
    lda ($47),y
    sta $63
    jmp string_temporary
variable_store:
    jsr variable_type
    bmi integer_store
    lda $46
    bmi string_store_value
    jsr require_numeric
float_store:
    ldy #0
    lda $61
    sta ($47),y
    beq @zero
    iny
    lda $62
    and #$7F
    sta delta
    lda $66
    and #$80
    ora delta
    sta ($47),y
    ldy #2
@mantissa:
    lda $61,y
    sta ($47),y
    iny
    cpy #5
    bcc @mantissa
    rts
@zero:
    ldy #1
    lda #0
@fill:
    sta ($47),y
    iny
    cpy #5
    bcc @fill
    rts
integer_store:
    jsr require_numeric
    lda #7
    jsr numeric_call
    ldy #0
    lda num+1
    sta ($47),y
    iny
    lda num
    sta ($47),y
    lda #0
    iny
    rts                         ; array integers occupy exactly two bytes
string_store_value:
    lda $0D
    bmi :+
    jmp type_error
:
    jmp string_store

overflow_error:
    ldx #<overflow_text
    ldy #>overflow_text
    jmp error
division_error:
    ldx #<division_text
    ldy #>division_text
    jmp error
quantity_error:
    ldx #<quantity_text
    ldy #>quantity_text
    jmp error
formula_error:
    ldx #<formula_text
    ldy #>formula_text
    jmp error
.segment "RODATA"
overflow_text: .byte "?OVERFLOW ERROR",0
division_text: .byte "?DIVISION BY ZERO ERROR",0
quantity_text: .byte "?ILLEGAL QUANTITY ERROR",0
formula_text: .byte "?FORMULA TOO COMPLEX ERROR",0
.segment "PAGE0"
