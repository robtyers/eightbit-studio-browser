; Original base Dragon 32 DLOAD grammar. Exact-profile synthetic observations
; establish IO without a transfer and FC for a supplied numeric selector.
; This does not implement the Dragon 64 serial facility or a network device.
CMD_DLOAD
        ; The base ROM closes an open file before even validating its arguments.
        jsr     DATA_CLOSE
        jsr     EX_UPCASE
        cmpa    #'M
        bne     DLOAD_NAME
        leax    1,x
DLOAD_NAME
        jsr     EX_END
        beq     DLOAD_UNAVAILABLE
        jsr     EVAL_STRING
        jsr     STRING_RELEASE
        jsr     EX_OPTIONAL_COMMA
        bne     DLOAD_END
        jsr     EVAL_NUMBER
        jmp     NATIVE_RANGE_ERROR
DLOAD_END
        jsr     CMD_REQUIRE_END
DLOAD_UNAVAILABLE
        ldb     #42
        jmp     NATIVE_SET_ERROR
