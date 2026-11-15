; Original binary cassette loader. Apache-2.0.
; Shared file search supplies checked 15-byte headers and data blocks. Offset
; arithmetic wraps at the published 16-bit boundary; the public block input
; checks each RAM extent before streaming bytes directly to their destination.
CMD_CLOAD
        ldb     #2
        bra     FILE_SUFFIX
NATIVE_SHARED12_TAPE_LOAD_INC_0
        jmp     FILE_BAD_BLOCK
CMD_CSAVE
        ldb     #4
FILE_SUFFIX
        jsr     EX_UPCASE
        cmpa    #'M
        bne     FILE_LOAD_BEGIN
        leax    1,x
        decb
FILE_LOAD_BEGIN
        jmp     FILE_BEGIN
FILE_LOAD_HEADER
        lda     $01e2
        ldb     8,u
        cmpb    #1
        bne     FILE_BASIC_HEADER
        cmpa    #2
        bne     NATIVE_SHARED12_TAPE_LOAD_INC_0
        tst     $01e3
        bne     NATIVE_SHARED12_TAPE_LOAD_INC_0
        ldd     $01e5
        addd    4,u
        std     $9d
        ldd     $01e7
        addd    4,u
        std     6,u
        neg     8,u
        rts
FILE_BASIC_HEADER
        tsta
        bne     NATIVE_SHARED12_TAPE_LOAD_INC_0
        ldb     $01e3
        beq     FILE_HEADER_FLAG
        incb
        bne     NATIVE_SHARED12_TAPE_LOAD_INC_0
        decb
FILE_HEADER_FLAG
        pshs    b
        jsr     NEW_PROGRAM
        ldd     TXTTAB
        std     6,u
        neg     8,u
        tst     ,s+
        beq     FILE_HEADER_RETURN
        dec     8,u
        ldd     #INPUTBUF
        std     6,u
FILE_HEADER_RETURN
        rts

