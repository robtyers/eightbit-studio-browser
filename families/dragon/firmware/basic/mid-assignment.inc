; Original MID$ assignment. All temporaries live on the guest stack.
; Frame: destination/array offset0, array flag2, start3, count4, token cursor6.
; Resolve an array descriptor again after expressions create scalar variables.
CMD_MID
        ldd     ,x++
        cmpd    #$ff98
        jsr     NATIVE_CHECK_NE
        jsr     EX_OPEN
        leas    -8,s
        tfr     s,u
        jsr     VAR_FIND_NATIVE
        jsr     EX_STRING_CHECK
        clr     2,u
        tfr     y,d
        cmpy    ARYTAB
        blo     MID_DESTINATION
        subd    ARYTAB
        inc     2,u
MID_DESTINATION
        std     ,u
        jsr     GFX_NEXT_BYTE
        tstb
        lbeq    STATEMENT_FAIL
        decb
        stb     3,u
        ldb     #255
        stb     4,u
        jsr     EX_OPTIONAL_COMMA
        bne     MID_CLOSE
        jsr     GFX_BYTE
        stb     4,u
MID_CLOSE
        jsr     MULTI_CLOSE
        jsr     EX_EQUALS
        bsr     MID_TARGET
        ldb     3,u
        cmpb    ,y
        jsr     NATIVE_CHECK_HS
        ldb     ,y
        subb    3,u
        cmpb    4,u
        bhs     MID_DETACH
        stb     4,u
MID_DETACH
        ; Heap values already belong to their variable. Detach programme data
        ; before RHS evaluation so literals stay intact and self-copy overlaps
        ; the writable value, as it does for an existing heap string.
        ldd     2,y
        cmpd    VARTAB
        bhs     MID_EVALUATE
        pshs    x,u
        tfr     y,x
        jsr     STRING_LOAD
        clr     STR_OWNER
        jsr     STRING_STORE
        puls    x,u
        jsr     ST_CHECK
MID_EVALUATE
        jsr     EVAL_NATIVE
        jsr     EX_STRING_CHECK
        jsr     CMD_REQUIRE_END
        stx     6,u
        ldb     FAC
        beq     MID_COMPLETE
        cmpb    4,u
        bhs     MID_COPY
        stb     4,u
MID_COPY
        bsr     MID_TARGET
        clra
        ldb     3,u
        addd    2,y
        tfr     d,y
        ldx     STR_PTR
        ldb     4,u
        ; The observed legacy zero-count path copies 256 bytes. It is bounded
        ; guest bus work, including ordinary ROM write rejection at $8000.
        ; Forward copying deliberately preserves self-overlap behaviour.
        bsr     COPY_BYTES
MID_COMPLETE
        ldx     6,u
        clra

MID_RETURN
        leas    8,s
        rts
MID_TARGET
        ldy     ,u
        tst     2,u
        beq     MID_TARGET_DONE
        tfr     y,d
        addd    ARYTAB
        tfr     d,y
MID_TARGET_DONE
        rts

