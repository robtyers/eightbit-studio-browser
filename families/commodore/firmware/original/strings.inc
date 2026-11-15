.segment "CODE"
; Original immutable strings, public scalar descriptors and the published three
; temporary descriptors. Moving collection updates every live descriptor; values
; remain in ordinary guest RAM and collection is interruptible guest execution.
require_both_numeric:
    pha
    lda $12
    bpl :+
    jmp type_error
:
    pla
require_numeric:
    pha
    lda $0D
    bpl @ok
@error:
    jmp type_error
@ok:
    pla
    rts
require_string:
    pha
    lda $0D
    bmi @ok
    jmp type_error
@ok:
    pla
    rts
string_literal:
    jsr consume
    copy_pointer txt, $62
    jsr peek
    ldx #0
@character:
    cmp #0
    beq @end
    cmp #34
    beq @closed
    inx
    jeq long_error
    jsr consume
    jmp @character
@closed:
    jsr consume
@end:
    stx $61
string_temporary:
    ldx $16
    cpx #$19
    jcc bad_program
    cpx #$22
    jcs formula_error
    stx $66
    lda $61
    sta $00,x
    lda $62
    sta $01,x
    lda $63
    sta $02,x
    inx
    inx
    inx
    stx $16
    lda #$FF
    sta $0D
    rts
string_refresh:
    ldx $66
    lda $00,x
    sta $61
    lda $01,x
    sta $62
    lda $02,x
    sta $63
    rts
string_release:
    lda $66
    sta $16
    rts
string_output:
    jsr string_refresh
    copy_pointer $62, ptr
    ldy #0
@byte:
    cpy $61
    beq string_release
    lda (ptr),y
    jsr $FFD2
    iny
    bne @byte
    jmp string_release

string_store:
    lda $61
    jsr string_allocate
    jsr string_refresh
    copy_pointer $62, nxt
    ldy #0
@copy:
    cpy $61
    beq @descriptor
    lda (nxt),y
    sta (ptr),y
    iny
    bne @copy
@descriptor:
    ldy #0
    lda $61
    sta ($47),y
    iny
    lda ptr
    sta ($47),y
    iny
    lda ptr+1
    sta ($47),y
    jmp string_release

string_concat:
    lda $0D
    and $12
    bpl @type
    clc
    lda $61
    adc $69
    jcs long_error
    pha
    jsr string_allocate
    ; FAC/ARG carry descriptor indices as well as cached bytes. GC may have
    ; moved either operand; reacquire their pointers after allocation.
    ldx $66
    lda $01,x
    sta nxt
    lda $02,x
    sta nxt+1
    ldy #0
@left:
    tya
    cmp $00,x
    beq @right_pointer
    lda (nxt),y
    sta (ptr),y
    iny
    bne @left
@right_pointer:
    tya
    clc
    adc ptr
    sta dst
    lda ptr+1
    adc #0
    sta dst+1
    ldx $6E
    lda $01,x
    sta nxt
    lda $02,x
    sta nxt+1
    ldy #0
@right:
    tya
    cmp $00,x
    beq @result
    lda (nxt),y
    sta (dst),y
    iny
    bne @right
@result:
    pla
    sta $61
    copy_pointer ptr, $62
    jsr string_release
    jmp string_temporary
@type:
    jmp type_error

string_compare:
    lda $0D
    and $12
    bpl @type
    ldx $66
    lda $01,x
    sta ptr
    lda $02,x
    sta ptr+1
    lda $00,x
    sta size
    ldx $6E
    lda $01,x
    sta nxt
    lda $02,x
    sta nxt+1
    lda $00,x
    sta delta
    ldy #0
@byte:
    cpy size
    beq @left_end
    cpy delta
    beq @greater
    lda (ptr),y
    cmp (nxt),y
    bne @order
    iny
    bne @byte
@left_end:
    cpy delta
    beq @equal
    bcc @less
@order:
    bcc @less
@greater:
    lda #1
    bne @done
@less:
    lda #4
    bne @done
@equal:
    lda #2
@done:
    pha
    jsr string_release
    lda #0
    sta $0D
    pla
    rts
@type:
    jmp type_error

; A is the length. PTR returns the new immutable allocation. Collection is
; attempted once; it cannot invent space or borrow RAM outside the BASIC heap.
string_allocate:
    pha
    jsr allocation_candidate
    bcs @fit
    jsr string_collect
    jsr allocation_candidate
    jcc memory_error
@fit:
    copy_pointer ptr, $33
    pla
    rts
allocation_candidate:
    tsx
    sec
    lda $33
    sbc $0103,x                 ; length beneath this helper's real JSR frame
    sta ptr
    lda $34
    sbc #0
    sta ptr+1
    bcc @no
    cmp $32
    bne @order
    lda ptr
    cmp $31
