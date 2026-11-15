; Original decimal presentation for packed BASIC numbers. Apache-2.0.
; Nine significant decimal digits; guest arithmetic, no host conversion.
; LIST, tracing and line diagnostics use the same exact digit conversion,
; omitting only PRINT's surrounding spaces. Their FAC is not a live value.
PRINT_UNSIGNED
        pshs    d,x,y,u
        ; Feed the integer directly to BCD at exponent 160. Normalising it
        ; first would require up to 31 subsequent decimal halvings.
        std     FMANT+2
        clra
        clrb
        std     FMANT
        sta     FSIGN
        lda     #160
        sta     FAC
        bra     FORMAT_ENTRY
PRINT_DATA_NUMBER
        pshs    d,x,y,u
        ldb     #$80+UF_PLUS
FORMAT_ENTRY
        ldy     #USING_EMIT
FORMAT_CONSUMER_ENTRY
        ; Width zero selects ordinary nine-digit formatting. UF_PLUS supplies
        ; the leading sign/blank; a positive flag also requests the trailing
        ; PRINT blank. Data-file records use bit seven to suppress that blank.
        clra
        pshs    d,y
        tfr     s,u
        jsr     USING_NUMBER
        leas    4,s
        puls    d,x,y,u,pc

FORMAT_OUT equ CHROUT

; Private field plan, arranged in output phase order. Rendering consumes the
; first four flags; comma and parser-only sign choices occupy the remaining bits.
UF_TRAIL       equ 1
UF_EXP         equ 2
UF_CURRENCY    equ 4
UF_STARS       equ 8
UF_COMMA       equ 16
UF_PLUS        equ 32
UF_TRAIL_PLUS  equ 64
UF_STRING      equ 128


; A is the current raw ASCII character. B reports a consumed minus sign;
; X advances only over a sign. DATA and VAL retain their own number rules.
ASCII_SIGN
        clrb
        cmpa    #'+
        beq     ASCII_SIGN_NEXT
        cmpa    #'-
        bne     ASCII_SIGN_END
ASCII_MINUS
        incb
ASCII_SIGN_NEXT
        leax    1,x
ASCII_SIGN_END
        rts


; Exact decimal digits shared by ordinary and field-formatted output. Y is a
; ten-byte caller buffer; A returns the first digit's decimal exponent. Only
; temporary guest stack memory is used. The caller chooses its rounding place.
FORMAT_DECIMAL
        ; FAC and U survive this private conversion. D/X/Y are scratch;
        ; both callers replace Y before using it again.
        pshs    u
        leas    -48,s
        clr     1,s
        leau    3,s
        tst     FAC
        beq     DECIMAL_ZERO
        ; Public integer conversion retains the exponent while replacing the
        ; mantissa. Its zero result must remain printable without normalising
        ; that published representation or scanning an empty decimal magnitude.
        ldd     FMANT
        bne     DECIMAL_CONVERT
        ldd     FMANT+2
        bne     DECIMAL_CONVERT
DECIMAL_ZERO
        ldb     #10
        lda     #'0
DECIMAL_ZERO_DIGIT
        sta     ,y+
        decb
        bne     DECIMAL_ZERO_DIGIT
        clra
        bra     DECIMAL_RETURN
DECIMAL_CONVERT
        tfr     u,x
        ldb     #45
DECIMAL_CLEAR
        clr     ,x+
        decb
        bne     DECIMAL_CLEAR
        lda     #32
        sta     2,s
DECIMAL_BINARY_BIT
        lsl     FMANT+3
        rol     FMANT+2
        rol     FMANT+1
        rol     FMANT
        ; Feed the discarded high bit back into the vacated low bit. INC
        ; retains carry for BCD, and 32 rotations restore the whole FAC.
        bcc     DECIMAL_BINARY_READY
        inc     FMANT+3
DECIMAL_BINARY_READY
        jsr     FORMAT_DOUBLE
        dec     2,s
        bne     DECIMAL_BINARY_BIT
        ldb     FAC
        subb    #160
        bcs     DECIMAL_SCALE_DOWN
        stb     2,s
        beq     DECIMAL_EXTRACT
DECIMAL_SCALE_UP
        clra
        jsr     FORMAT_DOUBLE
        dec     2,s
        bne     DECIMAL_SCALE_UP
        bra     DECIMAL_EXTRACT
