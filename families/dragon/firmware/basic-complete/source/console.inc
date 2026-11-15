BOOT
        orcc    #$50
        clra
        tfr     a,dp
        ; Select normal RAM before pushing any return address or reading its workspace.
        clr     $ffda
        clr     $ffdd
        lds     #$7fff
        lda     RSTFLAG
        cmpa    #$55
        bne     COLD
        ldx     RSTVEC
        lda     ,x
        cmpa    #$12
        bne     COLD
        jmp     ,x
COLD
        ; BOOT has selected RAM, masked interrupts and established DP/S.
        bsr     BASIC_HW_INIT
        ldx     #0
        jsr     SETUP_CLEAR
        lds     STACKTOP
        leas    1,s
        jsr     CLS
        ldx     #BANNER
        jsr     PUTS
        andcc   #$af
DIRECT
        ldb     #2
        stb     ERROR_CODE
        jsr     DIRECT_LINE
        ldx     #PROMPT
        jsr     PUTS
        jsr     READLINE
        bcs     DIRECT_ERROR
        lda     ,x
        beq     DIRECT
        cmpa    #'0
        blo     DIRECT_PENDING
        cmpa    #'9
        bhi     DIRECT_PENDING
        jsr     STORE_SOURCE_LINE
        bcs     DIRECT_ERROR
        jsr     CMD_CLEAR_VARIABLES
        lds     STACKTOP
        leas    1,s
        bra     DIRECT
DIRECT_PENDING
        jsr     TOKENISE
        bcs     DIRECT_ERROR
        ldx     #TOKENBUF
        jmp     EXECUTE_DIRECT
DIRECT_ERROR
        jsr     BASIC_ERROR
        bra     DIRECT
WARM
        nop
        jsr     STRING_TEMP_RESET
        bsr     BASIC_HW_INIT
        clr     DEVNUM
        clr     FORMAT_BUFFER
        lds     STACKTOP
        leas    1,s
        jsr     DIRECT_LINE
        andcc   #$af
        bra     DIRECT

BASIC_HW_INIT
        leay    BASIC_HW_READY,pcr
        bra     HW_INIT
BASIC_HW_READY
        ldd     #$3537
        sta     $ff03
        stb     $ff23
        rts
HW_INIT
        ldx     #$ffc0
HW_SAM
        clr     ,x
        leax    2,x
        cmpx    #$ffe0
        blo     HW_SAM
        clr     $ffdd
        clr     $ffc9
        ldu     #$ff00
        ldx     #HW_REGISTERS
HW_REGISTER
        ldd     ,x++
        stb     a,u
        cmpx    #HW_REGISTERS_END
        blo     HW_REGISTER
        jmp     ,y
HW_REGISTERS
        fcb     $03,4,$02,$ff,$03,0,$02,$ff
        fcb     $01,$34,$03,$34,$21,4,$20,2
        fcb     $21,0,$20,$fe,$21,$34,$23,4
        fcb     $22,0,$23,0,$22,$f8,$23,$34
HW_REGISTERS_END

SETUP
        ; A public software reinitialisation can follow an interrupted
        ; interpreter. Discard only our private owners and continuations.
        ldx     #$03d9
        ; Cold boot enters here with X=0 and clears the full working area.
SETUP_CLEAR
CLEAR_WORK
        clr     ,x+
        cmpx    #$0400
        blo     SETUP_CLEAR
        jsr     STRING_TEMP_RESET
        ldd     #$1e01
        std     TXTTAB
        addd    #2
        jsr     VARIABLE_ENDS
        clr     $1e00
        clr     $1e01
        clr     $1e02
        ldd     #$7ffe
        std     MEMTOP
        std     $74
        std     FRETOP
        subd    #200
        std     STACKTOP
        jsr     DATA_RESET
        jsr     GFX_INIT
        jsr     RND_INIT
        jsr     DIRECT_LINE
        lda     #$55
        sta     RSTFLAG
        ldd     #WARM
        std     RSTVEC
        lda     #$ff
        sta     $148
        sta     $149
        ldd     #$010d
        std     $14a
        lda     #10
        sta     $14c
        ldd     #$da5c
        std     $95
        ldd     #$1074
        std     $99
        ldd     #$8400
        std     $9b
        lda     #32
        sta     $6d
        lda     #16
        sta     $6a
        ldd     #$0134
        std     USR_TAB
        ldx     #$0100
