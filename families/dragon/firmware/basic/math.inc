; Original 32-significant-bit MBF arithmetic. Apache-2.0.
; FAC is the left operand, ARG the right. Guard/sticky bits round to nearest, exact halfway magnitudes upwards.
; C signals range/division errors to the future language error dispatcher.
F_SUB
        lda     ASIGN
        eora    #$80
        sta     ASIGN
F_ADD
        clr     FGUARD
        clr     AGUARD
        tst     ARG
        beq     NATIVE_ISLAND_MATH_INC_0
        tst     FAC
        bne     F_ADD_ORDER
        ldx     #ARG
        ldy     #FAC
        jsr     FAC_UNPACKED_COPY
        jmp     F_FINISH
NATIVE_ISLAND_MATH_INC_0
        jmp     F_FINISH
F_ADD_ORDER
        lda     FAC
        cmpa    ARG
        bhs     F_ALIGN
        jsr     F_SWAP
F_ALIGN
        lda     FAC
        suba    ARG
        cmpa    #40
        bhs     NATIVE_ISLAND_MATH_INC_0
        tfr     a,b
        tstb
        beq     F_ALIGNED
F_ALIGN_LOOP
        lsr     AMANT
        ror     AMANT+1
        ror     AMANT+2
        ror     AMANT+3
        ror     AGUARD
        bcc     F_ALIGN_EXACT
        lda     AGUARD
        ora     #1
        sta     AGUARD
F_ALIGN_EXACT
        decb
        bne     F_ALIGN_LOOP
F_ALIGNED
        lda     FAC
        sta     ARG
        lda     FSIGN
        eora    ASIGN
        asla
        lda     #0
        sbca    #0
        sta     TEMP
        ; A zero/FF mask selects addition or two's-complement subtraction.
        ; EOR and the descending loads/stores preserve the byte carry chain.
        asla
        lda     AGUARD
        eora    TEMP
        adca    FGUARD
        sta     FGUARD
        ldx     #FMANT+4
        ldy     #AMANT+4
        ldb     #4
F_ADD_BYTES
        lda     ,-y
        eora    TEMP
        adca    ,-x
        sta     ,x
        decb
        bne     F_ADD_BYTES
        tst     TEMP
        bmi     F_SUBTRACT_RESULT
        bcc     F_ROUND
        ror     FMANT
        ror     FMANT+1
        ror     FMANT+2
        ror     FMANT+3
        ror     FGUARD
        bcc     F_ADD_NO_STICKY
        lda     FGUARD
        ora     #1
        sta     FGUARD
F_ADD_NO_STICKY
        inc     FAC
        beq     F_RANGE
        bra     F_ROUND
F_SUBTRACT_RESULT
        bcs     F_DIFFERENCE_MAGNITUDE
        jsr     F_NEGATE
        neg     FGUARD
        ldx     #FMANT+4
        ldb     #4
F_NEGATE_DIFFERENCE
        lda     #0
        sbca    ,-x
        sta     ,x
        decb
        bne     F_NEGATE_DIFFERENCE
F_DIFFERENCE_MAGNITUDE
        lda     FMANT
        ora     FMANT+1
        ora     FMANT+2
        ora     FMANT+3
        ora     FGUARD
        beq     F_ZERO_JUMP

F_NORMALISE
        tst     FMANT
        bmi     F_ROUND
        lsl     FGUARD
        rol     FMANT+3
        rol     FMANT+2
        rol     FMANT+1
        rol     FMANT
        dec     FAC
        bne     F_NORMALISE
F_ZERO_JUMP
        jmp     FAC_ZERO
F_ROUND
        tst     FGUARD
        bpl     F_FINISH
F_INCREMENT
        ldx     #FMANT+4
F_INCREMENT_BYTE
        inc     ,-x
        bne     F_FINISH
        cmpx    #FMANT
        bhi     F_INCREMENT_BYTE
        lda     #$80
        sta     FMANT
        inc     FAC
        beq     F_RANGE
F_PACK_SIGN
        lda     FMANT
        eora    FSIGN
        anda    #$7f
        eora    FSIGN
        rts
F_FINISH
        bsr     F_PACK_SIGN
NATIVE_TAIL_5
        sta     FSIGN
        clrb
        rts
F_RANGE
        coma
        rts
F_SWAP
        pshs    u
        ldx     #FAC
        ldy     #ARG
F_SWAP_LOOP
        ldd     ,x
        ldu     ,y
        std     ,y++
        stu     ,x++
        cmpx    #FAC+6
        bne     F_SWAP_LOOP
        puls    u,pc

F_MUL
        tst     FAC
        beq     F_ZERO_JUMP
        tst     ARG
        beq     F_ZERO_JUMP
        jsr     F_PRODUCT_SIGN
        addb    ARG
        adca    #0
        subd    #128
        std     MEX
        ; The shared native product retains all 64 bits on the guest stack.
        leas    -8,s
        tfr     s,u
        bsr     F_PRODUCT
        tst     ,u
        bmi     F_PRODUCT_NORMAL
        lsl     7,u
        rol     6,u
        rol     5,u
        rol     4,u
        bsr     F_SHIFT_EXPONENT
