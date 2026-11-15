.segment "PAGE0"
; Original array implementation, using the measured public name/span/dimension
; header and packed 5/2/3-byte elements. Subscripts and DIM bounds live on the
; hardware stack while their guest expressions are evaluated.
dim_statement:
    jsr variable_name
    jsr skip_spaces
    cmp #'('
    jne syntax_error
    lda #1
    jsr array_reference
    jsr skip_spaces
    cmp #','
    bne @done
    jsr consume
    jmp dim_statement
@done:
    jsr check_end
    jmp dispatch

array_reference:
    pha
    lda $45
    pha
    lda $46
    pha
    jsr consume
    lda #0
    sta $0B
@index:
    tsx
    cpx #$A0
    jcc formula_error
    lda $0B
    pha
    jsr address_value
    pla
    clc
    adc #1
    sta $0B
    lda num
    pha
    lda num+1
    pha
    jsr skip_spaces
    cmp #','
    bne @close
    jsr consume
    jmp @index
@close:
    cmp #')'
    jne syntax_error
    jsr consume
    tsx
    stx $0F
    txa
    clc
    adc $0B
    adc $0B
    tax
    lda $0101,x
    sta $46
    lda $0102,x
    sta $45
    lda $0103,x
    sta $0C
    copy_pointer $2F, ptr
@record:
    lda ptr
    cmp $31
    bne @header
    lda ptr+1
    cmp $32
    jeq array_create
@header:
    jsr array_span
    ldy #0
    lda (ptr),y
    cmp $45
    bne @next
    iny
    lda (ptr),y
    cmp $46
    beq @found
@next:
    copy_pointer nxt, ptr
    jmp @record
@found:
    lda $0C
    jne redim_error
array_element:
    ldy #4
    lda (ptr),y
    cmp $0B
    jne subscript_error
    jsr array_size
    lda #0
    sta num
    sta num+1
    lda #5
    sta $54
    ldx $0F
    inx
    stx $4E
@dimension:
    ldy $54
    lda (ptr),y
    sta $57
    iny
    lda (ptr),y
    sta $56
    ldx $4E
    lda $0100,x
    cmp $57
    bcc @in_range
    jne subscript_error
    lda $0101,x
    cmp $56
    jcs subscript_error
@in_range:
    jsr array_multiply
    ldx $4E
    clc
    lda num
    adc $0101,x
    sta num
    lda num+1
    adc $0100,x
    sta num+1
    jcs subscript_error
    inc $4E
    inc $4E
    inc $54
    inc $54
    lda $0B
    asl
    clc
    adc #5
    cmp $54
    bne @dimension
    lda $55
    sta $56
    lda #0
    sta $57
    jsr array_multiply
    clc
    lda num
    adc $54
    sta num
    lda num+1
    adc #0
    sta num+1
    clc
    lda ptr
    adc num
    sta $47
    lda ptr+1
    adc num+1
    sta $48
    jcs bad_program
    clc
    lda $47
    adc $55
    sta $58
    lda $48
    adc #0
    jcs bad_program
    cmp nxt+1
    jcc array_return
    jne bad_program
    lda $58
    cmp nxt
    jcc array_return
    jne bad_program
    jmp array_return

array_create:
    jsr array_size
    lda $55
    sta num
    lda #0
    sta num+1
    lda #0
    sta $4E
@count:
    jsr array_bound
    jsr array_multiply
    inc $4E
    lda $4E
    cmp $0B
    bcc @count
    lda $0B
    asl
    clc
    adc #5
    sta $54
    clc
    adc num
    sta num
    lda num+1
    adc #0
    sta num+1
    jcs memory_error
    jsr grow_heap
    copy_pointer $31, ptr
    copy_pointer nxt, $31
    copy_pointer ptr, dst
    ldy #0
    lda #0
@clear:
    sta (dst),y
    increment dst
    lda dst
    cmp nxt
    bne @zero
    lda dst+1
    cmp nxt+1
    beq @header
@zero:
    lda #0
    jmp @clear
