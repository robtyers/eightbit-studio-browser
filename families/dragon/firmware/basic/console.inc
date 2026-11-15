BOOT
        orcc    #$50
        clra
        tfr     a,dp
        ; Select normal RAM before pushing any return address or reading its workspace.
        clr     $ffda
        clr     $ffdd
        lds     #$7fff
        lda     RSTFLAG
        cmpa    #$12
        beq     WARM
        cmpa    #$55
        bne     COLD
        ldx     RSTVEC
        lda     ,x
        cmpa    #$12
        bne     COLD
        jmp     ,x
COLD
        ; BOOT has selected RAM, masked interrupts and established DP/S.
        bsr     HW_INIT
        ldx     #0
CLEAR_WORK
        clr     ,x+
        cmpx    #$0400
        bne     CLEAR_WORK
        jsr     SETUP
        lds     STACKTOP
        leas    1,s
        jsr     CLS
        ldx     #BANNER
        jsr     PUTS
        andcc   #$af
DIRECT
        clr     ERROR_CODE
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
        bsr     HW_INIT
        clr     DEVNUM
        clr     FORMAT_BUFFER
        lds     STACKTOP
        leas    1,s
        jsr     DIRECT_LINE
        andcc   #$af
        bra     DIRECT

HW_INIT
        ldx     #$ffc0
HW_SAM
        clr     ,x
        leax    2,x
        cmpx    #$ffe0
        blo     HW_SAM
        clr     $ffdd
        clr     $ffc9
        pshs    u
        ldu     #$ff00
        ldx     #HW_REGISTERS
HW_REGISTER
        ldd     ,x++
        stb     a,u
        cmpx    #HW_REGISTERS_END
        blo     HW_REGISTER
        puls    u,pc
HW_REGISTERS
        fcb     $03,4,$02,$ff,$03,0,$02,$ff
        fcb     $01,$34,$03,$35,$21,4,$20,2
        fcb     $21,0,$20,$fe,$21,$34,$23,4
        fcb     $22,0,$23,0,$22,$f8,$23,$34
HW_REGISTERS_END

SETUP
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
        sta     $149
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
        clr     $6c
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
        lbmi    FILE_ASCII_BYTE
        bne     FORMAT_WRITE
        pshs    a
        lda     DEVNUM
        cmpa    #$fe
        puls    a
        beq     PRINTEROUT
SCREENOUT
        pshs    d,x,y,u
        jsr     OUTPUT_HOOK
        ldx     CURSOR
        cmpa    #13
        beq     SCREEN_CR
        cmpa    #8
        beq     SCREEN_BS
        cmpa    #12
        beq     SCREEN_CLEAR
        cmpa    #32
        blo     SCREEN_END
        cmpa    #128
        bhs     SCREEN_CELL
        cmpa    #'a
        blo     SCREEN_UPPER
        cmpa    #'z
        bhi     SCREEN_UPPER
        anda    #$1f
        bra     SCREEN_CELL
SCREEN_UPPER
        anda    #$3f
        ora     #$40
SCREEN_CELL
        sta     ,x+
        bra     SCREEN_SAVE
SCREEN_CR
        tfr     x,d
        andb    #$e0
        addd    #32
        tfr     d,x
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
        tfr     x,d
        andb    #31
        stb     $6c
SCREEN_END
        puls    d,x,y,u,pc
SCREEN_BS
        cmpx    #$0400
        bls     SCREEN_END
        leax    -1,x
        lda     #$60
        sta     ,x
        bra     SCREEN_CURSOR
SCREEN_CLEAR
        jsr     CLS
        bra     SCREEN_END
PRINTEROUT
        pshs    d,x,y,u
PRINT_WAIT
        ldb     $ff22
        bitb    #1
        bne     PRINT_WAIT
        ; An empty printer line contains one blank before its carriage return.
        ; Wait before changing the public column, including the recursive blank.
        cmpa    #13
        bne     PRINT_ADVANCE
        tst     $9c
        bne     PRINT_CLEAR_COLUMN
        lda     #32
        bsr     PRINTEROUT
        lda     #13
PRINT_CLEAR_COLUMN
        clr     $9c
        bra     PRINT_WRITE
PRINT_ADVANCE
        inc     $9c
PRINT_WRITE
        sta     $ff02
        ldb     $ff20
        andb    #$fd
        stb     $ff20
        nop
        orb     #2
        stb     $ff20
PRINT_RETURN
        puls    d,x,y,u,pc

BLINK
        pshs    a,x
        dec     $8f
        bne     BLINK_END
        ldx     CURSOR
        lda     ,x
        eora    #$40
        sta     ,x
BLINK_END
        puls    a,x,pc
KEYIN
        pshs    b,x,y
        jsr     INPUT_HOOK
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
        cmpa    #'/
        bhi     SHIFT_DIGIT
SHIFT_PUNCTUATION
        adda    #16
        bra     KEY_RETURN
SHIFT_DIGIT
        suba    #16
        cmpa    #32
        bne     KEY_RETURN
        com     $149
        clra
KEY_RETURN
        sta     $87
        puls    b,x,y,pc
READLINE
        ldx     #INPUTBUF
READLINE_INTO
        pshs    x
        jsr     LINE_HOOK
        clr     LINELEN
        clr     LINE_INVALID
LINE_NEXT
        jsr     $bbe5
        beq     LINE_NEXT
        cmpa    #3
        beq     LINE_CANCEL
        cmpa    #13
        beq     LINE_FINISH
        cmpa    #8
        beq     LINE_BACK
        cmpa    #32
        blo     LINE_NEXT
        ldb     LINELEN
        ; Keep the terminator inside the published $02DD-$03D8 input area.
        cmpb    #251
        beq     LINE_TOO_LONG
        sta     ,x+
        inc     LINELEN
NATIVE_TAIL_7
        jsr     SCREENOUT
        bra     LINE_NEXT
LINE_TOO_LONG
        stb     LINE_INVALID
        bra     LINE_NEXT
LINE_BACK
        tst     LINELEN
        beq     LINE_NEXT
        dec     LINELEN
        leax    -1,x
        bra     NATIVE_TAIL_7

LINE_CANCEL
        ldx     ,s
        clr     LINELEN
        inc     LINE_INVALID
LINE_FINISH
        clr     ,x
        lda     #13
        jsr     SCREENOUT
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
        lda     $ff21
        ora     #$38
NATIVE_TAIL_11
        sta     $ff21
        rts
MOTOROFF
        lda     $ff21
        anda    #$f7
        ora     #$30
        bra     NATIVE_TAIL_11

KEYMAP
        fcb     $5e,$0a,$08,$09,$20,$0d,$0c,$03,0,0,0,0
; Shifted navigation characters are part of the published keyboard result.
; In particular Shift-Backspace leaves EDIT insertion with character $15.
        fcb     $5f,$5b,$15,$5d,$20,$0d,$5c
BANNER
        fcc     "EIGHTBIT BASIC DEVELOPMENT"
        fcb     13
        fcc     "NOT QUALIFIED"
        fcb     13,0
PROMPT
        fcc     "DEV> "
EMPTY_TEXT
        fcb     0

