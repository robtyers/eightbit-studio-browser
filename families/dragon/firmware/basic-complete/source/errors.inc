; Original guest diagnostics. Codes/text are observable BASIC interface facts.
; Internal arithmetic helpers return carry; their expression boundaries select
; language errors. Published signed-word conversions raise TM/FC directly.
NATIVE_TYPE_ERROR
        ldb     #24
        bra     NATIVE_SET_ERROR
NATIVE_RANGE_ERROR
        ldb     #8
        bra     NATIVE_SET_ERROR
NATIVE_OVERFLOW_ERROR
        ldb     #10
        bra     NATIVE_SET_ERROR
NATIVE_ZERO_ERROR
        ldb     #20
NATIVE_SET_ERROR
        stb     ERROR_CODE
        jmp     NATIVE_FAIL


NATIVE_NEXT_ERROR
        ldb     #0
        bra     NATIVE_SET_ERROR

NATIVE_RETURN_ERROR
        ldb     #4
        bra     NATIVE_SET_ERROR

NATIVE_CONT_ERROR
        ldb     #32
        bra     NATIVE_SET_ERROR

NATIVE_DIMENSION_ERROR
        ldb     #18
        bra     NATIVE_SET_ERROR

NATIVE_SUBSCRIPT_ERROR
        ldb     #16
        bra     NATIVE_SET_ERROR

NATIVE_MEMORY_ERROR
        ldb     #12
        bra     NATIVE_SET_ERROR

NATIVE_UNDEFINED_FN_ERROR
        ldb     #34
        bra     NATIVE_SET_ERROR

; Public arithmetic retains its carry contract; only the BASIC EXP boundary
; converts a failed positive exponent to the language overflow diagnostic.
EX_EXP
        jsr     F_EXP
        bcs     NATIVE_OVERFLOW_ERROR
        rts

EX_TAN
        jsr     F_TAN
        bcs     NATIVE_OVERFLOW_ERROR
        rts

; Internal language failures already use the published even-byte selectors.
LEGACY_ERROR_HOOK
        stx     TXT_PTR
        ldb     ERROR_CODE
        cmpb    #50
        bls     LEGACY_ERROR_READY
        ldb     #2
LEGACY_ERROR_READY
        jsr     $018e
        stb     ERROR_CODE
        rts
