; Original native parser failure frame. Apache-2.0.
; Caller has saved D/Y/U, then supplies a native body in Y. X remains the token
; cursor, U is restored, and carry reports success/failure. No code is decoded.
; Frame: output mode0, reserved1, previous boundary2, temporary mark4, D6, Y8, U10, caller return12.
; DEVNUM is public live state: failures retain callbacks and parsed selectors.
; Internal inline entries take their native body from the JSR return word.
NATIVE_INLINE
        puls    y
NATIVE_FRAME
        ldd     TEMP_FREE
        pshs    d
        ldd     STATEMENT_SP
        pshs    d
        lda     FORMAT_BUFFER
        ldb     DEVNUM
        pshs    d
        sts     STATEMENT_SP
        ldd     6,s
        jsr     ,y
NATIVE_RETURN
        puls    d
        sta     FORMAT_BUFFER
        ldd     ,s++
        std     STATEMENT_SP
        leas    2,s
        puls    d,y,u,pc
NATIVE_FAIL
        tfr     s,d
        lds     STATEMENT_SP
        ; Only contexts in the discarded stack interval belong to this
        ; failure. Untouched private workspace may contain a foreign value.
        ldy     STR_ROOTS
NATIVE_TEXT_UNWIND
        beq     NATIVE_TEXT_READY
        pshs    d
        cmpy    ,s++
        blo     NATIVE_TEXT_READY
        cmpy    STATEMENT_SP
        bhs     NATIVE_TEXT_READY
        ldy     ,y
        bra     NATIVE_TEXT_UNWIND
NATIVE_TEXT_READY
        sty     STR_ROOTS
        ldb     5,s
        jsr     STRING_TEMP_REWIND
        coma
        bra     NATIVE_RETURN