DECIMAL_SCALE_DOWN
        negb
        stb     2,s
DECIMAL_HALVE
        tfr     u,x
        ldb     #45
        clra
DECIMAL_HALF_BYTE
        lda     ,x
        rora
        pshs    cc
        bpl     DECIMAL_HALF_TENS
        suba    #$30
DECIMAL_HALF_TENS
        bita    #8
        beq     DECIMAL_HALF_STORE
        suba    #3
DECIMAL_HALF_STORE
        sta     ,x+
        puls    cc
        decb
        bne     DECIMAL_HALF_BYTE
        dec     2,s
        bne     DECIMAL_HALVE
DECIMAL_EXTRACT
        tfr     u,x
        lda     #40
        sta     ,s
DECIMAL_LEADING
        jsr     FORMAT_BCD_DIGIT
        dec     ,s
        tsta
        beq     DECIMAL_LEADING
        ldb     #10
DECIMAL_DIGIT_OUT
        adda    #'0
        sta     ,y+
        decb
        beq     DECIMAL_READY
        jsr     FORMAT_BCD_DIGIT
        bra     DECIMAL_DIGIT_OUT
DECIMAL_READY
        lda     ,s
DECIMAL_RETURN
        leas    48,s
        puls    u,pc

; Round B (1..9) ASCII digits in place using the following guard digit.
; Carry reports a new leading digit, so the caller can advance its exponent.
DECIMAL_ROUND
        abx
        lda     ,x
        cmpa    #'5
        blo     DECIMAL_ROUND_CLEAR
DECIMAL_ROUND_NEXT
        inc     ,-x
        lda     ,x
        cmpa    #':
        blo     DECIMAL_ROUND_CLEAR
        lda     #'0
        sta     ,x
        decb
        bne     DECIMAL_ROUND_NEXT
        inc     ,x
        orcc    #1
        rts
DECIMAL_ROUND_CLEAR
        andcc   #$fe
DECIMAL_ROUND_END
        rts

; PRINT USING retains the authoritative descriptor and a byte cursor. The
; descriptor follows collection and array movement; later text remains live
; when an argument callback changes its pointer or payload. U's frame is:
; reference0, BASIC cursor2, remaining format characters4, seen5, separator6, release mark7,
; field width/flags/configuration8..11, spare12, relocation flag13, current text end14.
PRINT_USING
        leax    1,x
        jsr     EVAL_STRING
        tst     FAC
        lbeq    NATIVE_RANGE_ERROR
        lda     #';
        jsr     EX_EXPECT
USING_FRAME_READY
        ; No metadata is written until the complete private frame is safe.
        leau    -162,s
        cmpu    STREND
        lbls    NATIVE_FAIL
        leas    -16,s
        tfr     s,u
        jsr     VALUE_CAPTURE
        ldb     2,u
        stb     13,u
        ldb     3,u
        stb     7,u
        stx     2,u
        clra
        clrb
        std     5,u
        decb
        stb     4,u
USING_NEXT
        ldb     13,u
        lsrb
        jsr     VALUE_REFERENCE_CARRY
        ; Compute the live extent once. Subtracting the clamped remainder
        ; obtains the cursor without rereading the descriptor or its payload.
        ldb     ,y
        cmpb    4,u
        bhs     USING_REMAINING
        stb     4,u
USING_REMAINING
        tst     4,u
        beq     USING_PATTERN_END
        clra
        addd    2,y
        std     14,u
        subb    4,u
        sbca    #0
        tfr     d,x
        lbsr    USING_FIELD
        bcc     USING_VALUE
        lda     ,x
        dec     4,u
        jsr     CHROUT
        bra     USING_NEXT
USING_PATTERN_END
        ldx     2,u
        jsr     EX_END
        beq     USING_FINISH
        tst     5,u
        lbeq    NATIVE_RANGE_ERROR
        clr     5,u
        dec     4,u
        bra     USING_NEXT
USING_VALUE
        ; A parsed field already owns its exact consumed width.
        ldb     4,u
        subb    8,u
        stb     4,u
        ldx     2,u
        jsr     EX_END
        beq     USING_FINISH
        tst     9,u
        bmi     USING_ARGUMENT
        lbsr    USING_FIELD_LIMIT
