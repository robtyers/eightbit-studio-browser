; Independently authored from published C64 interfaces, not ROM implementation.
; Public RAM remains guest-owned. Every action below is ordinary guest 6510 code.
.setcpu "6502"
.include "branches.inc"
.export reset_entry, irq_handler, nmi_handler
.export gate_banner, gate_irq, gate_nmi, gate_recovery
.export gate_math
.export gate_open, gate_close, gate_chkin, gate_chkout
.export device_absent, file_not_open, clrchn_default
.export editor_capture
.export main_entry
.import warm_entry, banner_entry
.import math_entry
.import quantity_error
.import error_default, tokenise_default, list_keyword_default, dispatch_default, factor_default
.import open_default, close_default, chkin_default, chkout_default
.segment "CODE"
reset_entry:
    sei
    cld
    ldx #$F8
    txs
    lda #$37
    sta $01                     ; latch before direction, so KERNAL stays visible
    lda #$2F
    sta $00
    jsr ramtas
    jsr restor
    jsr ioinit
    jsr cint
    jsr gate_banner             ; original page 1 returns through immutable gate
    jmp ($A000)

ramtas:
    lda #0
    ldx #2
@clear:
    sta $00,x
    inx
    bne @clear
    ldx #0
@page:
    sta $0200,x
    sta $0300,x
    inx
    bne @page
    lda #$00
    sta $0281
    lda #$08
    sta $0282
    sta $2C
    sta $2E
    sta $30
    sta $32
    lda #1
    sta $2B
    lda #3
    sta $2D
    sta $2F
    sta $31
    lda #$A0
    sta $0284
    sta $34
    sta $38
    lda #$00
    sta $0283
    sta $33
    sta $37
    sta $0801
    sta $0802
    sta $41
    sta $3F
    sta $40
    lda #8
    sta $42
    lda #$3C
    sta $B2
    lda #3
    sta $B3
    lda #10
    sta $0289
    lda #64
    sta $C5
    lda #<key_plain
    sta $F5
    lda #>key_plain
    sta $F6
    lda #1
    sta $02A6
    lda #$4C
    sta $0310
    lda #<quantity_error
    sta $0311
    lda #>quantity_error
    sta $0312
    ldx #11
@basic_vectors:
    lda basic_vectors,x
    sta $0300,x
    dex
    bpl @basic_vectors
    ldx #11
@chrget:
    lda chrget_original,x
    sta $73,x
    dex
    bpl @chrget
    jsr clrchn_default
    clc
    rts

ioinit:
    lda #$7F
    sta $DC0D
    sta $DD0D
    lda $DC0D
    lda $DD0D
    lda #0
    sta $DC0E
    sta $DC0F
    sta $DD0E
    sta $DD0F
    sta $DC03
    sta $D01A
    ldx #$18
@sid:
    sta $D400,x
    dex
    bpl @sid
    lda #$FF
    sta $DC00
    sta $DC02
    lda #3
    sta $DD00
    lda #$3F
    sta $DD02
    lda #<irq_default
    sta $0314
    lda #>irq_default
    sta $0315
    lda #<irq_return
    sta $0316
    sta $0318
    lda #>irq_return
    sta $0317
    sta $0319
    lda #$24                    ; 16421 clocks per approximately 60 Hz jiffy
    sta $DC04
    lda #$40
    sta $DC05
    lda #$81
    sta $DC0D
    lda #$11
    sta $DC0E
    clc
    rts

; Preserve the published user IRQ register/status stack shape. The synthetic
; return frame delivers a callback's RTI to the page-gate epilogue. The final
; gate RTI then consumes the real hardware frame. No host register substitution.
irq_handler:
    php
    pha
    txa
    pha
    tya
    pha
    tsx
    inc $0105,x
    bne :+
    inc $0106,x
:
    lda $0107,x
    sta $0104,x                 ; expose the real interrupted status to user RTI
    and #$10
    bne @brk
    jmp ($0314)
@brk:
    jmp ($0316)
nmi_handler:
    php
    pha
    txa
    pha
    tya
    pha
    tsx
    inc $0105,x
    bne :+
    inc $0106,x
:
    lda $0107,x
    sta $0104,x
    jmp ($0318)
irq_default:
    lda $DC0D
    and #1
    beq irq_return
    jsr udtim
    jsr scnkey
irq_return:
    pla
    tay
    pla
    tax
    pla
    rti

