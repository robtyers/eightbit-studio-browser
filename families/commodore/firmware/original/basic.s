; Independently authored guest 6510 BASIC front end, Apache-2.0.
; Public token/RAM/link facts: C64 PRG memory map and BASIC V2 format.
; No reference firmware/interpreter code or host evaluator is used.
; BASIC values and evaluation execute entirely in original guest code.
.setcpu "6502"
.include "branches.inc"
.export warm_entry
.export quantity_error
.export error_default, tokenise_default, list_keyword_default, dispatch_default, factor_default
.import gate_recovery, gate_math
.import editor_capture

; Only published BASIC input/utility/temporary workspace is used. No user RAM,
; tape buffer, spare page-two storage or host continuation stores language data.
num = $14                      ; public integer temporary
ptr = $22                      ; public utility pointers
cur = $24
inidx = $0B                    ; input buffer pointer
mode = $0F                     ; DATA / LIST quote scanning
inlen = $4B                    ; published BASIC temporary area $4B-$60
over = $4C
outidx = $4D
wordlen = $4E
token = $4F
nxt = $50
at = $52
oldend = $54
newend = $56
src = $58
dst = $5A
finish = $5C
previous = $5E
flags = $60
txt = $7A                      ; current BASIC text position
size = $08
delta = $07

.macro increment pointer
    inc pointer
    bne :+
    inc pointer+1
:
.endmacro
.macro decrement pointer
    lda pointer
    bne :+
    dec pointer+1
:
    dec pointer
.endmacro
.macro copy_pointer source, target
    lda source
    sta target
    lda source+1
    sta target+1
.endmacro

.segment "PAGE0"
warm_entry:
    ldx #$F8
    txs                         ; non-local guest recovery owns its stack reset
    lda #0
    sta $3E
prompt_entry:
    cli
    lda #$FF
    sta $3A                     ; direct mode, never an invented programme line
    sta $39
    lda #$19
    sta $16
    lda #$80
    sta $9D
    jsr ensure_newline
    ldx #<ready_text
    ldy #>ready_text
    jsr message
    jmp read_line
.segment "CODE"
read_line:
    lda #0
    sta inlen
    sta over
    lda #2
    sta $D0
@key:
    jsr $FFE4
    jeq @key
    cmp #3
    jeq break_error
    cmp #13
    jeq @enter
    cmp #20
    jeq @delete
    cmp #32
    jcc @control
    ldx inlen
    cpx #80
    jcs @long
    inc inlen
@control:
    jsr $FFD2
    jmp @key
@long:
    lda #1
    sta over                    ; consume to CR, leaving the stored programme intact
    jmp @key
@delete:
    ldx inlen
    beq @delete_screen
    dec inlen
@delete_screen:
    jsr $FFD2
    jmp @key
@enter:
    jsr editor_capture
    stx inlen
    tya
    ora over
    sta over
    lda #0
    sta $D0
    lda #13
    jsr $FFD2
    ldx inlen
    lda over
    beq :+
    jmp long_error
:
    jsr tokenise
    lda #0
    sta txt
    lda #2
    sta txt+1
    jsr skip_spaces
    jeq read_line
    cmp #'0'
    jcc @direct
    cmp #('9'+1)
    jcs @direct
    jsr decimal_line
    jsr edit_line
    jmp read_line
@direct:
    lda #$19
    sta $16
    jmp dispatch