USING_ARGUMENT
        jsr     EVAL_NATIVE
        stx     2,u
        tst     9,u
        bpl     USING_NUMERIC
        jsr     EX_STRING_CHECK
        ldx     STR_PTR
        ldb     FAC
USING_STRING
        lda     #32
        tstb
        beq     USING_STRING_OUT
        lda     ,x+
        decb
USING_STRING_OUT
        jsr     CHROUT
        dec     8,u
        bne     USING_STRING
        jsr     STRING_RELEASE
        bra     USING_VALUE_END
USING_NUMERIC
        jsr     EX_NUMBER_CHECK
        leau    8,u
        ldy     #USING_EMIT
        jsr     USING_NUMBER
        leau    -8,u
USING_VALUE_END
        inc     5,u
        clr     6,u
        ldx     2,u
        jsr     EX_END
        beq     USING_NEXT
        cmpa    #';
        beq     USING_SEPARATOR
        cmpa    #',
        lbne    NATIVE_FAIL
USING_SEPARATOR
        leax    1,x
        stx     2,u
        inc     6,u
        lbra    USING_NEXT
USING_FINISH
        tst     6,u
        bne     USING_RETURN
        lda     #13
        jsr     CHROUT
USING_RETURN
        ldx     2,u
        ldb     7,u
        jsr     STRING_TEMP_REWIND
        leas    16,s
        clra
        rts

; C set means literal text, with X unchanged. The bounded fetch consumes its
; lookahead once; phases pass that byte onwards rather than rereading it.
USING_FIELD
        pshs    x,y
        clra
        ldb     #-1
        stb     10,u
        bsr     USING_FIELD_GET
        cmpb    #'!
        beq     USING_STRING_FIELD
        cmpb    #'%
        beq     USING_PERCENT
        cmpb    #'+
        bne     USING_PREFIX
        ora     #UF_PLUS
        bsr     USING_FIELD_GET
USING_PREFIX
        cmpb    #'*
        beq     USING_STARS_MATCH
        cmpb    #'$
        beq     USING_DOLLARS_MATCH
        cmpb    #'#
        beq     USING_INTEGER_ADVANCE
        cmpb    #'.
        bne     USING_LITERAL
        cmpx    14,u
        bhs     USING_LITERAL
        ldb     ,x
        cmpb    #'#
        beq     USING_POINT_START
USING_LITERAL
        coma
        puls    x,y,pc
USING_PERCENT
        bsr     USING_FIELD_GET
        cmpb    #'%
        beq     USING_STRING_FIELD
        cmpb    #32
        beq     USING_PERCENT
        bra     USING_LITERAL
USING_STRING_FIELD
        lda     #UF_STRING
        bra     USING_FIELD_DONE
USING_STARS_MATCH
        ora     #UF_STARS
USING_DOLLARS_MATCH
        cmpx    14,u
        bhs     USING_LITERAL
        cmpb    ,x+
        bne     USING_LITERAL
        bita    #UF_STARS
        beq     USING_CURRENCY
        bsr     USING_FIELD_GET
        cmpb    #'$
        bne     USING_DIGIT_FIELD
USING_CURRENCY
        ora     #UF_CURRENCY
USING_INTEGER_ADVANCE
        bsr     USING_FIELD_GET
USING_DIGIT_FIELD
        cmpb    #'#
        beq     USING_INTEGER_ADVANCE
        cmpb    #',
        bne     USING_INTEGER_END
        ora     #UF_COMMA
        bra     USING_INTEGER_ADVANCE
USING_INTEGER_END
        leay    -1,x
        cmpb    #'.
        bne     USING_EXP_FIELD
        bra     USING_POINT
USING_POINT_START
        leay    -1,x
USING_POINT
        clr     10,u
USING_FRACTION_ADVANCE
        bsr     USING_FIELD_GET
        cmpb    #'#
        bne     USING_EXP_FIELD
        inc     10,u
        bra     USING_FRACTION_ADVANCE
USING_EXP_FIELD
        cmpb    #'^ 
        bne     USING_TRAILER
        pshs    y
        leay    2,x
        cmpy    14,u
        puls    y
        bhs     USING_TRAILER
        cmpb    ,x
        bne     USING_TRAILER
        cmpb    1,x
        bne     USING_TRAILER
        cmpb    2,x
        bne     USING_TRAILER
        leax    3,x
        ora     #UF_EXP
        bsr     USING_FIELD_GET
