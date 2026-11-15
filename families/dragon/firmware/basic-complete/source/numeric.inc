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
        ldd     FMANT
INT_NORMALISE
        tsta
        bmi     INT_NORMALISED
        lslb
        rola
        dec     FAC
        bra     INT_NORMALISE
INT_NORMALISED
        std     FMANT
        jmp     F_FINISH



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
        ; The existing arithmetic guard records whether any fractional bit was
        ; discarded. Private conversions still return truncation without
        ; changing FAC; the public floor conversion consumes this one fact.
        clr     FGUARD
        lda     FAC
        cmpa    #129
        blo     INT_ZERO
        cmpa    #144
        bhi     INT_OVERFLOW
        nega
        adda    #144
        sta     TEMP
        ldd     FMANT+2
        beq     INT_HIGH_WORD
        inc     FGUARD
INT_HIGH_WORD
        ldd     FMANT
INT_SHIFT
        tst     TEMP
        beq     INT_MAGNITUDE_DONE
        lsra
        rorb
        bcc     INT_SHIFT_EXACT
        inc     FGUARD
INT_SHIFT_EXACT
        dec     TEMP
        bra     INT_SHIFT
INT_MAGNITUDE_DONE
        andcc   #$fe
        rts
INT_ZERO
        tst     FAC
        beq     INT_EXACT_ZERO
        inc     FGUARD
INT_EXACT_ZERO
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
        ldd     ,x++
        sta     FSIGN
        ora     #$80
        std     FMANT
        ldd     ,x++
        std     FMANT+2
        rts
FAC_STORE
        lda     FAC
        sta     ,x+
        beq     FAC_STORE_ZERO
        jsr     F_PACK_SIGN
        ldb     FMANT+1
        std     ,x++
        ldd     FMANT+2
        std     ,x++
        rts
FAC_STORE_ZERO
        clrb
        std     ,x++
        std     ,x++
        rts

; Published signed-word conversions use floor, raise FC, and leave the signed
; 32-bit integer in the FAC mantissa. Internal FAC_TO_INT keeps its truncation
; and carry contract for private address and formatting helpers.
PUBLIC_FAC_TO_INT
        jsr     EX_NUMBER_CHECK
PUBLIC_FAC_WORD
        jsr     FAC_TO_INT
        bcs     PUBLIC_INTEGER_RANGE
        tst     FSIGN
        bpl     PUBLIC_INTEGER_RESULT
        tst     FGUARD
        beq     PUBLIC_INTEGER_RESULT
        subd    #1
        bvs     PUBLIC_INTEGER_RANGE
PUBLIC_INTEGER_RESULT
        std     FMANT+2
        ; Sign-extend the returned word while preserving the published FAC
        ; exponent and sign byte. Reload D after storing the upper half.
        asla
        lda     #0
        sbca    #0
        tfr     a,b
        std     FMANT
        ldd     FMANT+2
        andcc   #$fe
        rts
PUBLIC_INTEGER_RANGE
        jmp     NATIVE_RANGE_ERROR
