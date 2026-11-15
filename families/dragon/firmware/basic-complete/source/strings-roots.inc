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
GC_RECORD
        ; Scalars and arrays both contain five-byte values after a validated
        ; header. One descriptor loop owns visitation and its failure cleanup.
        cmpy    ARYTAB
        bhs     GC_ARRAY
        leax    7,y
        cmpx    ARYTAB
        bhi     GC_ROOT_BAD
        ldd     #2
        bra     GC_RECORD_VALUES
GC_ARRAY
        cmpy    STREND
        beq     GC_TEMPORARIES
        jsr     ARRAY_RECORD
        bcs     GC_ROOT_BAD
GC_RECORD_VALUES
        pshs    x
        tst     1,y
        bpl     GC_RECORD_NEXT
        leay    d,y
GC_RECORD_VALUE
        ; Both record forms have an exact non-empty sequence of values.
        jsr     GC_VISIT
        bcs     GC_RECORD_BAD
        leay    5,y
        cmpy    ,s
        bne     GC_RECORD_VALUE
GC_RECORD_NEXT
        puls    y
        bra     GC_RECORD
GC_RECORD_BAD
        leas    2,s
GC_ROOT_BAD
        coma
        rts
GC_TEMPORARIES
        jsr     STRING_TEMP_CHECK
        bcs     GC_ROOT_BAD
        ldy     #TEMP_START
GC_PUBLIC_TEMPORARY
        cmpy    TEMP_FREE
        beq     GC_PUBLIC_DONE
        jsr     GC_VISIT
        bcs     GC_ROOT_BAD
        leay    5,y
        bra     GC_PUBLIC_TEMPORARY
GC_PUBLIC_DONE
GC_ROOT_OK
        rts