USING_TRAILER
        bita    #UF_PLUS
        bne     USING_UNREAD
        cmpb    #'-
        beq     USING_TRAIL_MINUS
        cmpb    #'+
        bne     USING_UNREAD
        ora     #UF_TRAIL_PLUS
USING_TRAIL_MINUS
        ora     #UF_TRAIL
        bra     USING_FIELD_DONE
USING_UNREAD
        leax    -1,x
USING_FIELD_DONE
        sta     9,u
        tfr     y,d
        subd    ,s
        stb     11,u
        tfr     x,d
        subd    ,s
        stb     8,u
        leas    2,s
        clra
        puls    y,pc

; Even the virtual end byte advances once, so declining lookahead always
; steps back by one. No byte outside the descriptor's extent is read.
USING_FIELD_GET
        clrb
        cmpx    14,u
        bhs     USING_FIELD_END
        ldb     ,x
USING_FIELD_END
        leax    1,x
        rts

USING_FIELD_LIMIT
        ldb     11,u
        tst     10,u
        bmi     USING_BOUND
        addb    10,u
        incb
USING_BOUND
        cmpb    #17
        lbhi    NATIVE_RANGE_ERROR
        rts

; U's first four bytes hold width, flags, fractional places and integer width.
; The common decimal converter supplies exact digits; both print forms share
; nine-significant-digit presentation. The retained 46-byte frame has six
; renderer scratch bytes below U; no duplicate FAC is needed. Reverse output may overwrite only digits
; already consumed; the longest fixed field has 16 fractional places, at most
; ten integer digits after rounding, one point and one trailing sign. Commas consume field
; positions and reduce that fractional maximum. The original stack guard is
; deliberately retained, so this compaction does not widen the accepted heap.
USING_NUMBER
        ; Y is a fixed native consumer of the completed character span.
        pshs    x,y,u
        leax    -148,s
        cmpy    #VALUE_COPY
        bne     USING_NUMBER_ROOM
        ; STR$ previously owned a 32-byte copy buffer and two extra return
        ; words. Retain its exact rejection threshold without that second copy.
        leax    -36,x
USING_NUMBER_ROOM
        cmpx    STREND
        lbls    NATIVE_FAIL
        ldd     ,u
        ldy     2,u
        ; The copied configuration is the first four bytes of this frame.
        ; Push it once instead of allocating and storing those bytes again.
        leas    -42,s
        pshs    d,y
        leau    12,s
        clr     -8,u
        tst     FSIGN
        bpl     USING_POSITIVE
        lda     #'-
        bra     USING_SIGN_STORE
USING_POSITIVE
        ; B still holds the copied field flags; the sign test preserves it.
        bitb    #UF_PLUS
        beq     USING_DIGITS
        lda     #'+
        tst     -12,u
        bne     USING_SIGN_STORE
        lda     #32
USING_SIGN_STORE
        sta     -8,u
USING_DIGITS
        clr     14,u
        tst     -12,u
        bne     USING_DECIMAL_START
        lda     -11,u
        sta     14,u
USING_DECIMAL_START
        leay    4,u
        jsr     FORMAT_DECIMAL
        sta     ,u
        tst     -12,u
        lbeq    USING_CANONICAL

USING_DECIMAL_READY
        lda     -11,u
        bita    #UF_EXP
        bne     USING_PRESENTATION_ROUND
        lda     ,u
        cmpa    #9
        lbge    USING_FALLBACK
USING_PRESENTATION_ROUND
        jsr     USING_ROUND_NINE
USING_PRESENTATION_READY
        clr     -7,u
        lda     4,u
        ; Carry records zero until the scientific displacement is formed.
        ; Field flag tests and byte increments preserve that predicate.
        cmpa    #'1
        bcc     USING_NONZERO
        dec     ,u
USING_NONZERO
        lda     -11,u
        bita    #UF_EXP
        beq     USING_ROUND_FIELD
        ldb     -9,u
        decb
        bita    #UF_TRAIL
        beq     USING_EXP_CURRENCY
        incb
USING_EXP_CURRENCY
        bita    #UF_CURRENCY
        beq     USING_EXP_POSITION
        tst     -10,u
        ble     USING_EXP_POSITION
        decb
USING_EXP_POSITION
        decb
        stb     1,u
        bcs     USING_EXP_READY
        lda     ,u
        suba    1,u
        sta     -7,u
        stb     ,u