VECTOR_LOOP
        lda     #$7e
        sta     ,x+
        ldd     #IRQ_RETURN
        std     ,x++
        cmpx    #$0112
        blo     VECTOR_LOOP
        ldd     #FRAME_IRQ
        std     $010d
        ldd     #CARTRIDGE_FIRQ
        std     $0110
        ldx     #KEYOLD
        lda     #$ff
ROLLOVER_INIT
        sta     ,x+
        cmpx    #$015a
        blo     ROLLOVER_INIT
        ldx     #$015e
        ldd     #$3912
HOOK_LOOP
        std     ,x++
        stb     ,x+
        cmpx    #$01b0
        blo     HOOK_LOOP
        ldx     #SCANNER_TEMPLATE
        ldy     #$009f
SCANNER_INSTALL
        ldb     #SCANNER_TEMPLATE_END-SCANNER_TEMPLATE
        jsr     COPY_BYTES
        ldd     #EXEC_ERROR
        std     $9d
        ldd     #INPUTBUF-1
        std     TXT_PTR
        ldx     #TOKEN_STUBS_ROM
        ldy     #$0120
TOKEN_STUBS_INSTALL
        ldb     #20
        jsr     COPY_BYTES
        ldx     #$0134
USR_TABLE_INIT
        ldd     #CMD_UNSUPPORTED
        std     ,x++
        cmpx    #$0148
        blo     USR_TABLE_INIT
        rts
FRAME_IRQ
        lda     $ff02
        ldd     TIMER+1
        addd    #1
        std     TIMER+1
        bcc     IRQ_RETURN
        inc     TIMER
IRQ_RETURN
        rti

; CART is the PIA1 CB1 source. Other FIRQ sources retain their own acknowledgement.
CARTRIDGE_FIRQ
        ifdef   STUDIO_EXTENDED
        ; The Studio profile has no cartridge slot. Acknowledge its PIA source
        ; without entering $C000, which contains this profile's BASIC code.
        ; FIRQ has a short hardware frame, so retain the accumulator explicitly.
        pshs    a
        lda     $ff22
        puls    a
        rti
        else
        tst     $ff23
        bpl     IRQ_RETURN
        clr     >RSTFLAG
        lda     #$34
        sta     $ff23
        ; Allow a nominal 2^20 E cycles for cartridge contacts to settle. This
        ; original loop does not claim analogue or cycle-exact OEM equivalence.
        ldx     #0
CARTRIDGE_SETTLE
        nop
        nop
        nop
        nop
        leax    -1,x
        bne     CARTRIDGE_SETTLE
        jmp     $c000
        endc

CLS
        pshs    a,x
        lda     #$60
        bra     CLS_BEGIN
CLS_COLOUR
        pshs    a,x
CLS_BEGIN
        ldx     #$0400
CLS_LOOP
        sta     ,x+
        cmpx    #$0600
        blo     CLS_LOOP
        ldx     #$0400
        stx     CURSOR
        puls    a,x,pc
PUTS
        lda     ,x+
        beq     PUTS_END
        bsr     CHROUT
        bra     PUTS
PUTS_END
        rts
; A bounded transient RAM sink also serves detokenised editor text. The target
; advances and LENGTH counts remaining bytes; cassette mode keeps its own count.
FORMAT_WRITE
        pshs    x
        tst     FORMAT_LENGTH
        beq     FORMAT_WRITE_END
        ldx     FORMAT_TARGET
        sta     ,x+
        stx     FORMAT_TARGET
        dec     FORMAT_LENGTH
