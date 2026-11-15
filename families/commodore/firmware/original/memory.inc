.segment "CODE"
; Independently authored guest memory and machine-code interfaces. All accesses
; take the ordinary CPU bus path, including I/O and writes beneath visible ROM.
comma:
    jsr skip_spaces
    cmp #','
    jne syntax_error
    jmp consume
address_value:
    jsr expression
    jsr require_numeric
    lda #15
    jmp numeric_call
byte_value:
    jsr address_value
    lda num+1
    jne quantity_error
    lda num
    rts
poke_statement:
    jsr address_value
    lda num
    pha
    lda num+1
    pha
    jsr comma
    jsr byte_value
    pha
    jsr check_end
    pla
    tax
    pla
    sta ptr+1
    pla
    sta ptr
    txa
    ldy #0
    sta (ptr),y
    jmp dispatch
wait_statement:
    jsr address_value
    lda num
    pha
    lda num+1
    pha
    jsr comma
    jsr byte_value
    pha
    jsr skip_spaces
    cmp #','
    bne @no_xor
    jsr consume
    jsr byte_value
    jmp @ready
@no_xor:
    lda #0
@ready:
    pha
    jsr check_end
    pla
    sta delta
    pla
    sta size
    pla
    sta ptr+1
    pla
    sta ptr
    ldy #0
@poll:
    lda (ptr),y
    eor delta
    and size
    beq @poll
    jmp dispatch
sys_statement:
    jsr address_value
    jsr check_end
    lda $030F
    pha
    lda $030C
    ldx $030D
    ldy $030E
    plp
    jsr call_sys
    php
    sta $030C
    stx $030D
    sty $030E
    pla
    sta $030F
    cld
    jmp dispatch
call_sys:
    jmp (num)

.segment "PAGE0"
