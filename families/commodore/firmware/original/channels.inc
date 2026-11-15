; Original keyboard/screen logical channels only. No host or attached peripheral
; is invented. Public file tables are bounded to their ten published records.
open_default:
    lda $98
    cmp #10
    bcc @find
    lda #1
    sec
    rts
@find:
    lda $B8
    jsr channel_find
    bcc @device
    lda #2
    sec
    rts
@device:
@open:
    ldx $98
    lda $B8
    sta $0259,x
    lda $BA
    sta $0263,x
    lda $B9
    sta $026D,x
    inc $98
    lda $BA
    beq @ready
    cmp #3
    jne device_absent
@ready:
    lda #0
    sta $90
    clc
    rts
channel_find:
    ldx #0
@entry:
    cpx $98
    bcs @missing
    cpx #10
    bcs @missing
    cmp $0259,x
    beq @found
    inx
    jmp @entry
@found:
    sec
    rts
@missing:
    clc
    rts
close_default:
    jsr channel_find
    bcc @done
    dec $98
@move:
    cpx $98
    bcs @done
    lda $025A,x
    sta $0259,x
    lda $0264,x
    sta $0263,x
    lda $026E,x
    sta $026D,x
    inx
    jmp @move
@done:
    jmp clrchn_default
chkin_default:
    txa
    jsr channel_find
    jcc file_not_open
    lda $0263,x
    beq @keyboard
    cmp #3
    jne device_absent
    lda #6
    sec
    rts
@keyboard:
    sta $99
    clc
    rts
chkout_default:
    txa
    jsr channel_find
    jcc file_not_open
    lda $0263,x
    cmp #3
    beq @screen
    cmp #0
    jne device_absent
    lda #7
    sec
    rts
@screen:
    sta $9A
    clc
    rts