udtim:
    inc $A2
    bne @done
    inc $A1
    bne @done
    inc $A0
@done:
    lda $A0
    cmp #$4F
    bne @return
    lda $A1
    cmp #$1A
    bne @return
    lda $A2
    cmp #$00
    bne @return
    sta $A0
    sta $A1
@return:
    rts
settim:
    php
    sei
    sta $A2
    stx $A1
    sty $A0
    plp
    rts
rdtim:
    php
    sei
    lda $A2
    ldx $A1
    ldy $A0
    plp
    rts

scnkey:
    txa
    pha
    tya
    pha
    jmp @scan
@modifiers:
    lda #0
    sta $028D
    lda #$7F
    sta $DC00
    lda $DC01
    and #4
    bne :+
    lda #4
    sta $028D
:
    lda $DC01
    and #32
    bne :+
    lda $028D
    ora #2
    sta $028D
:
    lda #$FD
    sta $DC00
    lda $DC01
    and #$80
    beq @shift
    lda #$BF
    sta $DC00
    lda $DC01
    and #$10
    bne @translated
@shift:
    lda $028D
    ora #1
    sta $028D
@translated:
    rts
@scan:
    jsr @modifiers
    lda $028D
    and #3
    cmp #3
    bne @remember_shift
    cmp $028E
    beq @remember_shift
    lda $0291
    bmi @remember_shift
    lda $D018
    eor #2
    sta $D018
    lda #3
@remember_shift:
    sta $028E
    ldy $C5
    lda #64
    sta $C5
    ldx #0
@key:
    lda key_rows,x
    sta $DC00
    lda $DC01
    and key_bits,x
    jne @next
    lda key_plain,x
    jeq @next
    stx $C5
    cpy $C5
    bne @fresh
    lda $028A
    and #$40
    jne @done
    lda $028A
    bmi @repeat
    lda key_plain,x
    cmp #20
    beq @repeat
    cmp #17
    beq @repeat
    cmp #29
    beq @repeat
    cmp #32
    jne @done
@repeat:
    lda $028C
    beq @rate
    dec $028C
    jmp @done
@rate:
    dec $028B
    jne @done
    lda #4
    sta $028B
    jmp @translate
@fresh:
    lda #16
    sta $028C
    lda #4
    sta $028B
@translate:
    ; Sample modifiers with the accepted key, rather than an earlier row scan.
    ; IRQ entry may precede the physical SHIFT/key transition by many cycles.
    jsr @modifiers
    lda $028D
    and #4
    bne @control
    lda $028D
    and #2
    bne @commodore
    lda $028D
    and #1
    beq @plain
    lda key_shift,x
    jmp @queue
@control:
    lda key_plain,x
    cmp #'9'
    bne :+
    lda #18
    jmp @queue
:
    cmp #'0'
    bne :+
    lda #146
    jmp @queue
:
    cmp #'1'
    bcc @control_letter
    cmp #'9'
    bcs @control_letter
    sec
    sbc #'1'
    tax
    lda colour_controls,x
    jmp @queue
@control_letter:
    cmp #'A'
    bcc @plain
    cmp #('Z'+1)
    bcs @plain
    and #31
    jmp @queue
@commodore:
    lda key_plain,x
    cmp #'1'
    bcc @graphic_letter
    cmp #'9'
    bcs @graphic_letter
    sec
    sbc #('1'-8)
    tax
    lda colour_controls,x
    jmp @queue
@graphic_letter:
    cmp #'A'
    bcc @plain
    cmp #('Z'+1)
    bcs @plain
    sec
    sbc #'A'
    tax
    lda commodore_letters,x
    jmp @queue
@plain:
    lda key_plain,x
@queue:
    ldy $C6
    cpy $0289
    bcs @done
    cpy #10
    bcs @done
    sta $0277,y
    inc $C6
    jmp @done
@next:
    inx
    cpx #64
    jne @key
@done:
    lda #$FF
    sta $DC00
    pla
    tay
    pla
    tax
    rts
getin_default:
    php
    sei
    txa
    pha
    tya
    pha
    lda #0
    ldx $C6
    beq @result
    lda $0277
    pha
    dex
    stx $C6
    ldy #0
@shift:
    cpy $C6
    beq @value
    lda $0278,y
    sta $0277,y
    iny
    bne @shift
@value:
    pla
@result:
    sta $D7
    pla
    tay
    pla
    tax
    plp
    lda $D7
    rts