@order:
    rts
@no:
    clc
    rts

; Descriptor iterator. Scalars precede temporary roots; array roots are added
; by the array implementation. PTR points at a three-byte descriptor, carry
; clear means the bounded iterator has finished.
string_roots_begin:
    copy_pointer $2D, $4B
    lda #0
    sta $4D
    rts
string_root_next:
    lda $4D
    cmp #3
    jeq @temporary
    cmp #2
    jeq @element
    cmp #1
    jeq @array
@scalar:
    lda $4B
    cmp $2F
    bne @record
    lda $4C
    cmp $30
    jeq @arrays
@record:
    copy_pointer $4B, ptr
    clc
    lda $4B
    adc #7
    sta $4B
    lda $4C
    adc #0
    sta $4C
    cmp $30
    jcc @type
    jne bad_program
    lda $4B
    cmp $2F
    jcc @type
    jne bad_program
@type:
    ldy #0
    lda (ptr),y
    bmi @scalar
    iny
    lda (ptr),y
    bpl @scalar
    increment ptr
    increment ptr
    sec
    rts
@arrays:
    lda #1
    sta $4D
@array:
    lda $4B
    cmp $31
    bne @header
    lda $4C
    cmp $32
    jeq @temporaries
@header:
    copy_pointer $4B, ptr
    lda $50
    pha
    lda $51
    pha
    jsr array_span
    copy_pointer nxt, $4E
    pla
    sta $51
    pla
    sta $50
    ldy #0
    lda (ptr),y
    bmi @skip_array
    iny
    lda (ptr),y
    bpl @skip_array
    ldy #4
    lda (ptr),y
    asl
    clc
    adc #5
    clc
    adc ptr
    sta $4B
    lda ptr+1
    adc #0
    sta $4C
    lda #2
    sta $4D
    jmp @element
@skip_array:
    copy_pointer $4E, $4B
    jmp @array
@element:
    lda $4B
    cmp $4E
    bne @string_element
    lda $4C
    cmp $4F
    bne @string_element
    lda #1
    sta $4D
    jmp @array
@string_element:
    copy_pointer $4B, ptr
    clc
    lda $4B
    adc #3
    sta $4B
    lda $4C
    adc #0
    sta $4C
    cmp $4F
    jcc @array_root
    jne bad_program
    lda $4B
    cmp $4E
    jcc @array_root
    jne bad_program
@array_root:
    sec
    rts
@temporaries:
    lda #3
    sta $4D
    lda #$19
    sta $4B
    lda #0
    sta $4C
@temporary:
    lda $4B
    cmp $16
    bcs @end
    sta ptr
    lda #0
    sta ptr+1
    clc
    lda $4B
    adc #3
    sta $4B
    sec
    rts
@end:
    clc
    rts

string_collect:
    copy_pointer $37, $50       ; exclusive old-address ceiling
    copy_pointer $37, $56       ; next compacted destination
@block:
    lda #0
    sta $52
    sta $53
    sta $54
    jsr string_roots_begin
@root:
    jsr string_root_next
    jcc @selected
    ldy #0
    lda (ptr),y
    beq @root
    sta delta
    iny
    lda (ptr),y
    sta $58
    iny
    lda (ptr),y
    sta $59
    cmp $32
    bcc @root
    bne @heap
    lda $58
    cmp $31
    bcc @root
@heap:
    lda $59
    cmp $38
    bcc @below_limit
    bne @root
    lda $58
    cmp $37
    bcs @root
@below_limit:
    lda $59
    cmp $51
    bcc @above_best
    bne @root
    lda $58
    cmp $50
    bcs @root
@above_best:
    lda $59
    cmp $53
    bcc @root
    bne @better
    lda $58
    cmp $52
    bcc @root
    bne @better
    lda delta
    cmp $54
    bcc @root
@better:
    copy_pointer $58, $52
    lda delta
    sta $54
    jmp @root
@selected:
    lda $54
    jeq @finished
    ; Validate a complete live allocation before moving it.
    clc
    adc $52
    sta $58
    lda $53
    adc #0
    cmp $38
    jcc @copy
    jne bad_program
    lda $58
    cmp $37
    jcc @copy
    jne bad_program
@copy:
    sec
    lda $56
    sbc $54
    sta $56
    lda $57
    sbc #0
    sta $57
    ldy $54
@move:
    dey
    lda ($52),y
    sta ($56),y
    tya
    bne @move
    jsr string_roots_begin
@relocate:
    jsr string_root_next
    bcc @next
    ldy #1
    lda (ptr),y
    cmp $52
    bne @relocate
    iny
    lda (ptr),y
    cmp $53
    bne @relocate
    lda $57
    sta (ptr),y
    dey
    lda $56
    sta (ptr),y
    jmp @relocate