USING_EXP_READY
        ; Integer-only exponent fields have an observed overflow presentation
        ; once their mantissa position reaches the ordinary nine-digit bound.
        tst     -10,u
        bgt     USING_ROUND_FIELD
        lda     1,u
        cmpa    #9
        blt     USING_ROUND_FIELD
        ldb     #1
        stb     -12,u
        ldb     4,u
        cmpb    #'0
        beq     USING_WIDE_ZERO
        adda    -10,u
        sta     ,u
USING_WIDE_ZERO
        ldb     #-1
        stb     -10,u
        bra    USING_RENDER

USING_ROUND_FIELD
        ldb     -10,u
        bpl     USING_ADD_PLACES
        clrb
USING_ADD_PLACES
        addb    ,u
        incb
        cmpb    #9
        ble     USING_ROUND_VALID
        ldb     #9
USING_ROUND_VALID
        tstb
        bmi     USING_RENDER
        ; The original exponent is the displayed exponent plus its scientific
        ; displacement. Retain only the decimal scale, not two more copies of
        ; those same quantities. Exact digit extraction has retained FAC.
        decb
        subb    ,u
        subb    -7,u
        stb     2,u
        ; The byte accumulator owns the scale calculation without a stack
        ; sum. EX_ABS clears B, so the persisted scale is reloaded afterwards.
        jsr     EX_ABS
USING_SCALE_NUMBER
        ldb     2,u
        jsr     SCALE_DECIMAL

USING_HALF_INTEGER
        jsr     FP_RUN
        fcb     FP_KARG+FPC_HALF,FP_ADD,FP_FIX,FP_END
        leay    4,u
        jsr     FORMAT_DECIMAL
        suba    2,u
        suba    -7,u
        sta     ,u
        lda     4,u
        cmpa    #'0
        bne     USING_RENDER
        lda     #-1
        sta     ,u
USING_RENDER
; Build the bounded field backwards in stack data. This directly supplies its
; final width without changing the global STR$ / cassette formatting route.
USING_RENDER_BUFFER
USING_PLAN_INTEGER
        ldb     ,u
        incb
        bgt     USING_PLAN_READY
        clrb
        lda     -10,u
        ble     USING_FORCE_ZERO
        lda     -9,u
        tst     -8,u
        beq     USING_ZERO_CURRENCY
        deca
USING_ZERO_CURRENCY
        pshs    a
        lda     -11,u
        bita    #UF_CURRENCY
        puls    a
        beq     USING_ZERO_ROOM
        deca
USING_ZERO_ROOM
        tsta
        ble     USING_PLAN_READY
USING_FORCE_ZERO
        incb
USING_PLAN_READY
        stb     -6,u
        leax    33,u
        clr     ,x
        lsr     -11,u
        bcc     USING_RENDER_EXPONENT
        lda     -8,u
        bne     USING_TRAILER_OUT
        lda     #32
        ldb     -11,u
        bitb    #UF_TRAIL_PLUS/2
        beq     USING_TRAILER_OUT
        lda     #'+
USING_TRAILER_OUT
        sta     ,-x
        clr     -8,u
USING_RENDER_EXPONENT
        lsr     -11,u
        bcc     USING_RENDER_FRACTION
        ldb     -7,u
        bpl     USING_EXP_ABS
        negb
USING_EXP_ABS
        lda     #'0
USING_EXP_TENS
        cmpb    #10
        blo     USING_EXP_OUT
        subb    #10
        inca
        bra     USING_EXP_TENS
USING_EXP_OUT
        addb    #'0
        std     ,--x
        ldb     #'+
        tst     -7,u
        bpl     USING_EXP_SIGN
        ldb     #'-
USING_EXP_SIGN
        lda     #'E
        std     ,--x
USING_RENDER_FRACTION
        lda     ,u
        sta     -5,u
        ldb     -10,u
        bmi     USING_INTEGER_COUNT
        adda    -10,u
        sta     -5,u
        tstb
        beq     USING_POINT_OUT
USING_FRACTION_LOOP
        lbsr     USING_OUTPUT_DIGIT
        decb
        bne     USING_FRACTION_LOOP
USING_POINT_OUT
        lda     #'.
        sta     ,-x
USING_INTEGER_COUNT
        ldb     -6,u
