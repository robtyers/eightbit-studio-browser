; Original D64 supplement interfaces; no historical firmware code is used.
D64_KEY_EVENT
        pshs d
        ldb $011c
        cmpb #2
        beq D64_KEY_EVENT_DONE
        ldb #1
        stb $011c
        lda $011f
        ldb #8
        mul
        std $011d
D64_KEY_EVENT_DONE
        puls d,pc
D64_REPEAT_IRQ
        lda $ff02
        ldd TIMER+1
        addd #1
        std TIMER+1
        bcc D64_REPEAT_TICK
        inc TIMER
D64_REPEAT_TICK
        lda $ff21
        bita #8
        bne D64_REPEAT_DONE
        tst $011c
        beq D64_REPEAT_DONE
        ldx #KEYOLD
        ldb #8
D64_REPEAT_SCAN
        lda ,x+
        cmpa #$ff
        bne D64_REPEAT_HELD
        decb
        bne D64_REPEAT_SCAN
        clr $011c
        ldd #0
        std $011d
        rti
D64_REPEAT_HELD
        tst $011c
        beq D64_REPEAT_DONE
        ldd $011d
        beq D64_REPEAT_DUE
        subd #1
        std $011d
        bne D64_REPEAT_DONE
D64_REPEAT_DUE
        ldx #KEYOLD
        ldb #8
        lda #$ff
D64_REPEAT_CLEAR
        sta ,x+
        decb
        bne D64_REPEAT_CLEAR
        lda #2
        sta $011c
        clra
        ldb $011f
        std $011d
D64_REPEAT_DONE
        rti
D64_SERIAL_BYTE
        pshs b,x,y
D64_SERIAL_WAIT
        ldb $ff05
        bitb #16
        beq D64_SERIAL_WAIT
        sta $ff04
        cmpa #13
        bne D64_SERIAL_DONE
        ldx $03fd
        beq D64_SERIAL_DONE
D64_SERIAL_DELAY
        ldy #1119
D64_SERIAL_DELAY_UNIT
        leay -1,y
        bne D64_SERIAL_DELAY_UNIT
        leax -1,x
        bne D64_SERIAL_DELAY
D64_SERIAL_DONE
        puls b,x,y,pc

; Published D64 serial entries. Polling is guest work and remains cancellable
; through the bounded machine scheduler; no host input is fabricated.
D64_SERIN
        pshs cc,b
D64_SERIN_WAIT
        ldb $ff05
        bitb #8
        beq D64_SERIN_WAIT
        lda $ff04
        puls cc,b,pc
D64_SEROUT
        pshs cc,b
D64_SEROUT_WAIT
        ldb $ff05
        bitb #16
        beq D64_SEROUT_WAIT
        sta $ff04
        puls cc,b,pc
D64_SERSET
        cmpb #6
        bhi D64_SERSET_BAD
        ldx #D64_SERIAL_RATES
        ldb b,x
        pshs b
        ldb $ff07
        andb #$f0
        orb ,s+
        stb $ff07
        andcc #$fe
        rts
D64_SERSET_BAD
        orcc #1
        rts
D64_SERIAL_RATES
        fcb 3,6,7,8,10,12,14
