; Original Eightbit Studio monitor, Apache-2.0, 14 October 2026.
; Hardware facts: Motorola M6809PM and the Dragon physical PIA matrix.
; No original firmware, emulator implementation or external glyph data is used.
        org     $8000
CURSOR  equ     $40
LATCH   equ     $42
COL     equ     $43
MASK    equ     $44
ROW     equ     $45
ADDR    equ     $46
COUNT   equ     $48
VALUE   equ     $4a
OVERRUN equ     $4b
INPUT   equ     $0100
RESET
        orcc    #$50
        lds     #$03ff
        clra
        tfr     a,dp
        sta     $ff01
        sta     $ff00
        sta     $ff03
        lda     #$ff
        sta     $ff02
        lda     #4
        sta     $ff01
        sta     $ff03
        clr     $ff23
        lda     #$f8
        sta     $ff22
        lda     #4
        sta     $ff23
        clr     $ff22
        ; SAM text mode, display base $0400, standard RAM/ROM map.
        clr     $ffc0
        clr     $ffc2
        clr     $ffc4
        clr     $ffc6
        clr     $ffc9
        clr     $ffca
        clr     $ffcc
        clr     $ffce
        clr     $ffd0
        clr     $ffd2
        clr     $ffde
        ; Select SAM's 64K DRAM routing for the installed 32K bank.
        clr     $ffda
        clr     $ffdd
        clr     LATCH
        jsr     CLEAR
        ldx     #BANNER
        jsr     PRINT
PROMPT
        clr     OVERRUN
        lda     #$0d
        jsr     PUTCH
        lda     #$3e
        jsr     PUTCH
        ldu     #INPUT
READLINE
        jsr     GETKEY
        tsta
        beq     READLINE
        cmpa    #$0d
        beq     COMMAND
        cmpa    #8
        beq     BACKSPACE
        cmpa    #$20
        blo     READLINE
        cmpu    #INPUT+95
        blo     STORECHAR
        lda     #1
        sta     OVERRUN
        bra     READLINE
STORECHAR
        sta     ,u+
        jsr     PUTCH
        bra     READLINE
BACKSPACE
        cmpu    #INPUT
        beq     READLINE
        leau    -1,u
        ldx     CURSOR
        cmpx    #$0400
        bls     READLINE
        leax    -1,x
        stx     CURSOR
        lda     #$20
        sta     ,x
        bra     READLINE
COMMAND
        tst     OVERRUN
        lbne    BAD
        clr     ,u
        lda     #$0d
        jsr     PUTCH
        ldy     #INPUT
        lda     ,y+
        cmpa    #'H'
        lbeq    HELP
        cmpa    #'R'
        lbeq    RESTART
        sta     VALUE
        jsr     HEXWORD
        lbcs    BAD
        std     ADDR
        lda     VALUE
        cmpa    #'D'
        lbeq    DUMP
        cmpa    #'G'
        lbeq    GO
        cmpa    #'L'
        lbeq    LOAD
        jsr     HEXWORD
        lbcs    BAD
        std     COUNT
        lda     VALUE
        cmpa    #'M'
        lbeq    MODIFY
        cmpa    #'F'
        lbeq    FILL
        cmpa    #'C'
        lbeq    CHECKSUM
BAD
        ldx     #ERROR
        jsr     PRINT
        lbra    PROMPT
HELP
        jsr     ENDLINE
        bcs     BAD
        ldx     #HELPMSG
        jsr     PRINT
        lbra    PROMPT
RESTART
        jsr     ENDLINE
        lbcs    BAD
        jmp     RESET
MODIFY
        jsr     ENDLINE
        bcs     BAD
        lda     COUNT
        bne     BAD
        ldd     #1
        jsr     RANGE
        bcs     BAD
        ldx     ADDR
        lda     COUNT+1
        sta     ,x
        lbra    PROMPT
GO
        jsr     ENDLINE
        bcs     BAD
        ldd     #1
        jsr     RANGE
        bcs     BAD
        ldx     ADDR
        jsr     ,x
        lbra    PROMPT
