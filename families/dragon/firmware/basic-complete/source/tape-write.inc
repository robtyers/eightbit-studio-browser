; Original digital cassette output, least-significant bit first.
; Both halves must represent the bit frequency. A constant short high half
; produces invalid zero bits for independent cassette waveform decoders.
TAPE_BYTE_OUT
        pshs    cc,d,x,y,u
        orcc    #$50
        ldx     #8
TAPE_WRITE_BIT
        lsr     1,s
        ldb     #41
        bcs     TAPE_WRITE_PERIOD
        ldb     #79
TAPE_WRITE_PERIOD
        stb     $82
        lda     #254
        sta     $ff20
TAPE_WRITE_HIGH
        decb
        bne     TAPE_WRITE_HIGH
        lda     #2
        sta     $ff20
        ldb     $82
        subb    #6
        ; The next byte's native return/framing/call supplies the final low
        ; interval. Avoid counting that overhead twice at byte boundaries.
        cmpx    #1
        bne     TAPE_WRITE_LOW
        subb    #16
TAPE_WRITE_LOW
        decb
        bne     TAPE_WRITE_LOW
        leax    -1,x
        bne     TAPE_WRITE_BIT
        puls    cc,d,x,y,u,pc
TAPE_PREP
        pshs    d,x
        jsr     MOTORON
        ; The published count is a wrapping 16-bit countdown: zero is 65,536.
        ldx     $90
TAPE_LEADER
        lda     #$55
        jsr     TAPE_BYTE_OUT
        leax    -1,x
        bne     TAPE_LEADER
TAPE_PREP_END
        ; Preparation retains the previous input status. TST also retains
        ; the six-cycle framing interval formerly occupied by CLR.
        tst     $81
        andcc   #$fe
        puls    d,x,pc