FORMAT_WRITE_END
        puls    x,pc


CHROUT
        tst     FORMAT_BUFFER
        ble     CHROUT_DEVICE
        jmp     FORMAT_WRITE
CHROUT_DEVICE
        pshs    d,x,y,u
        jsr     OUTPUT_HOOK
        ; The public selector is classified after the output hook: FF is
        ; cassette, FD discards, 00..7E is screen, and the rest is printer.
        ldb     DEVNUM
        incb
        beq     CHROUT_CASSETTE
        cmpb    #$fe
        beq     CHROUT_RETURN
        tstb
        bmi     PRINTER_CHARACTER
        bra     SCREEN_CHARACTER
CHROUT_CASSETTE
        ldb     $78
        decb
        beq     CHROUT_RETURN
CHROUT_FILE
        jsr     FILE_ASCII_BYTE
CHROUT_RETURN
        puls    d,x,y,u,pc

; Filename bytes use the generic output hook but always the text-screen sink.
SCREENOUT_HOOKED
        pshs    d,x,y,u
        jsr     OUTPUT_HOOK
        bra     SCREEN_CHARACTER
SCREENOUT_RAW
SCREENOUT
        pshs    d,x,y,u
SCREEN_CHARACTER
        ldx     CURSOR
        cmpa    #13
        beq     SCREEN_CR
        cmpa    #8
        beq     SCREEN_BS
        cmpa    #32
        blo     SCREEN_END
        cmpa    #128
        bhs     SCREEN_CELL
        cmpa    #96
        blo     SCREEN_UPPER
        anda    #$1f
        bra     SCREEN_CELL
SCREEN_UPPER
        anda    #$3f
        ora     #$40
SCREEN_CELL
        sta     ,x+
        bra     SCREEN_SAVE
SCREEN_CR
        ; CR clears the remainder of this row before advancing or scrolling.
        tfr     x,d
        andb    #31
        negb
        addb    #32
        lda     #$60
SCREEN_CR_CLEAR
        sta     ,x+
        decb
        bne     SCREEN_CR_CLEAR
SCREEN_SAVE
        cmpx    #$0600
        blo     SCREEN_CURSOR
        ldx     #$0420
        ldy     #$0400
SCROLL
        lda     ,x+
        sta     ,y+
        cmpx    #$0600
        blo     SCROLL
        ldx     #$05e0
        lda     #$60
        ldb     #32
SCROLL_CLEAR
        sta     ,y+
        decb
        bne     SCROLL_CLEAR
SCREEN_CURSOR
        stx     CURSOR
SCREEN_END
        puls    d,x,y,u,pc
SCREEN_BS
        cmpx    #$0400
        bls     SCREEN_END
        leax    -1,x
        lda     #$60
        sta     ,x
        bra     SCREEN_CURSOR
PRINTEROUT
        pshs    d,x,y,u
PRINTER_CHARACTER
        ; Do not mutate the public column while the printer is busy.
        bsr     PRINT_READY
        cmpa    #13
        beq     PRINT_CR
        bsr     PRINT_BYTE
        cmpa    #32
        blo     PRINT_RETURN
        inc     $9c
        ldb     $9c
        cmpb    $9b
        blo     PRINT_RETURN
        clr     $9c
        tst     $148
        bne     PRINT_RETURN
        bra     PRINT_EOL
PRINT_CR
        tst     $9c
        bne     PRINT_EOL
        ; The blank is a physical byte, without a second width transition.
        lda     #32
        bsr     PRINT_BYTE
PRINT_EOL
        clr     $9c
        ldx     #$14b
        ldb     $14a
        beq     PRINT_RETURN
PRINT_EOL_BYTE
        lda     ,x+
        bsr     PRINT_BYTE
        decb
        bne     PRINT_EOL_BYTE
PRINT_RETURN
        puls    d,x,y,u,pc