@header:
    ldy #0
    lda $45
    sta (ptr),y
    iny
    lda $46
    sta (ptr),y
    iny
    lda num
    sta (ptr),y
    iny
    lda num+1
    sta (ptr),y
    iny
    lda $0B
    sta (ptr),y
    lda #0
    sta $4E
@bounds:
    jsr array_bound
    lda $4E
    asl
    clc
    adc #5
    tay
    lda $57
    sta (ptr),y
    iny
    lda $56
    sta (ptr),y
    inc $4E
    lda $4E
    cmp $0B
    bcc @bounds
    lda $0C
    jeq array_element
array_return:
    lda $0B
    asl
    clc
    adc #3
    adc $0F
    tax
    txs
    rts

array_bound:
    lda $0C
    beq @implicit
    lda $4E
    asl
    clc
    adc $0F
    tax
    lda $0102,x
    clc
    adc #1
    sta $56
    lda $0101,x
    adc #0
    sta $57
    jcs memory_error
    rts
@implicit:
    lda #11
    sta $56
    lda #0
    sta $57
    rts
array_size:
    lda #2
    ldx $45
    bmi @done
    lda #3
    ldx $46
    bmi @done
    lda #5
@done:
    sta $55
    rts

; NUM *= $56/$57, with overflow rejected before any allocation or table write.
array_multiply:
    copy_pointer num, $58
    lda #0
    sta num
    sta num+1
    ldx #16
@bit:
    lsr $57
    ror $56
    bcc @shift
    clc
    lda num
    adc $58
    sta num
    lda num+1
    adc $59
    sta num+1
    jcs memory_error
@shift:
    dex
    beq @done
    asl $58
    rol $59
    bcc @bit
    lda $56
    ora $57
    jne memory_error
@done:
    rts

array_span:
    ; A complete header and its forward span must fit within the array table.
    clc
    lda ptr
    adc #5
    sta nxt
    lda ptr+1
    adc #0
    sta nxt+1
    jcs bad_program
    cmp $32
    jcc @read
    jne bad_program
    lda nxt
    cmp $31
    jcs bad_program
@read:
    ldy #2
    lda (ptr),y
    sta nxt
    iny
    lda (ptr),y
    sta nxt+1
    clc
    lda nxt
    adc ptr
    sta nxt
    lda nxt+1
    adc ptr+1
    sta nxt+1
    jcs bad_program
    cmp $32
    jcc @dimensions
    jne bad_program
    lda nxt
    cmp $31
    jcc @dimensions
    jne bad_program
@dimensions:
    ldy #4
    lda (ptr),y
    jeq bad_program
    asl
    jcs bad_program
    adc #5
    jcs bad_program
    clc
    adc ptr
    sta $58
    lda ptr+1
    adc #0
    cmp nxt+1
    jcc @ok
    jne bad_program
    lda $58
    cmp nxt
    jcs bad_program
@ok:
    rts

grow_heap:
    jsr heap_candidate
    bcs @done
    jsr string_collect
    jsr heap_candidate
    jcc memory_error
@done:
    rts
heap_candidate:
    clc
    lda $31
    adc num
    sta nxt
    lda $32
    adc num+1
    sta nxt+1
    bcs @no
    cmp $34
    bcc @yes
    bne @no
    lda nxt
    cmp $33
    bcc @yes
    beq @yes
@no:
    clc
    rts
@yes:
    sec
    rts

adjust_array_destination:
    lda $48
    cmp $4A
    bcc @done
    bne @array
    lda $47
    cmp $49
    bcc @done
@array:
    sec
    lda $2F
    sbc $49
    sta delta
    lda $30
    sbc $4A
    sta size
    clc
    lda $47
    adc delta
    sta $47
    lda $48
    adc size
    sta $48
@done:
    rts
subscript_error:
    ldx #<subscript_text
    ldy #>subscript_text
    jmp error
redim_error:
    ldx #<redim_text
    ldy #>redim_text
    jmp error
subscript_text: .byte "?BAD SUBSCRIPT ERROR",0
redim_text: .byte "?REDIM'D ARRAY ERROR",0
