; Bounded enumeration validates all storage before the collector edits links.
STRING_VISIT_ROOTS
        ldd     VARTAB
        cmpd    #$0600
        blo     GC_ROOT_BAD
        cmpd    ARYTAB
        bhi     GC_ROOT_BAD
        ldd     ARYTAB
        cmpd    STREND
        bhi     GC_ROOT_BAD
        ldd     STREND
        cmpd    STACKTOP
        bhs     GC_ROOT_BAD
        ldd     FRETOP
        cmpd    STACKTOP
        blo     GC_ROOT_BAD
        cmpd    MEMTOP
        bhi     GC_ROOT_BAD
        ldy     VARTAB
GC_SCALAR
        cmpy    ARYTAB
        beq     GC_ARRAY
        bhi     GC_ROOT_BAD
        leax    7,y
        cmpx    ARYTAB
        bhi     GC_ROOT_BAD
        tst     1,y
        bpl     GC_SCALAR_NEXT
        leay    2,y
        jsr     GC_VISIT
        bcs     GC_ROOT_BAD
        leay    -2,y
GC_SCALAR_NEXT
        leay    7,y
        bra     GC_SCALAR
GC_ARRAY
        cmpy    STREND
        beq     GC_TEMPORARIES
        bhi     GC_ROOT_BAD
        jsr     ARRAY_RECORD
        bcs     GC_ROOT_BAD
        pshs    x
        tst     1,y
        bpl     GC_ARRAY_NEXT
        leay    d,y
GC_ARRAY_VALUE
        cmpy    ,s
        beq     GC_ARRAY_NEXT
        ; ARRAY_RECORD proved an exact, non-empty sequence of five-byte
        ; values. GC_VISIT preserves Y and changes only descriptor pointers.
        jsr     GC_VISIT
        bcs     GC_ARRAY_BAD
        leay    5,y
        bra     GC_ARRAY_VALUE
GC_ARRAY_NEXT
        puls    y
        bra     GC_ARRAY
GC_ARRAY_BAD
        leas    2,s
GC_ROOT_BAD
        coma
        rts
GC_TEMPORARIES
        ; STRING_COLLECT owns the mode byte immediately above our return.
        tst     2,s
        beq     GC_STACK_ROOTS
        tst     $06
        beq     GC_REGISTER_ARGUMENT
        lda     FAC
        ldy     #STR_PTR
        jsr     GC_VISIT_REGISTER
        bcs     GC_ROOT_BAD
GC_REGISTER_ARGUMENT
        tst     STR_BOTH
        beq     GC_STACK_ROOTS
        lda     ARG
        ldy     #ARG+3
        jsr     GC_VISIT_REGISTER
        bcs     GC_ROOT_BAD
GC_STACK_ROOTS
        ldu     STR_ROOTS
GC_TEMP_ROOT
        leay    ,u
        beq     GC_ROOT_OK
        tfr     s,d
        pshs    d
        cmpu    ,s++
        blo     GC_ROOT_BAD
        cmpu    MEMTOP
        bhi     GC_ROOT_BAD
        leay    2,u
        jsr     GC_VISIT
        bcs     GC_ROOT_BAD
        ldd     ,u
        beq     GC_ROOT_OK
        pshs    u
        cmpd    ,s++
        bls     GC_ROOT_BAD
        tfr     d,u
        bra     GC_TEMP_ROOT
GC_ROOT_OK
        rts