; In-place compression is bounded by the original 80-byte input. Keyword order
; puts every longer prefix first (INPUT#/INPUT, PRINT#/PRINT, GOTO/GO).
tokenise:
    jmp ($0304)
tokenise_default:
    lda #0
    sta inidx
    sta outidx
    sta mode
@next:
    ldx inidx
    lda $0200,x
    bne :+
    ldx outidx
    sta $0200,x
    stx inlen
    rts
:
    pha
    lda mode
    and #4
    jne @literal
    pla
    cmp #34
    jne @not_quote
    pha
    lda mode
    eor #1
    sta mode
    jmp @literal
@not_quote:
    pha
    lda mode
    and #1
    jne @literal
    pla
    cmp #':'
    jne @not_colon
    pha
    lda #0
    sta mode
    jmp @literal
@not_colon:
    pha
    lda mode
    and #2
    jne @literal
    pla
    cmp #'?'
    jne @keywords
    lda #$99
    jmp @one
@literal:
    pla
@one:
    inc inidx
    jmp @emit
@keywords:
    lda #<keywords
    sta ptr
    lda #>keywords
    sta ptr+1
    lda #$80
    sta token
@candidate:
    ldy #0
    lda (ptr),y
    sta wordlen
    ldx inidx
    iny
@compare:
    lda $0200,x
    cmp #$C1
    bcc @ordinary
    cmp #$DB
    bcs @ordinary
    and #$7F
    cmp (ptr),y
    bne @miss
    inx
    jmp @matched
@ordinary:
    lda (ptr),y
    cmp $0200,x
    jne @miss
    inx
    iny
    cpy wordlen
    jcc @compare
    jeq @compare
@matched:
    stx inidx
    lda token
    cmp #$83
    bne :+
    lda #2
    sta mode
:
    lda token
    cmp #$8F
    bne :+
    lda #4
    sta mode
:
    lda token
    jmp @emit
@miss:
    sec
    lda ptr
    adc wordlen                ; add length plus its leading byte
    sta ptr
    bcc :+
    inc ptr+1
:
    inc token
    lda token
    cmp #$CC
    jcc @candidate
    ldx inidx
    lda $0200,x
    jmp @one
@emit:
    ldx outidx
    sta $0200,x
    inc outidx
    jmp @next

.segment "PAGE0"
peek:
    ldy #0
    lda (txt),y
    rts
consume:
    increment txt
    jmp peek
skip_spaces:
    decrement txt
    jmp $73                     ; original patchable guest CHRGET/CHRGOT path

; Decimal line numbers, not an approximate general BASIC numeric evaluator.
; Check before multiply/add: 6399*10+9 is the largest accepted value.
decimal_line:
    lda #0
    sta num
    sta num+1
    jsr peek
    cmp #'0'
    jcc @invalid
    cmp #('9'+1)
    jcs @invalid
@digit:
    sec
    sbc #'0'
    sta delta
    lda num+1
    cmp #$19
    jcc @multiply
    jne @invalid
    lda num
    cmp #$00                    ; 6400 = $1900
    jcs @invalid
@multiply:
    copy_pointer num, ptr
    asl num
    rol num+1
    asl num
    rol num+1
    clc
    lda num
    adc ptr
    sta num
    lda num+1
    adc ptr+1
    sta num+1
    asl num
    rol num+1
    clc
    lda num
    adc delta
    sta num
    bcc :+
    inc num+1
:
    jsr consume
    cmp #'0'
    jcc @done
    cmp #('9'+1)
    jcc @digit
@done:
    rts
@invalid:
    jmp syntax_error

; Every edit, RUN and LIST validates the complete existing linked list before
; reading bodies or moving any bytes. Increasing links make hostile cycles total.
validate_bounds:
    lda $2C
    cmp #8
    jcc @bad
    cmp #$A0
    jcs @bad
    cmp #8
    bne :+
    lda $2B
    jeq @bad
:
    lda $38
    cmp #$A0
    bcc :+
    jne @bad
    lda $37
    jne @bad
:
    clc
    lda $2B
    adc #2
    sta ptr
    lda $2C
    adc #0
    cmp $38
    jcc @ok
    jne @bad
    lda ptr
    cmp $37
    jeq @ok
    jcs @bad
@ok:
    rts
@bad:
    jmp bad_program
validate_program:
    jsr validate_bounds
    lda $2E
    cmp $38
    bcc :+
    jne @bad
    lda $2D
    cmp $37
    bcc :+
    jne @bad
:
    sec
    lda $2D
    sbc #2
    sta finish
    lda $2E
    sbc #0
    sta finish+1
    jcc @bad
    copy_pointer $2B, cur
    lda #0
    sta flags
@line:
    lda cur+1
    cmp finish+1
    jcc @record
    jne @bad
    lda cur
    cmp finish
    jcc @record
    jne @bad
    ldy #0
    lda (cur),y
    iny
    ora (cur),y
    jne @bad
    rts
@bad:
    jmp bad_program
@record:
    ; Require at least five bytes before reading a line header.
    clc
    lda cur
    adc #5
    sta nxt
    lda cur+1
    adc #0
    cmp finish+1
    bcc :+
    jne @bad
    lda nxt
    cmp finish
    beq :+
    jcs @bad
:
    ldy #0
    lda (cur),y
    sta nxt
    iny
    lda (cur),y
    sta nxt+1
    cmp finish+1
    bcc :+
    jne @bad
    lda nxt
    cmp finish
    beq :+
    jcs @bad
:
    sec
    lda nxt
    sbc cur
    sta size
    lda nxt+1
    sbc cur+1
    jne @bad
    lda size
    cmp #5
    jcc @bad
    ldy #3
    lda (cur),y
    cmp #$FA
    jcs @bad
    ldx flags
    jeq @number
    cmp previous+1
    jcc @bad
    jne @number
    dey
    lda (cur),y
    cmp previous
    jcc @bad
    jeq @bad
@number:
    ldy #2
    lda (cur),y
    sta previous
    iny
    lda (cur),y
    sta previous+1
    lda #1
    sta flags
    ldy #4
@body:
    lda (cur),y
    jeq @zero
    iny
    cpy size
    jcc @body
    jmp bad_program
@zero:
    iny
    cpy size
    jne @bad
    copy_pointer nxt, cur
    jmp @line

; Validated search returns the first line >= num, carry set only on equality.
find_line:
    copy_pointer $2B, cur
@next:
    ldy #1
    lda (cur),y
    jeq @not_equal
    ldy #3
    lda (cur),y
    cmp num+1
    jcc @advance
    jne @not_equal
    dey
    lda (cur),y
    cmp num
    jcc @advance
    jeq @equal
@not_equal:
    clc
    rts
@equal:
    sec
    rts
@advance:
    ldy #0
    lda (cur),y
    tax
    iny
    lda (cur),y
    sta cur+1
    stx cur
    jmp @next

edit_line:
    jsr skip_spaces
    jsr validate_program
    jsr find_line
    copy_pointer cur, at
    copy_pointer cur, oldend
    jcc @size                   ; copy_pointer uses only LDA/STA, preserving carry
    ldy #0
    lda (cur),y
    sta oldend
    iny
    lda (cur),y
    sta oldend+1
@size:
    lda #0
    sta size
    jsr peek
    jeq @measure
    sec
    lda inlen
    sbc txt
    clc
    adc #5
    sta size
@measure:
    ; New end = old VARTAB - removed bytes + inserted bytes. No writes until fit.
    sec
    lda oldend
    sbc at
    sta delta
    sec
    lda $2D
    sbc delta
    sta newend
    lda $2E
    sbc #0
    sta newend+1
    clc
    lda newend
    adc size
    sta newend
    lda newend+1
    adc #0
    sta newend+1
    cmp $38
    jcc @fit
    jne @full
    lda newend
    cmp $37
    jeq @fit
    jcc @fit
@full:
    jmp memory_error
@fit:
    copy_pointer oldend, src
    clc
    lda at
    adc size
    sta dst
    lda at+1
    adc #0
    sta dst+1
    lda dst+1
    cmp src+1
    jcc @left
    jne @right
    lda dst
    cmp src
    jcc @left
    jeq @write
@right:
    copy_pointer $2D, src
    copy_pointer newend, dst
@backward:
    decrement src
    decrement dst
    ldy #0
    lda (src),y
    sta (dst),y
    lda src
    cmp oldend
    jne @backward
    lda src+1
    cmp oldend+1
    jne @backward
    jmp @write
@left:
    ldy #0
@forward:
    lda (src),y
    sta (dst),y
    increment src
    increment dst
    lda src
    cmp $2D
    jne @forward
    lda src+1
    cmp $2E
    jne @forward
@write:
    lda size
    jeq @relink
    ldy #2
    lda num
    sta (at),y
    iny
    lda num+1
    sta (at),y
    iny
    ldx txt
@body:
    lda $0200,x
    sta (at),y
    jeq @relink
    iny
    inx
    jne @body
@relink:
    copy_pointer newend, $2D
    copy_pointer $2B, cur
    sec
    lda newend
    sbc #2
    sta finish
    lda newend+1
    sbc #0
    sta finish+1
@link:
    lda cur
    cmp finish
    jne @scan
    lda cur+1
    cmp finish+1
    jeq clear_variables
@scan:
    ldy #4
@terminator:
    lda (cur),y
    jeq @found
    iny
    jne @terminator
@found:
    iny
    tya
    clc
    adc cur
    sta nxt
    lda cur+1
    adc #0
    sta nxt+1
    ldy #0
    lda nxt
    sta (cur),y
    iny
    lda nxt+1
    sta (cur),y
    copy_pointer nxt, cur
    jmp @link
clear_variables:
    copy_pointer $2D, $2F
    copy_pointer $2D, $31
    copy_pointer $37, $33
    lda #0
    sta $3E                     ; editing / CLR invalidates a future CONT
    jsr $FFE7
    jmp restore_data

check_end:
    jsr skip_spaces
    jeq @ok
    cmp #':'
    jeq @ok
    jmp syntax_error
@ok:
    rts
dispatch:
    jsr skip_spaces
    jmp ($0308)
dispatch_default:
    jsr $FFE1
    bne :+
    jmp break_error
:
    jsr skip_spaces
    jeq next_line
    cmp #':'
    bne :+
    jsr consume
    jmp dispatch
:
    cmp #$99
    bne :+
    jsr consume
    jmp print_statement
:
    cmp #$88
    bne :+
    jsr consume
    jmp assignment
:
    cmp #$8B
    bne :+
    jsr consume
    jmp if_statement
:
    cmp #$8D
    bne :+
    jsr consume
    jmp gosub_statement
:
    cmp #$8E
    bne :+
    jsr consume
    jmp return_statement
:
    cmp #$89
    bne :+
    jsr consume
    jmp goto_statement
:
    cmp #$CB
    bne :+
    jsr consume
    jsr skip_spaces
    cmp #$A4
    jne syntax_error
    jsr consume
    jmp goto_statement
:
    cmp #$8A
    bne :+
    jsr consume
    jmp run_statement
:
    cmp #$9B
    bne :+
    jsr consume
    jmp list_statement
:
    cmp #$A2
    bne :+
    jsr consume
    jmp new_statement
:
    cmp #$9C
    bne :+
    jsr consume
    jsr check_end
    jsr clear_variables
    ldx #$F8
    txs
    jmp dispatch
:
    cmp #$8F
    jeq skip_line
    cmp #$9F
    bne :+
    jsr consume
    jmp open_statement
:
    cmp #$A0
    bne :+
    jsr consume
    jmp close_statement
:
    cmp #$98
    bne :+
    jsr consume
    jmp file_print_statement
:
    cmp #$9D
    bne :+
    jsr consume
    jmp command_statement
:
    cmp #$84
    bne :+
    jsr consume
    jmp file_input_statement
:
    cmp #$93
    beq @load
    cmp #$94
    beq @load
    cmp #$95
    bne :+
@load:
    pha
    jsr consume
    jmp load_save_statement
:
    cmp #$96
    bne :+
    jsr consume
    jmp function_definition
:
    cmp #$9A
    bne :+
    jsr consume
    jmp continue_statement
:
    cmp #$A1
    bne :+
    jsr consume
    jmp get_statement
:
    cmp #$85
    bne :+
    jsr consume
    jmp input_statement
:
    cmp #$86
    bne :+
    jsr consume
    jmp dim_statement
:
    cmp #$91
    bne :+
    jsr consume
    jmp on_statement
:
    cmp #$87
    bne :+
    jsr consume
    jmp read_statement
:
    cmp #$8C
    bne :+
    jsr consume
    jsr check_end
    jsr restore_data
    jmp dispatch
:
    cmp #$81
    bne :+
    jsr consume
    jmp for_statement
:
    cmp #$82
    bne :+
    jsr consume
    jmp next_statement
:
    cmp #$97
    bne :+
    jsr consume
    jmp poke_statement
:
    cmp #$92
    bne :+
    jsr consume
    jmp wait_statement
:
    cmp #$9E
    bne :+
    jsr consume
    jmp sys_statement
:
    cmp #$83
    bne :+
    jsr consume
    jmp data_statement
:
    cmp #$80
    bne :+
    jsr consume
    jsr check_end
    lda #0
    jmp stop_program
:
    cmp #$90
    bne :+
    jsr consume
    jsr check_end
    lda #1
    jmp stop_program
:
    cmp #$80
    jcs unsupported_error
    cmp #'A'
    jcc syntax_error
    cmp #('Z'+1)
    jcs syntax_error
    jmp assignment
skip_line:
    jsr peek
    jeq next_line
    jsr consume
    jmp skip_line
next_line:
    lda $3A
    cmp #$FF
    bne :+
    lda $3E
    jne prompt_entry
    jmp gate_recovery
:
    ldy #0
    lda (cur),y
    tax
    iny
    lda (cur),y
    sta cur+1
    stx cur
start_line:
    ldy #1
    lda (cur),y
    bne :+
    jmp gate_recovery
:
    ldy #2
    lda (cur),y
    sta $39
    iny
    lda (cur),y
    sta $3A
    clc
    lda cur
    adc #4
    sta txt
    lda cur+1
    adc #0
    sta txt+1
    jmp dispatch

print_statement:
    jsr skip_spaces
    jeq @newline
    cmp #':'
    jeq @newline
@item:
    cmp #$A3
    jeq @spacing
    cmp #$A6
    jeq @spacing
    cmp #';'
    jeq @separator
    cmp #','
    jeq @zone
    jsr expression
    lda $0D
    bmi @string_value
    jsr numeric_output
    jmp @after_value
@string_value:
    jsr string_output
@after_value:
    jsr skip_spaces
    jeq @newline
    cmp #':'
    jeq @newline
    jmp @item
@spacing:
    pha
    jsr consume
    jsr byte_value
    pha
    jsr skip_spaces
    cmp #')'
    jne syntax_error
    jsr consume
    pla
    sta num
    pla
    cmp #$A6
    beq @spaces
    lda num
@column:
    cmp #40
    bcc @target
    sec
    sbc #40
    jmp @column
@target:
    sta num
    cmp $D3
    bcs @difference
    lda #13
    jsr $FFD2
@difference:
    sec
    lda num
    sbc $D3
    sta num
@spaces:
    lda num
    beq @after_value
    lda #32
    jsr $FFD2
    dec num
    jmp @spaces
@zone:
    lda #32
    jsr $FFD2
    lda $D3
    jeq @separator
    cmp #10
    jeq @separator
    cmp #20
    jeq @separator
    cmp #30
    jne @zone
@separator:
    jsr consume
    jsr skip_spaces
    jeq @suppressed
    cmp #':'
    jne @item
@suppressed:
    jmp dispatch
@newline:
    lda #13
    jsr $FFD2
    jmp dispatch

data_statement:
    lda #0
    sta mode
@byte:
    jsr peek
    jeq @done
    cmp #34
    bne :+
    pha
    lda mode
    eor #1
    sta mode
    pla
:
    cmp #':'
    jne @next
    lda mode
    jeq @done
@next:
    jsr consume
    jmp @byte
@done:
    jmp dispatch

goto_statement:
    jsr skip_spaces
    jsr decimal_line
    jsr check_end
goto_target:
    jsr validate_program
    jsr find_line
    bcs :+
    jmp undefined_error
:
    lda #0
    sta $9D
    jmp start_line
run_statement:
    jsr skip_spaces
    jeq @begin
    cmp #':'
    jeq @begin
    jsr decimal_line
    jsr check_end
    jsr validate_program
    jsr find_line
    jcs @run
    jmp undefined_error
@begin:
    jsr validate_program
    copy_pointer $2B, cur
@run:
    jsr clear_variables
    ldx #$F8
    txs
    lda #0
    sta $9D
    jmp start_line
new_statement:
    jsr check_end
    jsr validate_bounds
    copy_pointer $2B, ptr
    ldy #0
    lda #0
    sta (ptr),y
    iny
    sta (ptr),y
    clc
    lda $2B
    adc #2
    sta $2D
    lda $2C
    adc #0
    sta $2E
    jsr clear_variables
    jmp gate_recovery

list_statement:
    lda #0
    sta $49
    sta $4A
    lda #$FF
    sta $47
    lda #$F9
    sta $48
    jsr skip_spaces
    jeq @range_ready
    cmp #':'
    jeq @range_ready
    cmp #$AB
    beq @upper
    jsr decimal_line
    copy_pointer num, $49
    copy_pointer num, $47
    jsr skip_spaces
    cmp #$AB
    bne @range_ready
    lda #$FF
    sta $47
    lda #$F9
    sta $48
@upper:
    jsr consume
    jsr skip_spaces
    jeq @range_ready
    cmp #':'
    jeq @range_ready
    jsr decimal_line
    copy_pointer num, $47
@range_ready:
    jsr check_end
    jsr validate_program
    copy_pointer $2B, cur
@line:
    jsr $FFE1
    bne :+
    jmp break_error
:
    ldy #1
    lda (cur),y
    bne :+
    jmp @finished
:
    ldy #2
    lda (cur),y
    sta num
    iny
    lda (cur),y
    sta num+1
    cmp $4A
    jcc @advance
    bne @upper_bound
    lda num
    cmp $49
    jcc @advance
@upper_bound:
    lda num+1
    cmp $48
    jcc @print_line
    jne @finished
    lda num
    cmp $47
    jeq @print_line
    jcs @finished
@print_line:
    jsr print_unsigned
    lda #32
    jsr $FFD2
    lda #0
    sta mode
    ldy #4
@body:
    lda (cur),y
    jeq @eol
    pha
    lda mode
    and #4
    jne @literal
    pla
    cmp #34
    jne @not_quote
    pha
    lda mode
    eor #1
    sta mode
    jmp @literal
@not_quote:
    pha
    lda mode
    and #1
    jne @literal
    pla
    cmp #':'
    bne :+
    pha
    lda #0
    sta mode
    jmp @literal
:
    pha
    lda mode
    and #2
    jne @literal
    pla
    cmp #$80
    jcc @emit
    cmp #$CC
    jcs @emit
    cmp #$83
    bne :+
    pha
    lda #2
    sta mode
    pla
:
    cmp #$8F
    bne :+
    pha
    lda #4
    sta mode
    pla
:
    jsr list_keyword
    jmp @next
@literal:
    pla
@emit:
    jsr $FFD2
@next:
    iny
    jne @body
@eol:
    lda #13
    jsr $FFD2
@advance:
    ldy #0
    lda (cur),y
    tax
    iny
    lda (cur),y
    sta cur+1
    stx cur
    jmp @line
@finished:
    lda $3E
    jne prompt_entry
    jmp gate_recovery
list_keyword:
    jmp ($0306)
list_keyword_default:
    sec
    sbc #$80
    tax
    tya
    pha
    lda #<keywords
    sta ptr
    lda #>keywords
    sta ptr+1
@word:
    ldy #0
    lda (ptr),y
    sta wordlen
    cpx #0
    jeq @print
    sec
    adc ptr
    sta ptr
    bcc :+
    inc ptr+1
:
    dex
    jmp @word
@print:
    iny
@letter:
    lda (ptr),y
    jsr $FFD2
    iny
    cpy wordlen
    jcc @letter
    jeq @letter
    pla
    tay
    rts

print_unsigned:
    ldx #0
    stx flags
@place:
    ldy #0
@subtract:
    lda num+1
    cmp powers_hi,x
    jcc @digit
    jne @take
    lda num
    cmp powers_lo,x
    jcc @digit
@take:
    sec
    lda num
    sbc powers_lo,x
    sta num
    lda num+1
    sbc powers_hi,x
    sta num+1
    iny
    jne @subtract
@digit:
    tya
    jne @output
    ldy flags
    jne @output
    cpx #4
    jne @next
@output:
    ora #'0'
    jsr $FFD2
    inc flags
@next:
    inx
    cpx #5
    jcc @place
    rts
ensure_newline:
    lda $D3
    beq :+
    lda #13
    jsr $FFD2
:
    rts
message:
    stx ptr
    sty ptr+1
    ldy #0
@byte:
    lda (ptr),y
    beq :+
    jsr $FFD2
    iny
    jne @byte
:
    rts

syntax_error:
    ldx #<syntax_text
    ldy #>syntax_text
    jmp error
long_error:
    ldx #<long_text
    ldy #>long_text
    jmp error
memory_error:
    ldx #<memory_text
    ldy #>memory_text
    jmp error
bad_program:
    ldx #<bad_text
    ldy #>bad_text
    jmp error
undefined_error:
    ldx #<undefined_text
    ldy #>undefined_text
    jmp error
unsupported_error:
    ldx #<unsupported_text
    ldy #>unsupported_text
    jmp error
break_error:
    lda $3A
    cmp #$FF
    beq @direct
    lda #1
    jmp stop_program
@direct:
    ldx #<break_text
    ldy #>break_text
    jmp error_print
error:
    stx ptr
    sty ptr+1
    ldx #0
@find:
    lda error_messages,x
    cmp ptr
    bne @next
    lda error_messages+1,x
    cmp ptr+1
    beq @code
@next:
    inx
    inx
    cpx #62
    bcc @find
    ldx #58                     ; original BAD PROGRAM extension
@code:
    txa
    lsr
    clc
    adc #1
    tax
    jmp ($0300)
error_default:
    cpx #0
    beq @unknown
    cpx #32
    bcc @known
@unknown:
    ldx #30
@known:
    txa
    sec
    sbc #1
    asl
    tax
    ldy error_messages+1,x
    lda error_messages,x
    tax
error_print:
    jsr ensure_newline
    jsr message
    lda $3A
    cmp #$FF
    jeq @direct
    ldx #<in_text
    ldy #>in_text
    jsr message
    copy_pointer $39, num
    jsr print_unsigned
@direct:
    lda #13
    jsr $FFD2
    jmp gate_recovery

.segment "RODATA"
powers_lo:
.lobytes 10000,1000,100,10,1
powers_hi:
.hibytes 10000,1000,100,10,1
ready_text: .byte "READY.",13,0
syntax_text: .byte "?SYNTAX ERROR",0
long_text: .byte "?STRING TOO LONG ERROR",0
memory_text: .byte "?OUT OF MEMORY ERROR",0
bad_text: .byte "?BAD PROGRAM ERROR",0
undefined_text: .byte "?UNDEF'D STATEMENT ERROR",0
unsupported_text: .byte "?UNIMPLEMENTED ERROR",0
break_text: .byte "BREAK",0
in_text: .byte " IN ",0
keywords:
.include "keywords.inc"
error_messages:
.word files_full_text,file_open_text,file_closed_text,file_missing_text,device_text
.word file_input_text,file_output_text,file_name_text,file_device_text,next_text
.word syntax_text,return_text,data_text,quantity_text,overflow_text,memory_text
.word undefined_text,subscript_text,redim_text,division_text,direct_input_text
.word type_text,long_text,bad_input_text,formula_text,continue_text,function_error_text
.word verify_error_text,load_error_text,bad_text,unsupported_text
verify_error_text: .byte "?VERIFY ERROR",0
load_error_text: .byte "?LOAD ERROR",0

.segment "PAGE0"
.include "expressions.inc"

.include "control.inc"
.include "memory.inc"
.include "loops.inc"
.include "data.inc"
.include "strings.inc"
.include "arrays.inc"
.include "time.inc"
.include "input.inc"
.include "continue.inc"
.include "functions.inc"
.include "files.inc"
