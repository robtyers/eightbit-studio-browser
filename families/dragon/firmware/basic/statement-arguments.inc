; Native carry-return bridges for non-parser interfaces. Apache-2.0.
ST_LINE_NUMBER
        jsr     PARSE_LINE_NUMBER
ST_CHECK
        bcs     NATIVE_ARGUMENT_ERROR
        rts
NATIVE_ARGUMENT_ERROR
        jmp     NATIVE_FAIL

EVAL_NATIVE equ EX_BODY
VAR_FIND_NATIVE
        jsr     VAR_FIND
        bra     ST_CHECK
; Required native grammar. A is the expected delimiter; no opcode is decoded.
MULTI_CLOSE
        lda     #')
        bra     EX_EXPECT
MULTI_COMMA
        lda     #',
        bra     EX_EXPECT
EX_EQUALS
        lda     #$cb
EX_EXPECT
        pshs    a
        jsr     EX_SPACE
        cmpa    ,s+
        bne     NATIVE_ARGUMENT_ERROR
        leax    1,x
        rts

; LEAX changes Z; repeat the successful comparison to restore its exact flags.
EX_OPTIONAL_COMMA
        jsr     EX_COMMA
        bne     EX_OPTIONAL_END
        leax    1,x
        cmpa    #',
EX_OPTIONAL_END
        rts

NATIVE_CHECK_NE
        bne     NATIVE_ARGUMENT_ERROR
        rts

NATIVE_CHECK_HS
        bhs     NATIVE_ARGUMENT_ERROR
        rts

NATIVE_CHECK_HI
        bhi     NATIVE_ARGUMENT_ERROR
        rts

EX_STRING_CHECK
        tst     $06
        lbeq    NATIVE_TYPE_ERROR
        rts

; D is the exclusive live-data end. Keep the original 128-byte parser margin
; below the caller's S; account for our return word and saved argument exactly.
NATIVE_STACK_ROOM
        pshs    d
        tfr     s,d
        subd    #124
        subd    ,s++
        bls     NATIVE_ARGUMENT_ERROR
        rts