chrin_default:
    jmp editor_read

cint:
    lda #$1B
    sta $D011
    lda #8
    sta $D016
    lda #$14
    sta $D018
    lda #0
    sta $D015
    lda #6
    sta $D020
    sta $D021
    lda #14
    sta $0286
    lda #4
    sta $0288
    jmp clear_screen
chrout_default:
    pha
    txa
    pha
    tya
    pha
    tsx
    lda $0103,x
    cmp #34
    beq @quote
    cmp #13
    jeq @existing
    cmp #148
    bne :+
    lda $D0
    bne @insert
    lda #148
:
    lda $D4
    beq @controls
    lda $0103,x
    cmp #20
    jeq @existing
    cmp #32
    bcc @quoted_low
    cmp #128
    bcc @controls
    cmp #160
    bcs @controls
    clc
    adc #64
    jmp @screen_store
@quoted_low:
    ora #128
    jmp @screen_store
@quote:
    lda $D4
    eor #1
    sta $D4
@controls:
    lda $0103,x
    ldx #15
@colour:
    cmp colour_controls,x
    beq @set_colour
    dex
    bpl @colour
    cmp #18
    beq @reverse
    cmp #146
    beq @normal
    cmp #29
    beq @right
    cmp #157
    beq @left
    cmp #17
    beq @down
    cmp #145
    beq @up
    cmp #14
    beq @lower
    cmp #142
    beq @upper
    jmp @existing
@set_colour:
    stx $0286
    jmp @done
@insert:
    jsr editor_insert
    jmp @done
@reverse:
    lda #$80
    sta $C7
    jmp @done
@normal:
    lda #0
    sta $C7
    jmp @done
@right:
    inc $D3
    lda $D3
    cmp #40
    bcc @moved
    jsr newline
    jmp @done
@left:
    lda $D3
    bne @left_column
    lda $D6
    beq @moved
    dec $D6
    lda #40
    sta $D3
@left_column:
    dec $D3
    jmp @done
@down:
    lda $D3
    pha
    jsr newline
    pla
    sta $D3
    jmp @done
@up:
    lda $D6
    beq @moved
    dec $D6
@moved:
    jmp @done
@lower:
    lda $D018
    ora #2
    sta $D018
    jmp @done
@upper:
    lda $D018
    and #$FD
    sta $D018
    jmp @done
@existing:
    cmp #147
    bne :+
    jsr clear_screen
    jmp @done
:
    cmp #19
    bne :+
    lda #0
    sta $D3
    sta $D6
    jmp @done
:
    cmp #13
    bne :+
    lda #0
    sta $D4
    sta $C7
    jsr newline
    jmp @done
:
    cmp #20
    bne @print
    lda $D3
    bne @delete_column
    lda $D6
    beq @done
    dec $D6
    lda #40
    sta $D3
@delete_column:
    dec $D3
    jsr editor_delete
    jmp @done
@print:
    cmp #32
    bcc @done
    cmp #128
    bcc @ascii
    cmp #160
    jcc @done
    cmp #$FF
    beq @pi
    cmp #192
    bcc @graphic
    sec
    sbc #128
    jmp @store
@pi:
    lda #94
    jmp @store
@graphic:
    sec
    sbc #64
    jmp @store
@ascii:
    cmp #64
    bcc @store
    cmp #96
    bcc @letter
    sec
    sbc #32
    jmp @store
@letter:
    sec
    sbc #64
@store:
    eor $C7
@screen_store:
    ldx $D8
    beq :+
    dec $D8
:
    pha
    jsr cursor_address
    pla
    ldy $D3
    sta ($D1),y
    lda $0286
    sta ($F3),y
    inc $D3
    lda $D3
    cmp #40
    bcc @done
    jsr wrap_line
@done:
    jsr cursor_address
    pla
    tay
    pla
    tax
    pla
    rts
.segment "RODATA"
basic_vectors:
.word error_default,warm_entry,tokenise_default,list_keyword_default,dispatch_default,factor_default
; Public queue values from original physical C=+letter fixtures, never ROM data.
commodore_letters:
.byte 176,191,188,172,177,187,165,180,162,181,161,182,167,170,185,175,171,178,174,163,184,190,179,189,183,173
colour_controls:
.byte 144,5,28,159,156,30,31,158,129,149,150,151,152,153,154,155
.segment "CODE"
cursor_address:
    lda #0
    sta $D1
    lda $0288
    sta $D2
    ldx $D6
    beq @done