PRINT_BYTE
        pshs    b
        bsr     PRINT_READY
        sta     $ff02
        ldb     $ff20
        andb    #$fd
        stb     $ff20
        nop
        orb     #2
        stb     $ff20
        puls    b,pc
PRINT_READY
        ldb     $ff22
        bitb    #1
        bne     PRINT_READY
        rts

BLINK
        pshs    a,x
        dec     $8f
        bne     BLINK_END
        ldx     CURSOR
        lda     ,x
        eora    #$40
        sta     ,x
        lda     #50
        sta     $8f
BLINK_END
        puls    a,x,pc
KEYIN
        pshs    b,x,y
        clr     KEYSAVE
        clr     SHIFTED
        lda     #$7f
        sta     $ff02
        lda     $ff00
        bita    #64
        bne     KEY_NO_SHIFT
        inc     SHIFTED
KEY_NO_SHIFT
        ldx     #KEYOLD
        clr     COL
        lda     #$fe
        sta     SCAN
KEY_COLUMN
        lda     SCAN
        sta     $ff02
        lda     $ff00
        coma
        anda    #$7f
        ldb     COL
        cmpb    #7
        bne     KEY_NEW
        anda    #$3f
KEY_NEW
        tfr     a,b
        anda    ,x
        comb
        stb     ,x+
        tst     KEYSAVE
        bne     KEY_NEXT
        tsta
        beq     KEY_NEXT
        clrb
KEY_ROW
        lsra
        bcs     KEY_FOUND
        addb    #8
        bra     KEY_ROW
KEY_FOUND
        addb    COL
        ; The first 43 matrix positions are contiguous ASCII ranges; only
        ; navigation/control keys need a table. Shift handling remains below.
        cmpb    #43
        bhs     KEY_CONTROL
        tfr     b,a
        adda    #'0
        andb    #$fc
        cmpb    #12
        bne     KEY_SAVE
        suba    #16
        bra     KEY_SAVE
KEY_CONTROL
        subb    #43
        ldy     #KEYMAP
        tst     SHIFTED
        beq     KEY_CONTROL_MAP
        cmpb    #7
        bhs     KEY_CONTROL_MAP
        leay    12,y
KEY_CONTROL_MAP
        lda     b,y
KEY_SAVE
        sta     KEYSAVE
KEY_NEXT
        inc     COL
        coma
        rol     SCAN
        bcs     KEY_COLUMN
        lda     #$ff
        sta     $ff02
        lda     KEYSAVE
        jsr     IS_UPPER_NAME
        bcc     KEY_SYMBOL
        ldb     SHIFTED
        tst     $149
        bne     KEY_ALPHA
        eorb    #1
KEY_ALPHA
        tstb
        beq     KEY_RETURN
        adda    #32
        bra     KEY_RETURN
KEY_SYMBOL
        tst     SHIFTED
        beq     KEY_RETURN
        cmpa    #',
        blo     KEY_RETURN
        cmpa    #';
        bls     KEY_SHIFT_PUNCTUATION
        cmpa    #'@
        bne     KEY_RETURN
        lda     #19
        bra     KEY_RETURN
KEY_SHIFT_PUNCTUATION
        ; Shift swaps the $2C..$3B punctuation/digit groups by bit four.
        eora    #$10
        cmpa    #32
        bne     KEY_RETURN
        com     $149
        clra
KEY_RETURN
        sta     $87
        puls    b,x,y,pc
; One input hook call belongs to one blocking character request. Matrix
; polling and INKEY$ retain the unhooked public keyboard primitive.
READ_CHARACTER equ DATA_BYTE
READ_CHARACTER_WAIT
        jsr     $bbe5
        beq     READ_CHARACTER_WAIT
        clr     $70
        rts
READLINE
        ldx     #INPUTBUF
READLINE_INTO
        pshs    x
        jsr     LINE_HOOK
        clr     LINE_INVALID
