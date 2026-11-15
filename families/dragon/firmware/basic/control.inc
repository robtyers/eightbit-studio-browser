; Original control flow. Apache-2.0. Frame layouts are black-box interface facts:
; FOR: tag80,var2,step-magnitude5,direction1,packed-limit5,line2,text2.
; GOSUB: tagBD,line2,text2,implementation-return2. Never copy OEM return targets.
CMD_GOSUB
        jsr     EX_NEXT_SPACE
        pshs    x
        jsr     ST_LINE_NUMBER
        bsr     CTL_FIND_LINE
        jsr     ST_CHECK
        puls    x
        ldu     CTL_SP
        leau    -7,u
        pshs    u
        ldd     STREND
        addd    #128
        cmpd    ,s++
        jsr     NATIVE_CHECK_HS
        stx     3,u
        ldd     CURLINE
        std     1,u
        ldd     #EXEC_AFTER_STATEMENT
        std     5,u
        lda     #$bd
        sta     ,u
        tfr     u,s
        sts     CTL_SP
        bsr     CTL_GO_TARGET
        clra
        jmp     EXEC_AFTER_STATEMENT
CTL_FIND_LINE
        jsr     VALIDATE_TEXT
        bcs     CTL_FIND_ERROR
        ldy     TXTTAB
CTL_FIND_LOOP
        ldd     ,y
        beq     CTL_FIND_ERROR
        ldd     2,y
        cmpd    LINE_NUMBER
        beq     CTL_FIND_OK
        bhi     CTL_FIND_ERROR
        ldy     ,y
        bra     CTL_FIND_LOOP
CTL_FIND_ERROR
        coma
        rts
CTL_FIND_OK
        clra
        rts
CTL_GO_TARGET
        ldd     2,y
        std     CURLINE
        ldd     ,y
        std     EXEC_NEXT
        leax    4,y
        lda     #1
        sta     EXEC_MODE
        sta     EXEC_JUMP
        rts
LOCAL_FAILURE_1
        jmp    NATIVE_FAIL
CMD_FOR
        leas    -18,s
        tfr     s,u
        jsr     VAR_FIND_NATIVE
        tst     $06
        bne     LOCAL_FAILURE_1
        cmpy    ARYTAB
        bhs     LOCAL_FAILURE_1
        sty     1,u
        jsr     EX_EQUALS
        jsr     EVAL_NUMBER
        jsr     EX_SPACE
        cmpa    #$bc
        bne     LOCAL_FAILURE_1
        ldy     1,u
        jsr     VALUE_STORE
        leax    1,x
        jsr     EVAL_NUMBER
        leay    9,u
        jsr     VALUE_STORE
        jsr     EX_SPACE
        cmpa    #$c1
        beq     CTL_FOR_STEP
        jsr     FP_RUN
        fcb     FP_KLOAD+0,FP_END
        bra     CTL_FOR_HAVE_STEP
CTL_FOR_STEP
        leax    1,x
        jsr     EVAL_NUMBER
CTL_FOR_HAVE_STEP
        jsr     CMD_REQUIRE_END
CTL_FOR_SAVE
        stx     16,u
        ldd     CURLINE
        std     14,u
        lda     #$80
        sta     ,u
        ldx     #FAC
        leay    3,u
        jsr     FAC_UNPACKED_COPY
        clra
        tst     FAC
        beq     CTL_FOR_ZERO
        inca
        tst     FSIGN
        bpl     CTL_FOR_DIRECTION
        nega
        bra     CTL_FOR_DIRECTION
CTL_FOR_ZERO
        ldd     #$8000
        std     4,u
        clra
CTL_FOR_DIRECTION
        sta     8,u
        ; Re-entering FOR for an active scalar replaces that frame and younger FORs.
        ldy     CTL_SP
CTL_FOR_SEARCH
        cmpy    CTL_BASE
        bhs     CTL_FOR_ALLOCATE
        lda     ,y
        cmpa    #$80
        bne     CTL_FOR_ALLOCATE
        ldd     1,y
        cmpd    1,u
        beq     CTL_FOR_REPLACE
        leay    18,y
        bra     CTL_FOR_SEARCH
CTL_FOR_REPLACE
        leay    18,y
        sty     CTL_SP
CTL_FOR_ALLOCATE
        ldy     CTL_SP
        leay    -18,y
        tfr     y,d
        subd    #128
        cmpd    STREND
        bls     NATIVE_SHARED12_CONTROL_INC_0
        leay    18,y
        leau    18,u
        ldb     #18
CTL_FOR_COPY
        lda     ,-u
        sta     ,-y
        decb
        bne     CTL_FOR_COPY
        tfr     y,s
        sts     CTL_SP
        ldx     16,y
        clra
        jmp     EXEC_AFTER_STATEMENT
