; Original interval collector. Select and close overlapping live intervals by
; bounded root scans, then move each union once. Descriptors remain read-only
; until the complete root set has passed validation. No temporary links or heap.
STRING_COLLECT_ACTIVE
        pshs    d,x,y,u
        lda     #1
        bra     GC_START
STRING_COLLECT
        pshs    d,x,y,u
        clra
GC_START
        ; The sole extra stack byte selects register roots. Ordinary collection
        ; must not retain a dead FAC left by the preceding BASIC statement.
        pshs    a
        ldd     MEMTOP
        std     GC_TOP
        addd    #1
        std     GC_HEAD
GC_SELECT
        clra
        clrb
        std     GC_LEN
        clr     GC_MODE
GC_EXPAND
        clr     GC_MODE+1
        jsr     STRING_VISIT_ROOTS
        bcs     GC_FAILED
        tst     GC_MODE+1
        bne     GC_EXPAND
        ldd     GC_LEN
        beq     GC_COMPLETE
        ldu     GC_TOP
        leau    1,u
        tfr     u,d
        subd    GC_LEN
        std     GC_DEST
        addd    GC_LAST
        subd    #1
        std     GC_TOP
        ldd     GC_LEN
        subd    GC_LAST
        tfr     d,y
        ldx     GC_LEN
GC_COPY_ROOT
        lda     ,-x
        sta     ,-u
        leay    -1,y
        bne     GC_COPY_ROOT
        inc     GC_MODE
        jsr     STRING_VISIT_ROOTS
        ldd     GC_LAST
        std     GC_HEAD
        bra     GC_SELECT
GC_COMPLETE
        ldd     GC_TOP
        std     FRETOP
GC_FAILED
GC_RETURN
        leas    1,s
        puls    d,x,y,u,pc
GC_VISIT
        pshs    d,x,y
        lda     ,y
        leay    2,y
        bra     GC_VISIT_LENGTH
; A is the length; Y addresses the pointer word, including FAC/ARG registers.
; Register roots use the same validation, interval closure and pointer update.
GC_VISIT_REGISTER
        pshs    d,x,y
GC_VISIT_LENGTH
        tsta
        beq     GC_VISIT_OK
        tfr     a,b
        clra
        addd    ,y
        bcs     GC_VISIT_BAD
        subd    #1
        cmpd    MEMTOP
        bhi     GC_VISIT_BAD
        addd    #1
        ldx     ,y
        cmpx    FRETOP
        bls     GC_VISIT_OK
        cmpx    GC_HEAD
        bhs     GC_VISIT_OK
        tst     GC_MODE
        bne     GC_UPDATE
        ; Keep the highest connected interval. A later root can extend it
        ; downwards; another bounded pass closes any earlier missed overlap.
        cmpx    GC_LEN
        bhs     GC_HIGHER
        cmpd    GC_LAST
        blo     GC_VISIT_OK
        cmpx    GC_LAST
        bhs     GC_EXTEND
        stx     GC_LAST
        cmpd    GC_LEN
        bls     GC_CHANGED
        bra     GC_NEW_END
GC_EXTEND
        cmpd    GC_LEN
        bls     GC_VISIT_OK
GC_NEW_END
        std     GC_LEN
        bra     GC_CHANGED
GC_HIGHER
        stx     GC_LAST
        std     GC_LEN
GC_CHANGED
        lda     #1
        sta     GC_MODE+1
        bra     GC_VISIT_OK
GC_UPDATE
        cmpd    GC_LAST
        blo     GC_VISIT_OK
        tfr     x,d
        addd    GC_DEST
        std     ,y
GC_VISIT_OK
        clra
        puls    d,x,y,pc
GC_VISIT_BAD
        coma
        puls    d,x,y,pc