F_PRODUCT_NORMAL
        lda     5,u
        ora     6,u
        ora     7,u
F_RESULT_STICKY
        tsta
        beq     F_RESULT_COPY
        lda     4,u
        ora     #1
        sta     4,u
F_RESULT_COPY
        ldd     ,u
        std     FMANT
        ldd     2,u
        std     FMANT+2
        lda     4,u
        sta     FGUARD
F_PRODUCT_EXACT
        leas    8,s
        ldd     MEX
        bsr     F_EXPONENT
        bcs     LOCAL_FAILURE_10
        tst     FAC
        beq     LOCAL_FAILURE_16
        jmp     F_ROUND
; Unsigned FMANT * AMANT into eight bytes at U; operands are not modified.
; Sixteen native MULs, two temporary stack bytes and no permanent workspace.
F_PRODUCT
        pshs    d,u
        clra
        clrb
        ; Each row writes its new high byte before the next row can read it.
        ; Only the low four bytes participate before that first write.
        std     4,u
        std     6,u
        ldy     #AMANT+4
        leau    7,u
F_PRODUCT_ROW
        lda     ,-y
        sta     ,s
        clr     1,s
        ldx     #FMANT+4
F_PRODUCT_LOOP
        lda     ,-x
        ldb     ,s
        mul
        addb    ,u
        adca    #0
        addb    1,s
        adca    #0
        stb     ,u
        sta     1,s
        leau    -1,u
        cmpx    #FMANT
        bhi     F_PRODUCT_LOOP
        lda     1,s
        sta     ,u
        leau    3,u
        cmpy    #AMANT
        bhi     F_PRODUCT_ROW
        puls    d,u,pc

F_SHIFT_EXPONENT
        rol     3,u
        rol     2,u
        rol     1,u
        rol     ,u
        ldd     MEX
        subd    #1
        std     MEX
        rts

F_EXPONENT
        tsta
        bmi     F_EXP_ZERO
        bne     LOCAL_FAILURE_10
        tstb
        beq     F_EXP_ZERO
        stb     FAC
        clra
        rts
LOCAL_FAILURE_10
        coma
        rts
LOCAL_FAILURE_16
        jmp     F_ZERO_JUMP
F_EXP_ZERO
        clr     FAC
        rts

F_DIV
        tst     ARG
        beq     LOCAL_FAILURE_10
        tst     FAC
        beq     LOCAL_FAILURE_16
        bsr     F_PRODUCT_SIGN
        subb    ARG
        sbca    #0
        addd    #129
        std     MEX
        leas    -9,s
        tfr     s,u
        ldd     FMANT
        std     ,u
        ldd     FMANT+2
        std     2,u
        clra
        clrb
        std     4,u
        std     6,u
        sta     8,u
        bsr     F_REMAINDER_COMPARE
        bhs     F_DIV_NORMAL
        clra
        bsr     F_SHIFT_EXPONENT
F_DIV_NORMAL
        bsr     F_REMAINDER_SUBTRACT
        inc     8,u
        lda     #39
        sta     TEMP
F_DIV_LOOP
        ; The quotient's leading bit remains zero until the final iteration,
        ; so one carry chain shifts both words without contaminating remainder.
        leax    9,u
        ldb     #9
        clra
F_DIV_SHIFT
        rol     ,-x
        decb
        bne     F_DIV_SHIFT
        bcs     F_DIV_SUBTRACT
        bsr     F_REMAINDER_COMPARE
        blo     F_QUOTIENT_ZERO
F_DIV_SUBTRACT
        bsr     F_REMAINDER_SUBTRACT
        inc     8,u
F_QUOTIENT_ZERO
        dec     TEMP
        bne     F_DIV_LOOP
        lda     ,u
        ora     1,u
        ora     2,u
        ora     3,u
        ; The same five-byte result copy serves product and quotient. Keep
        ; the remainder predicate in A; LEAU/LEAS do not replace its value.
        leau    4,u
        leas    1,s
        jmp     F_RESULT_STICKY

F_REMAINDER_COMPARE
        ldd     ,u
        cmpd    AMANT
        bne     F_REMAINDER_RETURN
        ldd     2,u
        cmpd    AMANT+2
F_REMAINDER_RETURN
        rts
F_REMAINDER_SUBTRACT
        ldd     2,u
        subd    AMANT+2
        std     2,u
        ldd     ,u
        sbcb    AMANT+1
        sbca    AMANT
        std     ,u
        rts

F_PRODUCT_SIGN
        lda     FSIGN
        eora    ASIGN
        sta     FSIGN
        clra
        ldb     FAC
        rts

