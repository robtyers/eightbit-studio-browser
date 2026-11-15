; Raw DATA/INPUT spans have no public descriptor until their guarded store.
STRING_VALUE
        clr     STR_DESC
        clr     STR_DESC+1
STRING_KNOWN_VALUE
        sta     FAC
        sta     STR_LEN
        stu     STR_PTR
        lda     #$ff
        sta     $06
        clrb
        rts
STRING_NEW_VALUE equ STRING_TEMP_VALUE

STRING_LITERAL
        leax    1,x
        tfr     x,u
        clrb
STRING_LITERAL_SCAN
        jsr     TEXT_READ
        beq     STRING_LITERAL_END
        leax    1,x
        cmpa    #'"
        beq     STRING_LITERAL_END
        incb
        bne     STRING_LITERAL_SCAN
        ; Native tail continues into the following block.
STRING_TOO_LONG
        ldb     #28
        jmp     NATIVE_SET_ERROR
STRING_LITERAL_END
        tfr     b,a
        lbsr     STRING_TEMP_VALUE
        cmpu    TXTTAB
        blo     STRING_LITERAL_COPY
        cmpu    VARTAB
        blo     STRING_LITERAL_OK
STRING_LITERAL_COPY
        jsr     STRING_RELEASE
        pshs    x
        ldx     STR_PTR
        clra
        ldb     FAC
        jsr     VALUE_COPY
        puls    x,pc

STRING_LITERAL_OK
        clra
        rts
STRING_LOAD
        stx     STR_DESC
        lda     ,x
        ldu     2,x
        bsr     STRING_KNOWN_VALUE
        clra
        rts

; Try the same bounded allocation before and after one collection. X is a
; private retry count, preserved by the collector and restored on success.
; Failure raises OS through the enclosing native language owner.
STRING_ALLOC
        pshs    d,x
        ldx     #2
STRING_ALLOC_TRY
        ldd     FRETOP
        subd    ,s
        bcs     STRING_ALLOC_ERROR
        cmpd    STACKTOP
        bhs     STRING_ALLOC_READY
        leax    -1,x
        beq     STRING_ALLOC_ERROR
        jsr     STRING_COLLECT
        bcs     STRING_ALLOC_ERROR
        bra     STRING_ALLOC_TRY
STRING_ALLOC_READY
        std     FRETOP
        tfr     d,u
        leau    1,u
        puls    d,x,pc
STRING_ALLOC_ERROR
        ldb     #26
        jmp     NATIVE_SET_ERROR

STRING_COPY
        tstb
        beq     STRING_COPY_END
STRING_COPY_BYTE
        lda     ,x+
        sta     ,u+
        decb
        bne     STRING_COPY_BYTE
STRING_COPY_END
        rts
STRING_STORE_COPY
        pshs    x,y,u
        bra     STRING_STORE_ALLOC
STRING_STORE
        pshs    x,y,u
        tst     FAC
        beq     STRING_STORE_EMPTY
        bsr     STRING_OWNED
        beq     STRING_STORE_READY
        ldd     STR_PTR
        cmpd    TXTTAB
        blo     STRING_STORE_ALLOC
        cmpd    VARTAB
        blo     STRING_STORE_READY
STRING_STORE_ALLOC
        clra
        ldb     FAC
        bsr     STRING_ALLOC
        ldx     STR_DESC
        beq     STRING_STORE_RAW
        ldx     2,x
        bra     STRING_STORE_SOURCE
STRING_STORE_RAW
        ldx     STR_PTR
STRING_STORE_SOURCE
        stu     STR_PTR
        bsr     STRING_COPY
        bra     STRING_STORE_READY
STRING_STORE_EMPTY
        clra
        clrb
        std     STR_PTR
STRING_STORE_READY
        lda     FAC
        sta     ,y
        clr     1,y
        ldd     STR_PTR
        std     2,y
        clr     4,y
        jsr     STRING_RELEASE
        sty     STR_DESC
        clra
STRING_STORE_FAIL
        puls    x,y,u,pc
STRING_CONCAT
        lda     FAC
        adda    ARG
        bcs     STRING_TOO_LONG
        ; B is the first operand's release mark. Keep canonical operands
        ; live through allocation; public descriptors own collection updates.
        pshs    a,b
        clra
        ldb     ,s
        bsr     STRING_ALLOC
        pshs    u
        ldx     STR_DESC
        ldx     2,x
        ldb     FAC
        bsr     STRING_COPY
        ldx     ARG+3
        ldx     2,x
        ldb     ARG
        bsr     STRING_COPY
        puls    u
        ldb     1,s
        jsr     STRING_TEMP_REWIND
        puls    a,b
        ; Native tail continues into the following block.

