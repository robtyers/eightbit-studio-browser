; Original descending-segment collector. One scan selects the highest connected
; segment encountered; no fixed-point overlap scan is required. Roots starting
; below that segment retain their original pointer until their remaining prefix
; moves. Every part of an overlapping root receives the same cumulative shift.
; The complete root set is validated before the first data or pointer mutation.
STRING_COLLECT
        pshs    d,x,y,u
GC_START
        ldd     MEMTOP
        std     GC_TOP
        addd    #1
        std     GC_HEAD
GC_SELECT
        clra
        clrb
        std     GC_LEN
        clr     GC_MODE
        jsr     STRING_VISIT_ROOTS
        bcs     GC_FAILED
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
        puls    d,x,y,u,pc
GC_VISIT
        pshs    d,x,y
        ; Keep Y at the descriptor base; the load already tests its length.
        lda     ,y
GC_VISIT_LENGTH
        beq     GC_VISIT_OK
        ; Static descriptors above MEMTOP do not belong to the heap. Leave
        ; them untouched, without reading payload or interpreting its extent.
        ldx     2,y
        cmpx    MEMTOP
        bhi     GC_VISIT_OK
        tfr     a,b
        clra
        addd    2,y
        bcs     GC_VISIT_BAD
        subd    #1
        ; A managed span must lie wholly below the configured heap ceiling.
        cmpd    MEMTOP
        bhi     GC_VISIT_BAD
        addd    #1
        cmpx    FRETOP
        bls     GC_VISIT_OK
        cmpx    GC_HEAD
        bhs     GC_VISIT_OK
        tst     GC_MODE
        bne     GC_UPDATE
        ; An earlier segment may already have moved the root's suffix.
        ; Clip to the old boundary, while retaining its original pointer.
        cmpd    GC_HEAD
        bls     GC_SELECT_SEGMENT
        ldd     GC_HEAD
GC_SELECT_SEGMENT
        cmpx    GC_LEN
        bhi     GC_SET_START
        cmpd    GC_LAST
        blo     GC_VISIT_OK
        cmpx    GC_LAST
        bhs     GC_EXTEND
GC_SET_START
        stx     GC_LAST
GC_EXTEND
        cmpd    GC_LEN
        bls     GC_VISIT_OK
        std     GC_LEN
        bra     GC_VISIT_OK
GC_UPDATE
        cmpx    GC_LAST
        blo     GC_VISIT_OK
        tfr     x,d
        addd    GC_DEST
        std     2,y
GC_VISIT_OK
        clra
        puls    d,x,y,pc
GC_VISIT_BAD
        coma
        puls    d,x,y,pc