USING_INTEGER_READY
        beq     USING_RENDER_CURRENCY
        ; A comma period of three or a sentinel of 255 owns this decision.
        ; The bounded integer span cannot exhaust the no-comma sentinel.
        lda     -11,u
        anda    #UF_COMMA/4
        deca
        sta     3,u
USING_INTEGER_LOOP
        bsr     USING_OUTPUT_DIGIT
        decb
        beq     USING_RENDER_CURRENCY
        dec     3,u
        bne     USING_INTEGER_LOOP
        lda     #3
        sta     3,u
        lda     #',
        sta     ,-x
        bra     USING_INTEGER_LOOP
USING_RENDER_CURRENCY
        lsr     -11,u
        bcc     USING_RENDER_SIGN
        lda     #'$
        sta     ,-x
USING_RENDER_SIGN
        lda     -8,u
        beq     USING_OUTPUT
        sta     ,-x
USING_OUTPUT
        leay    33,u
        pshs    x
        tfr     y,d
        subd    ,s++
        ; Numeric width is bounded to 22, keeping this difference signed.
        subb    -12,u
        ble     USING_PADDING
        tst     -12,u
        beq     USING_TEXT
        lda     #'%
        sta     ,-x
        ; Native tail continues into the following block.
USING_TEXT
        ; The native consumer runs while the completed stack span is live.
        ; Ordinary width zero also leaves D as its exact character count.
        jsr     [48,s]
USING_NUMBER_END
        leas    46,s
        puls    x,y,u,pc
USING_PADDING
        negb
        lsr     -11,u
        bcc     USING_PAD_SPACE
        lda     #'*
        bra     USING_PAD
USING_PAD_SPACE
        lda     #32
USING_PAD
        tstb
        beq     USING_TEXT
        sta     ,-x
        decb
        bra     USING_PAD
USING_EMIT
        jsr     PUTS
USING_TRAILING_SPACE
        ; Long USING fields may reuse the now-dead digit/scratch bytes. The
        ; ordinary 15-character maximum keeps its trailing-blank flag intact.
        tst     -12,u
        bne     USING_EMIT_END
        tst     14,u
        ble     USING_EMIT_END
        lda     #32
        jmp     FORMAT_OUT
USING_EMIT_END
        rts
USING_OUTPUT_DIGIT
        lda     -5,u
        dec     -5,u
        cmpa    #9
        bhs     USING_DIGIT_ZERO
        leay    4,u
        lda     a,y
        bra     USING_DIGIT_OUT
USING_DIGIT_ZERO
        lda     #'0
USING_DIGIT_OUT
        sta     ,-x
        rts

USING_FALLBACK
        lda     #'%
        jsr     FORMAT_OUT
        lda     -8,u
        cmpa    #'-
        beq     USING_FALLBACK_SIGN
        lda     #32
USING_FALLBACK_SIGN
        sta     -8,u
        clr     -12,u
; The ordinary and overflow paths use the same decimal field renderer. Their
; width is unconstrained, insignificant zeroes are trimmed, and exponent form
; is selected at the ordinary BASIC thresholds.
USING_CANONICAL
        jsr     USING_ROUND_NINE
USING_CANONICAL_TRIM
        leax    13,u
        ldb     #9
USING_TRIM_NEXT
        cmpb    #1
        beq     USING_CANONICAL_LAYOUT
        lda     ,-x
        cmpa    #'0
        bne     USING_CANONICAL_LAYOUT
        decb
        bra     USING_TRIM_NEXT
USING_CANONICAL_LAYOUT
        clr     -11,u
        clr     -9,u
        lda     ,u
        sta     -7,u
        cmpa    #9
        bge     USING_CANONICAL_EXP
        cmpa    #-2
        bge     USING_CANONICAL_FIXED
USING_CANONICAL_EXP
        lda     #UF_EXP
        sta     -11,u
        clr     ,u
USING_CANONICAL_FIXED
        subb    ,u
        decb
        bgt     USING_CANONICAL_PLACES
        ldb     #-1
USING_CANONICAL_PLACES
        stb     -10,u
        jmp     USING_RENDER_BUFFER

; Both presentation routes round the same ten-digit buffer and its exponent.
USING_ROUND_NINE
        leax    4,u
        ldb     #9
        jsr     DECIMAL_ROUND
        bcc     USING_ROUND_NINE_END
        inc     ,u
USING_ROUND_NINE_END
        rts