@row:
    clc
    lda $D1
    adc #40
    sta $D1
    bcc :+
    inc $D2
:
    dex
    bne @row
@done:
    lda $D1
    sta $F3
    sec
    lda $D2
    sbc $0288
    clc
    adc #$D8
    sta $F4
    rts
newline:
    lda #$80
    bne next_screen_row
wrap_line:
    lda #0
next_screen_row:
    pha
    lda #0
    sta $D3
    inc $D6
    lda $D6
    cmp #25
    jcc @done
    ; SAL is the published tape/screen-scroll pointer; EAL and USER supply
    ; bounded colour/source views. No user-reserved scratch page is used.
    lda #0
    sta $D1
    sta $AE
    lda $0288
    sta $D2
    lda #$D8
    sta $AF
    lda #40
    sta $AC
    sta $F3
    lda $0288
    sta $AD
    lda #$D8
    sta $F4
    ldx #3
@page:
    ldy #0
@copy:
    lda ($AC),y
    sta ($D1),y
    lda ($F3),y
    sta ($AE),y
    iny
    bne @copy
    inc $AD
    inc $D2
    inc $F4
    inc $AF
    dex
    bne @page
    ldy #0
@tail:
    lda ($AC),y
    sta ($D1),y
    lda ($F3),y
    sta ($AE),y
    iny
    cpy #192
    bcc @tail
    lda #$C0
    sta $D1
    sta $AE
    ldy #0
@blank:
    lda #32
    sta ($D1),y
    lda $0286
    sta ($AE),y
    iny
    cpy #40
    bcc @blank
    ldx #0
@links:
    lda $DA,x
    and #$80
    sta $D7
    lda $D9,x
    and #$7F
    ora $D7
    sta $D9,x
    inx
    cpx #24
    bcc @links
    lda #24
    sta $D6
@done:
    jsr cursor_address
    ldx $D6
    pla
    ora $D2
    sta $D9,x
    rts
clear_screen:
    lda #0
    sta $D1
    sta $F3
    lda $0288
    sta $D2
    lda #$D8
    sta $F4
    ldx #3
@page:
    ldy #0
@byte:
    lda #32
    sta ($D1),y
    lda $0286
    sta ($F3),y
    iny
    bne @byte
    inc $D2
    inc $F4
    dex
    bne @page
    ldy #0
@tail:
    lda #32
    sta ($D1),y
    lda $0286
    sta ($F3),y
    iny
    cpy #232
    bcc @tail
    lda #0
    sta $D3
    sta $D6
    sta $C7
    sta $D4
    sta $D0
    sta $D1
    lda $0288
    sta $D2
    ldx #0
@links:
    lda $D2
    ora #$80
    sta $D9,x
    clc
    lda $D1
    adc #40
    sta $D1
    bcc :+
    inc $D2
:
    inx
    cpx #25
    bcc @links
    jmp cursor_address

restor:
    ldx #31
@copy:
    lda defaults,x
    sta $0314,x
    dex
    bpl @copy
    rts
vector:
    php
    sei
    stx $C3
    sty $C4
    ldy #0
    bcc @set
@read:
    lda $0314,y
    sta ($C3),y
    iny
    cpy #32
    bne @read
    plp
    rts
@set:
    lda ($C3),y
    sta $0314,y
    iny
    cpy #32
    bne @set
    plp
    rts
memtop:
    bcc @set
    ldx $0283
    ldy $0284
    rts
@set:
    stx $0283
    sty $0284
    rts
membot:
    bcc @set
    ldx $0281
    ldy $0282
    rts
@set:
    stx $0281
    sty $0282
    rts
setmsg:
    sta $9D
    rts
settmo:
    sta $0285
    rts
readst:
    lda $90
    rts
setlfs:
    sta $B8
    stx $BA
    sty $B9
    rts
setnam:
    sta $B7
    stx $BB
    sty $BC
    rts
clrchn_default:
    lda #0
    sta $99
    lda #3
    sta $9A
    clc
    rts
clall_default:
    lda #0
    sta $98
    jmp clrchn_default
device_absent:
    lda #$80
    sta $90
    lda #5
    sec
    rts
file_not_open:
    lda #3
    sec
    rts
stop_default:
    lda $C5
    cmp #63
    rts
screen:
    ldx #40
    ldy #25
    rts
plot:
    bcc @set
    ldx $D6
    ldy $D3
    rts