; Canonical public temporaries occupy the existing eight five-byte slots.
; A length / U source produces one descriptor. All callers retain the same FAC
; result and X cursor; collection enumerates these actual public descriptors.
STRING_TEMP_VALUE
        pshs    d,x
        jsr     STRING_TEMP_CHECK
        bcs     STRING_TEMP_BAD
        cmpx    #TEMP_LIMIT
        bhs     STRING_TEMP_BAD
        puls    d
        sta     ,x
        clr     1,x
        stu     2,x
        clr     4,x
        stx     STR_DESC
        stx     TEMP_LAST
        leax    5,x
        stx     TEMP_FREE
        puls    x
        jmp     STRING_KNOWN_VALUE
LOCAL_FAILURE_4
        jmp     NATIVE_RANGE_ERROR
STRING_PRINT
        pshs    d,x
        ldx     STR_PTR
        ldb     FAC
        beq     STRING_PRINT_END
STRING_PRINT_BYTE
        lda     ,x+
        jsr     CHROUT
        decb
        bne     STRING_PRINT_BYTE
STRING_PRINT_END
        jsr     STRING_RELEASE
        puls    d,x,pc
STRING_LENGTH
        jsr     EX_STRING_CHECK
        jsr     STRING_RELEASE
        clra
        ldb     FAC
        jmp     INT_TO_FAC
STRING_ASC
        jsr     EX_STRING_CHECK
        jsr     STRING_RELEASE
        tst     FAC
        beq     LOCAL_FAILURE_4
        ldx     STR_PTR
        clra
        ldb     ,x
        jmp     INT_TO_FAC
STRING_CHR
        jsr     EX_TO_ADDRESS
        tsta
        bne     LOCAL_FAILURE_4
STRING_BYTE
        ; X fits in the existing save masks. The source byte remains at S.
        pshs    b,x
        ldd     #1
        leax    ,s
        jsr     VALUE_COPY
        puls    b,x,pc

STRING_VAL
        jsr     EX_STRING_CHECK
        ; Read in place with only a bounded text context on the stack. The
        ; enclosing evaluator owns U; no payload or NUL copy is required.
        leay    -4,s
        cmpy    STREND
        lbls    NATIVE_FAIL
        ldx     STR_PTR
        clra
        ldb     FAC
        leau    d,x
        ldd     STR_ROOTS
        pshs    d,u
        tfr     s,u
        stu     STR_ROOTS
        jsr     STRING_RELEASE
        jsr     EX_SPACE
        cmpa    #'&
        beq     STRING_VAL_RADIX
        jsr     ASCII_SIGN
STRING_VAL_SIGNED
        jsr     EX_SPACE
        cmpa    #'.
        beq     STRING_VAL_NUMBER
        cmpa    #'0
        blo     STRING_VAL_ZERO
        cmpa    #'9
        bhi     STRING_VAL_ZERO
STRING_VAL_NUMBER
        ; Decimal success/failure carry survives the context-only epilogue.
        jsr     NUMBER_SIGNED
        bra     STRING_VAL_RETURN

STRING_VAL_RADIX
        jsr     EX_BASE_NUMBER
        bra     STRING_VAL_RETURN
STRING_VAL_ZERO
        jsr     EX_FALSE
STRING_VAL_RETURN
        ; The parser may borrow U; this value owns its fixed endpoint frame.
        ldd     ,s
        std     STR_ROOTS
        leas    4,s
        rts
STRING_TEMP_BAD
        ldb     #30
        jmp     NATIVE_SET_ERROR
; Validate range, stride and paired pointer before interpreting public data.
STRING_TEMP_CHECK
        ldx     TEMP_FREE
        cmpx    #TEMP_START
        blo     STRING_TEMP_CHECK_BAD
        cmpx    #TEMP_LIMIT
        bhi     STRING_TEMP_CHECK_BAD
        tfr     x,d
        subd    #5
        cmpd    TEMP_LAST
        bne     STRING_TEMP_CHECK_BAD
        ; D still holds TEMP_FREE-5, including its validated low byte.
        subb    #$a4
STRING_TEMP_STRIDE
        beq     STRING_TEMP_VALID
        subb    #5
        bhs     STRING_TEMP_STRIDE
