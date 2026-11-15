; Delete a validated inclusive range, compact once and relink the survivors.
CMD_DEL
        leas    -4,s
        tfr     s,u
        tst     EXEC_MODE
        bne     DELETE_ERROR
        jsr     EX_SPACE
        tsta
        beq     DELETE_ERROR
        cmpa    #':
        beq     DELETE_ERROR
        jsr     LINE_RANGE
        bcs     DELETE_ERROR
        jsr     VALIDATE_TEXT
        bcs     DELETE_ERROR
        tfr     s,u
        ldy     TXTTAB
DELETE_FIND
        ldd     ,y
        beq     DELETE_DONE
        ldd     2,y
        cmpd    ,u
        bhs     DELETE_FIRST
        ldy     ,y
        bra     DELETE_FIND
DELETE_FIRST
        pshs    y
DELETE_LAST
        ldd     ,y
        beq     DELETE_MOVE
        ldd     2,y
        cmpd    2,u
        bhi     DELETE_MOVE
        ldy     ,y
        bra     DELETE_LAST
DELETE_MOVE
        puls    u
        pshs    x
DELETE_COPY
        tfr     y,x
        tfr     u,y
        jsr     COPY_PROGRAM_TAIL
        sty     VARTAB
        ; Reuse the existing relinker; both paths invalidate variable storage.
        sty     REC_END
        jsr     STORE_REINDEX
        puls    x
        jsr     CMD_CLEAR_VARIABLES
DELETE_DONE
        clra
        bra     DELETE_RETURN
DELETE_ERROR
        coma
DELETE_RETURN
        leas    4,s
        rts

