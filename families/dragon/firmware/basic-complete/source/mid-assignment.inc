; Original MID$ assignment. All temporaries live on the guest stack.
; Frame: tagged destination0, start2, count3. Cursor is saved for copying.
; Resolve an array descriptor again after expressions create scalar variables.
CMD_MID
        ldd     ,x++
        cmpd    #$ff98
        jsr     NATIVE_CHECK_NE
        jsr     EX_OPEN
        leas    -4,s
        tfr     s,u
        jsr     VAR_FIND_NATIVE
        jsr     EX_STRING_CHECK
        jsr     VARIABLE_REFERENCE
MID_DESTINATION
        std     ,u
        jsr     GFX_NEXT_BYTE
        tstb
        lbeq    STATEMENT_FAIL
        decb
        stb     2,u
        ldb     #255
        stb     3,u
        jsr     EX_OPTIONAL_COMMA
        bne     MID_CLOSE
        jsr     GFX_BYTE
        stb     3,u
MID_CLOSE
        jsr     MULTI_CLOSE
        jsr     EX_EQUALS
        bsr     MID_TARGET
        ldb     2,u
        cmpb    ,y
        jsr     NATIVE_CHECK_HS
        ldb     ,y
        subb    2,u
        cmpb    3,u
        bhs     MID_DETACH
        stb     3,u
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
        jsr     STRING_STORE_COPY
        puls    x,u
MID_EVALUATE
        jsr     EVAL_STRING
        jsr     CMD_REQUIRE_END
        pshs    x
        ldb     FAC
        beq     MID_COMPLETE
        cmpb    3,u
        bhs     MID_COPY
        stb     3,u
MID_COPY
        bsr     MID_TARGET
        clra
        ldb     2,u
        addd    2,y
        tfr     d,y
        ldx     STR_PTR
        ldb     3,u
        ; The observed legacy zero-count path copies 256 bytes. It is bounded
        ; guest bus work, including ordinary ROM write rejection at $8000.
        ; Forward copying deliberately preserves self-overlap behaviour.
        lbsr     COPY_BYTES
MID_COMPLETE
        puls    x
        clra

MID_RETURN
        leas    4,s
        rts
MID_TARGET equ VALUE_REFERENCE
