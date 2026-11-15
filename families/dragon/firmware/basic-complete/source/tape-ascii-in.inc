; ASCII programme input shares the public data-file character and record
; reader. Its EOF completes the load; syntax/IO errors retain the valid prefix
; and the open mode, exactly as an interrupted input operation does.
FILE_ASCII_LOAD
        lda     #1
        sta     $78
        jsr     DATA_REFILL
        lda     #$ff
        sta     DEVNUM
        bsr     FILE_ASCII_SCOPE
        bcc     FILE_ASCII_DONE
        jsr     MOTOROFF
        jmp     NATIVE_FAIL
FILE_ASCII_SCOPE
        pshs    d,y,u
        jsr     NATIVE_INLINE
FILE_ASCII_LINE
        jsr     DATA_READ_RECORD
        bcs     FILE_ASCII_EOF_INPUT
        jsr     EX_SPACE
        tsta
        beq     FILE_ASCII_LINE
        cmpa    #'0
        blo     FILE_ASCII_DIRECT
        cmpa    #'9
        bhi     FILE_ASCII_DIRECT
        jsr     STORE_SOURCE_LINE
        jsr     ST_CHECK
        bra     FILE_ASCII_LINE
FILE_ASCII_EOF_INPUT
        clr     $78
        lda     #$ff
        sta     EXEC_MODE
        clra
        rts
FILE_ASCII_DIRECT
        ldb     #50
        jmp     NATIVE_SET_ERROR
FILE_ASCII_DONE
        jmp     FILE_LOAD_STOP_MOTOR