@set:
    cpx #25
    bcs @invalid
    cpy #40
    bcs @invalid
    stx $D6
    sty $D3
    jsr cursor_address
    clc
    rts
@invalid:
    sec
    rts
iobase:
    ldx #0
    ldy #$DC
    rts

main_entry:
    jmp ($0302)

; The fixed RAM entry has its required LDA operand at $7A/$7B. Classification
; stays in original fixed ROM, leaving every RAM byte patchable by guest code.
chrgot_original:
    cmp #32
    beq @space
    cmp #'0'
    bcc @other
    cmp #('9'+1)
    bcs @other
    clc
    rts
@other:
    sec
    ora #0                      ; return the character's zero state, preserving C
    rts
@space:
    jmp $73
chrget_original:
    .byte $E6,$7A,$D0,2,$E6,$7B,$AD,0,2,$4C
    .word chrgot_original
defaults:
.word irq_default, irq_return, irq_return
.word open_entry, close_entry, chkin_entry, chkout_entry
.word clrchn_default, chrin_default, chrout_default, stop_default
.word getin_default, clall_default, file_not_open, device_absent, device_absent
open_entry:
    jsr gate_open
    rts
close_entry:
    jsr gate_close
    rts
chkin_entry:
    jsr gate_chkin
    rts
chkout_entry:
    jsr gate_chkout
    rts

; Matrix legends and electrical masks are public hardware facts. These byte
; tables are generated here independently, not recovered firmware tables.
key_plain:
.byte 20,13,29,0,0,0,0,17
.byte "3WA4ZSE",0
.byte "5RD6CFTX"
.byte "7YG8BHUV"
.byte "9IJ0MKON"
.byte "+PL-.:@,"
.byte 92,"*;",19,0,"=",94,"/"
.byte "1",0,0,"2 ",0,"Q",3
key_shift:
.byte 148,13,157,0,0,0,0,145
.byte 35,215,193,36,218,211,197,0
.byte 37,210,196,38,195,198,212,216
.byte 39,217,199,40,194,200,213,214
.byte 41,201,202,48,205,203,207,206
.byte 43,208,204,45,62,58,59,60
.byte 92,42,93,147,0,61,255,63
.byte 33,0,0,34,32,0,209,3
key_rows:
.repeat 64, index
.byte $FF ^ (1 << (index / 8))
.endrepeat
key_bits:
.repeat 64, index
.byte 1 << (index .mod 8)
.endrepeat

.include "editor.inc"
.macro call_gate label, page, target, ending
label:
    php
    pha
    lda #page
    sta $DE00
    pla
    plp
    jsr target
    php
    pha
    lda #0
    sta $DE01
    pla
    plp
    ending
.endmacro
.segment "GATES"
call_gate gate_banner, 1, banner_entry, rts
.res 10, $EA
call_gate gate_irq, 0, irq_handler, rti
.res 10, $EA
call_gate gate_nmi, 0, nmi_handler, rti
.res 10, $EA
gate_recovery:
    php
    pha
    lda #0
    sta $DE02
    pla
    plp
    jmp main_entry
.res 20, $EA
call_gate gate_math, 2, math_entry, rts
.res 10, $EA
call_gate gate_open, 2, open_default, rts
.res 10, $EA
call_gate gate_close, 2, close_default, rts
.res 10, $EA
call_gate gate_chkin, 2, chkin_default, rts
.res 10, $EA
call_gate gate_chkout, 2, chkout_default, rts

.segment "JUMPS"
    jmp cint
    jmp ioinit
    jmp ramtas
    jmp restor
    jmp vector
    jmp setmsg
    jmp device_absent
    jmp device_absent
    jmp memtop
    jmp membot
    jmp scnkey
    jmp settmo
    jmp device_absent
    jmp device_absent
    jmp device_absent
    jmp device_absent
    jmp device_absent
    jmp device_absent
    jmp readst
    jmp setlfs
    jmp setnam
    jmp ($031A)
    jmp ($031C)
    jmp ($031E)
    jmp ($0320)
    jmp ($0322)
    jmp ($0324)
    jmp ($0326)
    jmp ($0330)
    jmp ($0332)
    jmp settim
    jmp rdtim
    jmp ($0328)
    jmp ($032A)
    jmp ($032C)
    jmp udtim
    jmp screen
    jmp plot
    jmp iobase
.segment "VECTORS"
.word gate_nmi, reset_entry, gate_irq
