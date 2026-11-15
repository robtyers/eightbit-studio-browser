; Original guest diagnostics. Codes/text are observable BASIC interface facts.
; Arithmetic ABI calls still return carry without changing interpreter state;
; only their native expression/argument boundaries select a language error.
NATIVE_TYPE_ERROR
        ldb     #1
        bra     NATIVE_SET_ERROR
NATIVE_RANGE_ERROR
        ldb     #2
        bra     NATIVE_SET_ERROR
NATIVE_OVERFLOW_ERROR
        ldb     #3
        bra     NATIVE_SET_ERROR
NATIVE_ZERO_ERROR
        ldb     #4
NATIVE_SET_ERROR
        stb     ERROR_CODE
        jmp     NATIVE_FAIL

BASIC_ERROR
        lda     #'?
        jsr     CHROUT
        ldb     ERROR_CODE
        cmpb    #4
        bls     ERROR_SELECTED
        clrb
ERROR_SELECTED
        lslb
        ldx     #ERROR_CODES
        abx
        lda     ,x+
        jsr     CHROUT
        lda     ,x
        jsr     CHROUT
        ldx     #ERROR_SUFFIX
        jsr     PUTS
ERROR_LOCATION
        ldd     CURLINE
        cmpd    #$ffff
        beq     ERROR_NEWLINE
        pshs    d
        ldx     #ERROR_IN
        jsr     PUTS
        puls    d
        jsr     PRINT_UNSIGNED
ERROR_NEWLINE
        lda     #13
        jmp     CHROUT

