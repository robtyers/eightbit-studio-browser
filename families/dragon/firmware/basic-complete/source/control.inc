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
        ; The validated GOSUB owns the control stack now. Construct its
        ; published seven-byte record by native pushes, before leaving S there.
        ldd     CTL_SP
        subd    #7+128
        lbcs    NATIVE_MEMORY_ERROR
        cmpd    STREND
        lbls    NATIVE_MEMORY_ERROR
        lds     CTL_SP
        ldu     #EXEC_AFTER_STATEMENT
        ldd     CURLINE
        pshs    d,x,u
        ldb     #$bd
        pshs    b
        sts     CTL_SP
        bsr     CTL_GO_TARGET
        bra     CTL_STATEMENT_DONE
CTL_FIND_LINE
        jsr     VALIDATE_TEXT
        bcs     CTL_FIND_RETURN_ERROR
        jsr     PROGRAM_FIND
        beq     CTL_FIND_OK
CTL_FIND_ERROR
        ldb     #14
        stb     ERROR_CODE
CTL_FIND_RETURN_ERROR
        coma
        rts
CTL_FIND_OK
        clra
        rts
CTL_GO_TARGET
        sty     EXEC_NEXT
        leax    -1,y
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
        fcb     FP_KLOAD+FPC_ONE,FP_END
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
        ; Shared bounded frame lookup retains the provisional FOR record.
        ldy     1,u
        ldb     #$80
        bsr     CTL_FIND_FRAME
        tfr     u,y
        ; Balanced argument calls leave the provisional frame at S.
        tfr     s,u
        bcc     CTL_FOR_CHECK
CTL_FOR_ALLOCATE
        ldy     CTL_SP
        leay    -18,y
CTL_FOR_CHECK
        ; A replacement already names its destination. Publish only the
        ; completed control record, without changing and reloading CTL_SP.
        tfr     y,d
        subd    #128
        cmpd    STREND
        lbls    NATIVE_MEMORY_ERROR
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
        bra     CTL_STATEMENT_DONE
LOCAL_FAILURE_8
        jmp     NATIVE_NEXT_ERROR
CTL_NEXT_VARIABLE
        jsr     VAR_FIND_NATIVE
        tst     $06
        bne     LOCAL_FAILURE_8
        cmpy    ARYTAB
        bhs     LOCAL_FAILURE_8
CTL_NEXT_FIND
        ldb     #$80
        bsr     CTL_FIND_FRAME
        bcs     LOCAL_FAILURE_8
        ; Native tail continues into the following block.
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
CTL_NEXT_COMPARE
        ; Compare returns -1/0/+1. Terminate only for a non-zero comparison
        ; whose sign matches a non-zero step, including edited public frames.
        tsta
        beq     CTL_NEXT_REPEAT
        tst     8,u
        beq     CTL_NEXT_REPEAT
        eora    8,u
        bmi     CTL_NEXT_REPEAT
CTL_NEXT_DONE
        leau    18,u
        tfr     u,s
        sts     CTL_SP
        jsr     EX_OPTIONAL_COMMA
        bne     CTL_NEXT_COMPLETE
        ; Native tail continues into the following block.

CMD_NEXT
        jsr     EX_ALPHA
        bcs     CTL_NEXT_VARIABLE
        jsr     CMD_REQUIRE_END
        ldy     #0
        bra     CTL_NEXT_FIND
NATIVE_SHARED12_CONTROL_INC_0
        jmp     NATIVE_FAIL
CTL_NEXT_COMPLETE
        bra     CTL_STATEMENT_DONE
CTL_NEXT_REPEAT
        ; CTL_RESTORE_LINE and its checked programme walkers preserve U.
        ldd     14,u
        bsr     CTL_RESTORE_LINE
        bcs     NATIVE_SHARED12_CONTROL_INC_0
        ldx     16,u
        bra     CTL_RESUME

CMD_RETURN
        jsr     CMD_REQUIRE_END
        ldb     #$bd
        bsr     CTL_FIND_FRAME
        bcs     LOCAL_FAILURE_9
CTL_RETURN_FRAME
        ; The same restore path preserves the live control-frame pointer.
        ldd     1,u
        bsr     CTL_RESTORE_LINE
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
        rts
LOCAL_FAILURE_9
        jmp     NATIVE_RETURN_ERROR
CTL_RESTORE_DIRECT
        clra
        clrb
        std     EXEC_NEXT
CTL_RESTORE_ERROR
        rts

CMD_CONT
        jsr     CMD_REQUIRE_END
        tst     CONT_VALID
        lbeq    NATIVE_CONT_ERROR
        ldd     $29
        std     CURLINE
        ldx     $2f
        ldd     CONT_NEXT
        std     EXEC_NEXT
        ldd     CONT_SP
        tfr     d,u
CTL_RESUME
        ; The resume owner publishes the same control stack before the hook.
        ; CONT retains D's established hook argument while transferring U.
        stu     CTL_SP
        tfr     u,s
        ldu     #$03e8
        jsr     EXEC_NEXT_HOOK
        lda     #2
        sta     EXEC_MODE
CTL_STATEMENT_DONE
        clra
        stx     TXT_PTR
        jmp     EXEC_AFTER_STATEMENT

; B selects FOR ($80, with Y=variable or zero) or GOSUB ($bd). Only FOR
; records may be skipped; the base bound is checked before each tag read.
CTL_FIND_FRAME
        ldu     CTL_SP
CTL_FIND_FRAME_NEXT
        cmpu    CTL_BASE
        bhs     CTL_FIND_FRAME_ERROR
        lda     ,u
        cmpa    #$80
        bne     CTL_FIND_FRAME_OTHER
        cmpb    #$80
        bne     CTL_FIND_FRAME_SKIP
        leay    ,y
        beq     CTL_FIND_FRAME_FOUND
        cmpy    1,u
        beq     CTL_FIND_FRAME_FOUND
CTL_FIND_FRAME_SKIP
        leau    18,u
        bra     CTL_FIND_FRAME_NEXT
CTL_FIND_FRAME_OTHER
        cmpa    #$bd
        bne     CTL_FIND_FRAME_ERROR
        cmpb    #$bd
        bne     CTL_FIND_FRAME_ERROR
CTL_FIND_FRAME_FOUND
        clra
        rts
CTL_FIND_FRAME_ERROR
        coma
        rts
