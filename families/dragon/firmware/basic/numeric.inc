; Original FAC/packed-number conversion routines. Apache-2.0.
; Implementation is independent of OEM firmware; FAC interfaces are public facts.

; Both integer conversions preserve X/Y/U and the token cursor.
INT_TO_FAC
        clr     $06
        clr     FSIGN
        tsta
        bpl     INT_POSITIVE
        jsr     NEGATE_D
        dec     FSIGN
INT_POSITIVE
        std     FMANT
        beq     FAC_ZERO
        clr     FMANT+2
        clr     FMANT+3
INT_MAGNITUDE
        lda     #144
        sta     FAC
INT_NORMALISE
        tst     FMANT
        lbmi    F_FINISH
        lsl     FMANT+1
        rol     FMANT
        dec     FAC
        bra     INT_NORMALISE



FAC_ZERO
        clra
        clrb
        std     FAC
        std     FAC+2
        std     FAC+4
        rts

FAC_TO_INT
        bsr     FAC_WORD_MAGNITUDE
        bcs     INT_RETURN
        tst     FSIGN
        bmi     INT_NEGATIVE
        tsta
        bmi     INT_OVERFLOW
INT_RETURN
        rts
INT_NEGATIVE
        cmpd    #$8000
        bhi     INT_OVERFLOW
        jsr     NEGATE_D
        andcc   #$fe
        rts
; Shared unsigned magnitude. Public signed conversion and address conversion
; keep their own sign/range policy and preserve X/Y/U on every path.
FAC_WORD_MAGNITUDE
        lda     FAC
        cmpa    #129
        blo     INT_ZERO
        cmpa    #144
        bhi     INT_OVERFLOW
        nega
        adda    #144
        sta     TEMP
        ldd     FMANT
INT_SHIFT
        tst     TEMP
        beq     INT_MAGNITUDE_DONE
        lsra
        rorb
        dec     TEMP
        bra     INT_SHIFT
INT_MAGNITUDE_DONE
        andcc   #$fe
        rts
INT_ZERO
        clra
        clrb
        rts
INT_OVERFLOW
        coma
        rts

; X points at a five-byte packed value. Unpacked FAC carries its hidden bit.
FAC_LOAD
        lda     ,x+
        sta     FAC
        beq     FAC_ZERO
        lda     ,x+
        sta     FSIGN
        ora     #$80
        sta     FMANT
        ldd     ,x++
        std     FMANT+1
        lda     ,x+
        sta     FMANT+3
        rts
FAC_STORE
        lda     FAC
        sta     ,x+
        beq     FAC_STORE_ZERO
        jsr     F_PACK_SIGN
        sta     ,x+
        ldd     FMANT+1
        std     ,x++
        lda     FMANT+3
        sta     ,x+
        rts
FAC_STORE_ZERO
        clrb
        std     ,x++
        std     ,x++
        rts