LOCAL_FAILURE_8
        jmp     CMD_UNSUPPORTED

CMD_NEXT
        jsr     EX_ALPHA
        ldy     #0
        bcc     CTL_NEXT_FIND
        jsr     VAR_FIND_NATIVE
        tst     $06
        bne     LOCAL_FAILURE_8
        cmpy    ARYTAB
        bhs     LOCAL_FAILURE_8
CTL_NEXT_FIND
        ldu     CTL_SP
CTL_NEXT_SEARCH
        cmpu    CTL_BASE
        bhs     LOCAL_FAILURE_8
        lda     ,u
        cmpa    #$80
        bne     LOCAL_FAILURE_8
        leay    ,y
        beq     CTL_NEXT_FRAME
        pshs    y
        ldd     1,u
        cmpd    ,s++
        beq     CTL_NEXT_FRAME
        leau    18,u
        bra     CTL_NEXT_SEARCH
NATIVE_SHARED12_CONTROL_INC_0
        jmp     NATIVE_FAIL
CTL_NEXT_FRAME
        pshs    x
        ldx     1,u
        jsr     FAC_LOAD
        leax    3,u
        jsr     ARG_FROM_ROM
        jsr     F_ADD
        bcs     NATIVE_SHARED12_CONTROL_INC_0
        ldx     1,u
        jsr     FAC_STORE
        jsr     F_SWAP
        leax    9,u
        jsr     FAC_LOAD
        jsr     F_SWAP
        jsr     F_COMPARE
        puls    x
        tst     8,u
        beq     CTL_NEXT_REPEAT
        bmi     CTL_NEXT_NEGATIVE
        cmpa    #1
        bne     CTL_NEXT_REPEAT
        bra     CTL_NEXT_DONE
CTL_NEXT_NEGATIVE
        cmpa    #-1
        bne     CTL_NEXT_REPEAT
CTL_NEXT_DONE
        leau    18,u
        stu     CTL_SP
        lds     CTL_SP
        jsr     EX_OPTIONAL_COMMA
        bne     CTL_NEXT_COMPLETE
        jmp     CMD_NEXT
CTL_NEXT_COMPLETE
        clra
        jmp     EXEC_AFTER_STATEMENT
CTL_NEXT_REPEAT
        pshs    u
        ldd     14,u
        bsr     CTL_RESTORE_LINE
        puls    u
        bcs     NATIVE_SHARED12_CONTROL_INC_0
        ldx     16,u
        stu     CTL_SP
        jmp     CTL_RESUME

CMD_RETURN
        ldu     CTL_SP
CTL_RETURN_FIND
        cmpu    CTL_BASE
        bhs     LOCAL_FAILURE_9
        lda     ,u
        cmpa    #$bd
        beq     CTL_RETURN_FRAME
        cmpa    #$80
        bne     LOCAL_FAILURE_9
        leau    18,u
        bra     CTL_RETURN_FIND
CTL_RETURN_FRAME
        pshs    u
        ldd     1,u
        bsr     CTL_RESTORE_LINE
        puls    u
        bcs     LOCAL_FAILURE_9
        ldx     3,u
CTL_RETURN_SKIP_TARGET
        lda     ,x
        beq     CTL_RETURN_DONE
        cmpa    #':
        beq     CTL_RETURN_DONE
        leax    1,x
        bra     CTL_RETURN_SKIP_TARGET
CTL_RETURN_DONE
        leau    7,u
        stu     CTL_SP
        bra     CTL_RESUME
CTL_RESTORE_LINE
        std     CURLINE
        cmpd    #$ffff
        beq     CTL_RESTORE_DIRECT
        std     LINE_NUMBER
        jsr     CTL_FIND_LINE
        bcs     CTL_RESTORE_ERROR
        ldd     ,y
        std     EXEC_NEXT
        lda     #1
        sta     EXEC_MODE
        rts
LOCAL_FAILURE_9
        jmp     CMD_UNSUPPORTED
CTL_RESTORE_DIRECT
        clr     EXEC_MODE
CTL_RESTORE_ERROR
        rts

CMD_CONT
        jsr     CMD_REQUIRE_END
        tst     CONT_VALID
        beq     LOCAL_FAILURE_9
        ldd     $29
        std     CURLINE
        ldx     $2f
        ldd     CONT_NEXT
        std     EXEC_NEXT
        lda     #1
        sta     EXEC_MODE
        ldd     CONT_SP
        std     CTL_SP
CTL_RESUME
        lds     CTL_SP
        lda     #1
        sta     EXEC_JUMP
        clra
        jmp     EXEC_AFTER_STATEMENT