LINE_NEXT
        jsr     READ_CHARACTER
        clr     $70
        cmpa    #3
        beq     LINE_CANCEL
        cmpa    #13
        beq     LINE_FINISH
        cmpa    #8
        beq     LINE_BACK
        cmpa    #12
        beq     LINE_CLEAR
        cmpa    #32
        blo     LINE_NEXT
        ; The keyboard editor retains 249 characters. Excess input is ignored;
        ; editing still applies to the accepted prefix. EDIT has its own limit.
        cmpx    #INPUTBUF+249
        beq     LINE_NEXT
        sta     ,x+
NATIVE_TAIL_7
        jsr     SCREENOUT_HOOKED
        bra     LINE_NEXT
LINE_BACK
        cmpx    ,s
        beq     LINE_NEXT
        leax    -1,x
        bra     NATIVE_TAIL_7

LINE_CLEAR
        jsr     CLS
        ldx     ,s
        clr     LINE_INVALID
        bra     LINE_NEXT
LINE_CANCEL
        ldx     ,s
        inc     LINE_INVALID
LINE_FINISH
        clr     ,x
        lda     #13
        jsr     SCREENOUT_HOOKED
        ; The cursor is the sole editor length; $D7 remains caller-owned.
        tfr     x,d
        subd    ,s
        puls    x
        tst     LINE_INVALID
        beq     LINE_VALID
        coma
        rts
LINE_VALID
        clra
        rts
JOYIN
        ; Neither this loop nor SELECT_MUX writes the public Y/U registers.
        ; The comparator's four mux inputs are sampled without a host shortcut.
        ldx     #POTVAL+4
        lda     #3
        sta     COL
JOY_AXIS
        ldb     COL
        bsr     SELECT_MUX
        clrb
JOY_TRY
        tfr     b,a
        lsla
        lsla
        ora     #2
        sta     $ff20
        lda     $ff00
        bita    #128
        beq     JOY_DONE
        incb
        cmpb    #64
        blo     JOY_TRY
JOY_DONE
        ; Store the highest DAC level strictly below the input, clamped at zero.
        tstb
        beq     JOY_STORE
        decb
JOY_STORE
        stb     ,-x
        dec     COL
        bpl     JOY_AXIS
        rts
; B selects the two PIA mux lines without changing their other control bits.
SELECT_MUX
        lda     $ff01
        anda    #$f7
        bitb    #1
        beq     JOY_A
        ora     #8
JOY_A
        sta     $ff01
        lda     $ff03
        anda    #$f7
        bitb    #2
        beq     JOY_B
        ora     #8
JOY_B
        sta     $ff03
        rts

MOTORON
        pshs    x
        lda     $ff21
        ora     #$38
        sta     $ff21
        ; The public count wraps zero through 65536 eight-E-cycle iterations.
        ldx     $95
MOTOR_DELAY
        leax    -1,x
        bne     MOTOR_DELAY
        puls    x,pc
MOTOROFF
        lda     $ff21
        anda    #$f7
        ora     #$30
        sta     $ff21
        rts

KEYMAP
        fcb     $5e,$0a,$08,$09,$20,$0d,$0c,$03,0,0,0,0
; Shifted navigation characters are part of the published keyboard result.
; In particular Shift-Backspace leaves EDIT insertion with character $15.
        fcb     $5f,$5b,$15,$5d,$20,$0d,$5c
BANNER
        ifdef   STUDIO_EXTENDED
        fcc     "EIGHTBIT STUDIO BASIC"
        fcb     13
        fcc     "32K RAM - EXPANDED ROM"
        else
        fcc     "EIGHTBIT BASIC DEVELOPMENT"
        fcb     13
        fcc     "NOT QUALIFIED"
        endc
        fcb     13,0
PROMPT
        ifdef   STUDIO_EXTENDED
        fcc     "OK> "
        else
        fcc     "DEV> "
        endc
EMPTY_TEXT
        fcb     0