DUMP
        jsr     ENDLINE
        bcs     BAD
        ldx     ADDR
        cmpx    #$bff0
        bhi     BAD
        ldb     #8
DUMPLOOP
        lda     ,x+
        jsr     HEXOUT
        lda     #$20
        jsr     PUTCH
        decb
        bne     DUMPLOOP
        lbra    PROMPT
FILL
        jsr     HEXWORD
        lbcs    BAD
        tsta
        lbne    BAD
        stb     VALUE
        jsr     ENDLINE
        lbcs    BAD
        ldd     COUNT
        jsr     RANGE
        lbcs    BAD
        ldx     ADDR
        ldu     COUNT
FILLLOOP
        lda     VALUE
        sta     ,x+
        leau    -1,u
        cmpu    #0
        bne     FILLLOOP
        lbra    PROMPT
CHECKSUM
        jsr     ENDLINE
        lbcs    BAD
        ldd     COUNT
        jsr     RANGE
        lbcs    BAD
        ldx     ADDR
        ldu     COUNT
        clra
SUMLOOP
        adda    ,x+
        leau    -1,u
        cmpu    #0
        bne     SUMLOOP
        jsr     HEXOUT
        lbra    PROMPT
LOAD
        ; Validate the entire byte stream before any user RAM changes.
        jsr     SPACES
        tfr     y,u
        clr     COUNT
        clr     COUNT+1
LOADCHECK
        lda     ,y
        beq     LOADVALID
        jsr     HEXBYTE
        lbcs    BAD
        inc     COUNT+1
        bra     LOADCHECK
LOADVALID
        ldd     COUNT
        jsr     RANGE
        lbcs    BAD
        tfr     u,y
        ldx     ADDR
LOADWRITE
        lda     ,y
        beq     LOADDONE
        jsr     HEXBYTE
        sta     ,x+
        bra     LOADWRITE
LOADDONE
        lbra    PROMPT
RANGE
        ; D is a positive count <= $0400; ADDR is user RAM, no wrapping.
        cmpd    #1
        blo     RANGEFAIL
        cmpd    #$0400
        bhi     RANGEFAIL
        ldx     ADDR
        cmpx    #$0600
        blo     RANGEFAIL
        addd    ADDR
        bcs     RANGEFAIL
        cmpd    #$8000
        bhi     RANGEFAIL
        andcc   #$fe
        rts
RANGEFAIL
        orcc    #1
        rts
SPACES
        lda     ,y
        cmpa    #$20
        bne     SPACEEND
        leay    1,y
        bra     SPACES
SPACEEND
        rts
ENDLINE
        jsr     SPACES
        tsta
        bne     RANGEFAIL
        andcc   #$fe
        rts
HEXWORD
        jsr     SPACES
        pshs    x
        ldx     #0
        clrb
HEXLOOP
        lda     ,y
        beq     HEXEND
        cmpa    #$20
        beq     HEXEND
        jsr     NIBBLE
        bcs     HEXFAIL
        cmpb    #4
        bhs     HEXFAIL
        pshs    b
        tfr     x,d
        lslb
        rola
        lslb
        rola
        lslb
        rola
        lslb
        rola
        tfr     d,x
        puls    b
        lda     ,y+
        jsr     NIBBLE
        pshs    b
        tfr     a,b
        abx
        puls    b
        incb
        bra     HEXLOOP
HEXEND
        tstb
        beq     HEXFAIL
        tfr     x,d
        puls    x
        andcc   #$fe
        rts
HEXFAIL
        puls    x
        orcc    #1
        rts
HEXBYTE
        lda     ,y+
        jsr     NIBBLE
        bcs     BYTEFAIL
        lsla
        lsla
        lsla
        lsla
        pshs    a
        lda     ,y+
        jsr     NIBBLE
        bcs     BYTEPOPFAIL
        ora     ,s+
        andcc   #$fe
        rts
BYTEPOPFAIL
        leas    1,s
BYTEFAIL
        orcc    #1
        rts
