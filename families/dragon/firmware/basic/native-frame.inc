; Original native parser failure frame. Apache-2.0.
; Caller has saved D/Y/U, then supplies a native body in Y. X remains the token
; cursor, U is restored, and carry reports success/failure. No code is decoded.
; Frame: output mode0, device1, roots2, previous boundary4, D6, Y8, U10, caller return12.
; Internal inline entries take their native body from the JSR return word.
NATIVE_INLINE
        puls    y
NATIVE_FRAME
        ldd     STATEMENT_SP
        pshs    d
        ldd     STR_ROOTS
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
        std     STR_ROOTS
        ldd     ,s++
        std     STATEMENT_SP
        puls    d,y,u,pc
NATIVE_FAIL
        lds     STATEMENT_SP
        lda     1,s
        sta     DEVNUM
        coma
        bra     NATIVE_RETURN