@next:
    copy_pointer $52, $50
    jmp @block
@finished:
    copy_pointer $56, $33
    rts

; A is the one-argument function token, after its closing parenthesis.
string_function:
    cmp #$C4
    jeq string_format
    cmp #$C7
    jeq string_character
    pha
    jsr require_string
    jsr string_refresh
    pla
    cmp #$C5
    jeq string_value
    cmp #$C6
    beq @ascii
    lda $61
    pha
    jsr string_release
    pla
    jmp byte_number
@ascii:
    lda $61
    jeq quantity_error
    copy_pointer $62, ptr
    ldy #0
    lda (ptr),y
    pha
    jsr string_release
    pla
    jmp byte_number

string_character:
    jsr require_numeric
    lda #15
    jsr numeric_call
    lda num+1
    jne quantity_error
    lda num
    pha
    lda #1
    jsr string_allocate
    pla
    ldy #0
    sta (ptr),y
    lda #1
    sta $61
    copy_pointer ptr, $62
    jmp string_temporary

string_format:
    jsr require_numeric
    lda #5
    jsr numeric_call
    ldx #0
@length:
    lda $0100,x
    beq @copy
    inx
    cpx #32
    bcc @length
    jmp bad_program
@copy:
    dex                         ; STR$ keeps the leading sign space, not PRINT's tail
    stx $61
    lda #0
    sta $62
    lda #1
    sta $63
    jsr string_temporary
    lda $61
    jsr string_allocate
    jsr string_refresh
    copy_pointer $62, nxt
    ldy #0
@byte:
    cpy $61
    beq @result
    lda (nxt),y
    sta (ptr),y
    iny
    bne @byte
@result:
    copy_pointer ptr, $62
    jsr string_release
    jmp string_temporary

string_value:
    lda $11
    pha
    lda $12
    pha
    lda txt
    pha
    lda txt+1
    pha
    lda $66
    pha
    clc
    lda $62
    adc $61
    sta $11
    lda $63
    adc #0
    sta $12
    copy_pointer $62, txt
    lda #0
    sta size
@space:
    lda txt
    cmp $11
    bne @peek
    lda txt+1
    cmp $12
    beq @zero
@peek:
    jsr peek
    cmp #32
    bne @sign
    jsr consume
    jmp @space
@sign:
    cmp #'-'
    bne @plus
    lda #$FF
    sta size
    jsr consume
    jmp @number
@plus:
    cmp #'+'
    bne @number
    jsr consume
@number:
    lda size
    pha
    lda #16
    jsr gate_math
    cpx #6
    beq @bad_literal
    cpx #0
    jne numeric_error
    jmp @signed
@bad_literal:
    lda #0
    sta $61
@signed:
    pla
    sta $66
    jmp @return
@zero:
    lda #0
    sta $61
    sta $66
@return:
    pla
    sta $16
    pla
    sta txt+1
    pla
    sta txt
    pla
    sta $12
    pla
    sta $11
    lda #0
    sta $0D
    rts

substring_arguments:
    jsr require_string
    lda $66
    pha
    jsr comma
    jsr byte_value
    pha
    tsx
    lda $0103,x                 ; function token, beneath root and first number
    cmp #$CA
    bne @single_length
    lda num
    jeq quantity_error          ; MID$ positions start at one
    jsr skip_spaces
    cmp #','
    bne @whole_tail
    jsr consume
    jsr byte_value
    jmp @length
@whole_tail:
    lda #255
@length:
    sta size
    pla
    sec
    sbc #1
    sta delta
    jmp @close
@single_length:
    pla
    sta size
    lda #0
    sta delta
@close:
    jsr skip_spaces
    cmp #')'
    jne syntax_error
    jsr consume
    pla
    sta $66
    pla
    cmp #$C9
    bne @range
    jsr string_refresh
    lda $61
    sec
    sbc size
    bcc @range
    sta delta
@range:
    jsr string_refresh
    lda $61
    sec
    sbc delta
    bcs :+
    lda #0
:
    cmp size
    bcc :+
    lda size
:
    pha
    lda delta
    pha
    tsx
    lda $0102,x
    jsr string_allocate
    jsr string_refresh
    pla
    clc
    adc $62
    sta nxt
    lda $63
    adc #0
    sta nxt+1
    pla
    sta $61
    ldy #0
@byte:
    cpy $61
    beq @result
    lda (nxt),y
    sta (ptr),y
    iny
    bne @byte
@result:
    copy_pointer ptr, $62
    jsr string_release
    jmp string_temporary
.segment "PAGE0"