NIBBLE
        cmpa    #'0'
        blo     BYTEFAIL
        cmpa    #'9'
        bls     DIGIT
        cmpa    #'A'
        blo     BYTEFAIL
        cmpa    #'F'
        bhi     BYTEFAIL
        suba    #'A'-10
        andcc   #$fe
        rts
DIGIT
        suba    #'0'
        andcc   #$fe
        rts
HEXOUT
        pshs    a,b,x
        tfr     a,b
        lsra
        lsra
        lsra
        lsra
        jsr     HEXCHAR
        tfr     b,a
        anda    #15
        jsr     HEXCHAR
        puls    a,b,x,pc
HEXCHAR
        ldx     #DIGITS
        lda     a,x
        jmp     PUTCH
PRINT
        lda     ,x+
        beq     PRINTEND
        jsr     PUTCH
        bra     PRINT
PRINTEND
        rts
CLEAR
        pshs    a,x
        ldx     #$0400
        lda     #$20
CLEARLOOP
        sta     ,x+
        cmpx    #$0600
        bne     CLEARLOOP
        ldx     #$0400
        stx     CURSOR
        puls    a,x,pc
PUTCH
        pshs    a,b,x
        ldx     CURSOR
        cmpa    #$0d
        beq     NEWLINE
        anda    #$3f
        sta     ,x+
        bra     CURSORSAVE
NEWLINE
        tfr     x,d
        andb    #$e0
        addd    #32
        tfr     d,x
CURSORSAVE
        cmpx    #$0600
        blo     CURSOROK
        jsr     CLEAR
        bra     PUTEND
CURSOROK
        stx     CURSOR
PUTEND
        puls    a,b,x,pc
GETKEY
        pshs    b,x
        clr     $ff02
        lda     $ff00
        anda    #$7f
        cmpa    #$7f
        beq     RELEASED
        tst     LATCH
        bne     NOKEY
        clr     COL
        lda     #$fe
        sta     MASK
SCANCOL
        lda     MASK
        sta     $ff02
        lda     $ff00
        coma
        anda    #$7f
        bne     FOUNDROW
        inc     COL
        orcc    #1
        rol     MASK
        ldb     COL
        cmpb    #8
        blo     SCANCOL
        bra     NOKEY
FOUNDROW
        clr     ROW
SCANROW
        lsra
        bcs     GOTROW
        inc     ROW
        bra     SCANROW
GOTROW
        ldb     ROW
        aslb
        aslb
        aslb
        addb    COL
        ldx     #KEYS
        abx
        lda     ,x
        inc     LATCH
        bra     KEYDONE
RELEASED
        clr     LATCH
NOKEY
        clra
KEYDONE
        puls    b,x,pc
BANNER  fcc     "EIGHTBIT DRAGON MONITOR 0.1"
        fcb     13
        fcc     "ORIGINAL ROM - BASIC NOT READY"
        fcb     13
        fcc     "H FOR HELP"
        fcb     0
ERROR   fcc     "INVALID COMMAND OR RANGE"
        fcb     0
HELPMSG fcc     "HEX ADDRESSES/COUNTS/BYTES"
        fcb     13
        fcc     "D ADDR  DUMP 8 BYTES"
        fcb     13
        fcc     "M ADDR BYTE   F ADDR COUNT BYTE"
        fcb     13
        fcc     "C ADDR COUNT  CHECKSUM"
        fcb     13
        fcc     "L ADDR HEXBYTES   G ADDR"
        fcb     13
        fcc     "R RESET - USER RAM 0600-7FFF"
        fcb     0
DIGITS  fcc     "0123456789ABCDEF"
KEYS    fcc     "0123456789:;,-./@ABCDEFGHIJKLMNOPQRSTUVWXYZ"
        fcb     0,0,8,0,32,13,12,3,0,0,0,0,0
        ; Unused firmware is deterministic. SAM vectors mirror the last 32 bytes.
        fill    $ff,$bfe0-*
        fdb     RESET,RESET,RESET,RESET,RESET,RESET,RESET,RESET
        fdb     RESET,RESET,RESET,RESET,RESET,RESET,RESET,RESET