STRING_TEMP_CHECK_BAD
        coma
        rts
STRING_TEMP_VALID
        clra
        rts
; B is the low byte of a mark taken within the fixed public page.
STRING_TEMP_RESET
        ldb     #$a9
STRING_TEMP_REWIND
        lda     #1
        std     TEMP_FREE
        subd    #5
        std     TEMP_LAST
        rts
; Consuming the current owned value releases only its topmost descriptor.
; Borrowed scalar/array values never allocate or release a temporary slot.
STRING_RELEASE
        pshs    d
        bsr     STRING_OWNED
        bne     STRING_RELEASE_END
        bsr     STRING_TEMP_REWIND
STRING_RELEASE_END
        puls    d,pc
; Z means the current descriptor is the active top canonical temporary. D
; retains that descriptor, so B is already its release mark. Borrowed values
; and inactive/older canonical aliases do not pop the public temporary stack.
STRING_OWNED
        ldd     STR_DESC
        cmpd    #TEMP_START
        blo     STRING_BORROWED
        cmpd    TEMP_LAST
        andcc   #$fe
        rts
STRING_BORROWED
        andcc   #$fa
        rts
; Capture one typed operand in seven transient bytes. Numbers occupy 2..6.
; String references occupy 0..1; byte 2's carry bit marks an ARYTAB-relative
; element. Borrowed cartridge descriptors retain their full address. Byte 3
; is the release mark. The remaining string bytes are unused; length and
; character address are resolved from the descriptor after any collection.
; Public descriptors, including borrowed variables, remain authoritative.
VALUE_CAPTURE
        leay    2,u
        tst     $06
        bne     VALUE_CAPTURE_STRING
        jmp     VALUE_STORE
VALUE_CAPTURE_STRING
        bsr     STRING_OWNED
        beq     VALUE_CAPTURE_MARK
        ldb     TEMP_FREE+1
VALUE_CAPTURE_MARK
        stb     3,u
        ldd     STR_DESC
        bsr     DESCRIPTOR_REFERENCE
VALUE_CAPTURE_REFERENCE
        std     ,u
        tfr     cc,a
        sta     2,u
        lda     #$ff
        rts

; Capture a scalar or array lvalue without retaining an old table base.
; LET and MID$ lvalues are RAM records, so their two-byte representation can
; still use bit 15. General string operands can also name a ROM descriptor.
VARIABLE_REFERENCE
        ldd     VAR_PTR
        bsr     DESCRIPTOR_REFERENCE
        bcc     VARIABLE_REFERENCE_DONE
        ora     #$80
VARIABLE_REFERENCE_DONE
        rts
DESCRIPTOR_REFERENCE
        ; External machine-code functions may return a descriptor elsewhere
        ; in guest RAM or ROM. Only the array-table interval can relocate.
        cmpd    STREND
        bhs     VARIABLE_REFERENCE_DONE
        cmpd    ARYTAB
        blo     DESCRIPTOR_REFERENCE_RAW
        subd    ARYTAB
        orcc    #1
        rts
DESCRIPTOR_REFERENCE_RAW
        andcc   #$fe
        rts

; Resolve the saved public identity after any intervening scalar insertion.
VALUE_CAPTURED_REFERENCE
        ldb     2,u
        lsrb
VALUE_REFERENCE_CARRY
        ldd     ,u
        bcs     VALUE_RELATIVE_REFERENCE
        bra     VALUE_REFERENCE_DONE
VALUE_REFERENCE
        ldd     ,u
        bpl     VALUE_REFERENCE_DONE
        anda    #$7f
VALUE_RELATIVE_REFERENCE
        addd    ARYTAB
VALUE_REFERENCE_DONE
        tfr     d,y
        rts

VALUE_FRAME_LOAD
        leax    2,u
        tst     $06
        beq     VALUE_FRAME_READY
        bsr     VALUE_CAPTURED_REFERENCE
        tfr     y,x
VALUE_FRAME_READY
        ; Native tail continues into the following block.

; Original string values and heap allocation. Apache-2.0.
; Legacy records are [length,0,pointer-hi,pointer-lo,0]. Public $52 retains
; the authoritative descriptor and $56 its length; $4F and $50 cache length and
; character address while typed as a string. Machine-code entries clear $50.
; X addresses the common five-byte value; $06 selects its legacy interpretation.
VALUE_LOAD
        tst     $06
        bne     STRING_LOAD
        jmp     FAC_LOAD
