; Original guest cassette data-file lifecycle. Development candidate.
; Persistent state occupies only the published device/buffer workspace.
DATA_DEVICE
        jsr     EX_SPACE
        cmpa    #'#
        bne     DATA_DEVICE_VALUE
        leax    1,x
DATA_DEVICE_VALUE
        jsr     EVAL_NUMBER
DATA_DEVICE_FAC
        jsr     PUBLIC_FAC_TO_INT
        ; A signed-byte selector becomes visible even when its device is invalid.
        ; Word-range failures leave the previous public device untouched.
        pshs    a
        sex
        cmpa    ,s+
        bne     DATA_DEVICE_ERROR
        stb     DEVNUM
        cmpb    #-2
        blt     DATA_DEVICE_ERROR
        tstb
        bgt     DATA_DEVICE_ERROR
        rts
DATA_DEVICE_ERROR
        ldb     #40
        bra     DATA_ERROR_CODE
DATA_NOT_OPEN
        ldb     #46
        bra     DATA_ERROR_CODE
DATA_MODE_ERROR
        ldb     #44
        bra     DATA_ERROR_CODE
DATA_ALREADY_OPEN
        ldb     #38
        bra     DATA_ERROR_CODE
DATA_INPUT_PAST_END
        ldb     #48
        bra     DATA_ERROR_CODE
DATA_BAD_VALUE
        ldb     #36
        bra     DATA_ERROR_CODE
DATA_DIRECT_ERROR
        ldb     #22
        bra     DATA_ERROR_CODE
DATA_IO_ERROR
        jsr     MOTOROFF
        ldb     #42
DATA_ERROR_CODE
        jmp     NATIVE_SET_ERROR

CMD_OPEN
        jsr     EVAL_STRING
        tst     FAC
        lbeq     NATIVE_RANGE_ERROR
        ldy     STR_PTR
        lda     ,y
        ldb     #-6
        cmpa    #'O
        beq     DATA_OPEN_MODE
        incb
        cmpa    #'I
        beq     DATA_OPEN_MODE
        incb
DATA_OPEN_MODE
        jsr     STRING_RELEASE
        pshs    b
        jsr     MULTI_COMMA
        jsr     DATA_DEVICE
        jsr     MULTI_COMMA
        jsr     EVAL_STRING
        jsr     CMD_REQUIRE_END
        ; Filename callbacks own the live selection until OPEN consumes it.
        lda     DEVNUM
        clr     DEVNUM
        puls    b
        cmpb    #-4
        beq     DATA_MODE_ERROR
        cmpb    #-5
        bne     DATA_OPEN_DEVICE
        cmpa    #-2
        lbeq    DATA_MODE_ERROR
DATA_OPEN_DEVICE
        cmpa    #-1
        lbne     DATA_CLOSE_RETURN
        jmp     FILE_BEGIN

DATA_OPEN_WRITE
        lda     #1
        sta     $01e2
        ldd     #$ffff
        std     $01e3
        jmp     FILE_SAVE_PARSED
DATA_OPEN_WRITTEN
        lda     #2
        sta     $78
        jsr     DATA_BUFFER_RESET
        jmp     FILE_STOP_MOTOR
DATA_OPEN_READ
        jsr     DATA_BUFFER_RESET
        ; OPEN reads ASCII records from any file type when both flags are set.
        ; The public bytes are non-zero predicates, not fixed FF literals.
        ldd     $01e3
        tsta
        beq     DATA_HEADER_MODE_ERROR
        tstb
        beq     DATA_HEADER_MODE_ERROR
        lda     #1
        sta     $78
        jsr     DATA_REFILL
        jmp     FILE_LOAD_STOP_MOTOR

DATA_HEADER_MODE_ERROR
        jsr     MOTOROFF
        jmp     DATA_MODE_ERROR
DATA_BUFFER_RESET
        clr     $79
        ldd     #$01da
        std     $7a
        std     $7e
        rts
DATA_CHECK_INPUT
        lda     #1
        bra     DATA_CHECK_MODE
DATA_CHECK_OUTPUT
        lda     #2
DATA_CHECK_MODE
        tst     $78
        lbeq     DATA_NOT_OPEN
        cmpa    $78
        lbne     DATA_MODE_ERROR
        rts
CMD_CLOSE
        jsr     EX_END
        beq     DATA_CLOSE_SELECTED
DATA_CLOSE_ITEM
        jsr     DATA_DEVICE
        cmpb    #-1
        bne     DATA_CLOSE_NEXT
DATA_CLOSE_SELECTED
        bsr     DATA_CLOSE
DATA_CLOSE_NEXT
        clr     DEVNUM
        jsr     EX_OPTIONAL_COMMA
        beq     DATA_CLOSE_ITEM
        jmp     CMD_REQUIRE_END
DATA_CLOSE
        lda     $78
        beq     DATA_CLOSE_RETURN
        cmpa    #2
        bne     DATA_CLOSE_RESET
        jsr     FILE_ASCII_CLOSE
DATA_CLOSE_RESET
        clr     $78
        jsr     MOTOROFF
DATA_CLOSE_RETURN
        clra
        rts
; Preserve a live token cursor and all parser registers across physical input.
DATA_REFILL
        pshs    d,x
        tst     $79
        bne     DATA_REFILL_DONE
        tst     $7c
        bmi     DATA_REFILL_DONE
        ldx     #$ffff
DATA_REFILL_BLOCK
        leax    -1,x
        beq     DATA_REFILL_ERROR
        ldd     #$01da
        std     $7e
        jsr     MOTORON
        jsr     TAPE_BLOCK_IN
        jsr     MOTOROFF
        bcs     DATA_REFILL_ERROR
        lda     $7c
        cmpa    #$ff
        beq     DATA_REFILL_EOF
        cmpa    #1
        bne     DATA_REFILL_ERROR
        ldb     $7d
        beq     DATA_REFILL_BLOCK
        stb     $79
        ldd     #$01da
        std     $7a
        bra     DATA_REFILL_DONE
DATA_REFILL_EOF
        tst     $7d
        bne     DATA_REFILL_ERROR
DATA_REFILL_DONE
        andcc   #$fe
        puls    d,x,pc
DATA_REFILL_ERROR
        jmp     DATA_IO_ERROR
DATA_BYTE
        bsr     DATA_TRY_BYTE
        lbcs    DATA_INPUT_PAST_END
        rts
; EOF is a normal boundary for the programme loader and an error for ordinary
; data input. Both routes call the public hook before testing/refilling input.
DATA_TRY_BYTE
        jsr     INPUT_HOOK
        tst     DEVNUM
        lbeq    READ_CHARACTER_WAIT
        jsr     DATA_REFILL
        clr     $70
        tst     $79
        bne     DATA_BYTE_READY
        com     $70
        rts
DATA_BYTE_READY
        pshs    x
        ldx     $7a
        lda     ,x+
        stx     $7a
        dec     $79
        puls    x
        ; Consuming the final byte also reads the following block. This makes
        ; its EOF/IO result visible before the caller accepts the character.
        jmp     DATA_REFILL
DATA_EOF
        ldb     DEVNUM
        pshs    b
        jsr     DATA_DEVICE_FAC
        cmpb    #0
        beq     DATA_EOF_VALUE
        cmpb    #-1
        beq     DATA_EOF_CASSETTE
        jmp     DATA_MODE_ERROR
DATA_EOF_CASSETTE
        jsr     DATA_CHECK_INPUT
        jsr     DATA_REFILL
        clrb
        tst     $79
        bne     DATA_EOF_VALUE
        decb
DATA_EOF_VALUE
        puls    a
        sta     DEVNUM
        sex
        jmp     INT_TO_FAC

; Cassette PRINT uses the common grammar, including USING and TAB. Its
; value terminators consult the live device after each output callback.
DATA_PRINT
        jsr     DATA_CHECK_OUTPUT
        jsr     EX_SPACE
        jmp     PRINT_FIRST
DATA_INPUT_ENTRY
        jsr     INPUT_PROGRAMME
        jsr     EX_SPACE
        cmpa    #'#
        beq     DATA_INPUT_DEVICE
        clra
        jmp     INPUT_BEGIN
DATA_LINE_INPUT_ENTRY
        jsr     INPUT_PROGRAMME
        jsr     EX_NEXT_SPACE
        cmpa    #'#
        beq     DATA_LINE_DEVICE
        lda     #1
        jmp     INPUT_BEGIN
DATA_LINE_DEVICE
        lda     #3
        bra     DATA_INPUT_SETUP
DATA_INPUT_DEVICE
        lda     #2
DATA_INPUT_SETUP
        pshs    a
        jsr     DATA_DEVICE
        cmpb    #-2
        lbeq    DATA_MODE_ERROR
        tstb
        beq     DATA_INPUT_SELECTED
        jsr     DATA_CHECK_INPUT
DATA_INPUT_SELECTED
        jsr     MULTI_COMMA
        puls    a
        jmp     INPUT_BEGIN

; One bounded record collector serves data INPUT and ASCII CLOAD. It leaves
; the keyboard length byte untouched and returns the record length in A.
; Carry distinguishes an unfinished EOF tail from a complete CR-terminated line.
DATA_INPUT_RECORD
        bsr     DATA_READ_RECORD
        lbcs    DATA_INPUT_PAST_END
        rts
DATA_READ_RECORD
        pshs    y
        ldy     #INPUTBUF
DATA_INPUT_CHARACTER
        jsr     DATA_TRY_BYTE
        bcs     DATA_INPUT_RETURN
        cmpa    #13
        beq     DATA_INPUT_VALUE
        cmpa    #32
        blo     DATA_INPUT_CHARACTER
        cmpa    #127
        bhs     DATA_INPUT_CHARACTER
        cmpy    #INPUTBUF+249
        bhs     DATA_INPUT_CHARACTER
        sta     ,y+
        bra     DATA_INPUT_CHARACTER
DATA_INPUT_VALUE
        clr     ,y
        tfr     y,d
        subd    #INPUTBUF
        tfr     b,a
        ldx     #INPUTBUF
DATA_INPUT_RETURN
        puls    y,pc

; CLOAD/CLOADM abandon an open data stream. Saves and SKIPF reject it
; before touching its buffered record. A successful load header resets cursors.
DATA_FILE_AVAILABLE
        cmpb    #1
        beq     DATA_FILE_ABANDON
        cmpb    #2
        beq     DATA_FILE_ABANDON
        tst     $78
        lbne     DATA_ALREADY_OPEN
        rts
DATA_FILE_ABANDON
        clr     $78
        rts
; END closes the data stream; STOP and natural programme completion retain it.
DATA_END
        jsr     CMD_REQUIRE_END
        jsr     DATA_CLOSE
        jmp     CMD_END_VALIDATED

; Original named cassette-file search and skip. Apache-2.0.
; The name/cursor/block budget live on the native stack. Search/skip blocks use
; $01DA-$02D8; selected binary blocks stream into their public RAM destination.
; U is eight bytes into the 17-byte native frame: name -8, cursor 0,
; load offset/exclusive save end 4, read budget/save start 6, operation 8.
; A selected reader keeps only cursor4/budget6/operation8; saves drop the name.
; The sole block destination remains the published $7E cursor; frame exit is
; relative to U.
CMD_SKIPF
        clrb
FILE_BEGIN
        jsr     DATA_FILE_AVAILABLE
        leas    -17,s
        leau    8,s
        stb     8,u
        leay    -8,u
        ldb     #8
        lda     #32
FILE_EMPTY_NAME
        sta     ,y+
        decb
        bne     FILE_EMPTY_NAME
        clr     -8,u
        tst     8,u
        bmi     FILE_NAME_VALUE
        jsr     EX_END
        beq     FILE_NAME_READY
        jsr     EX_COMMA
        beq     FILE_NAME_READY
        jsr     EVAL_NATIVE
FILE_NAME_VALUE
        jsr     EX_STRING_CHECK
        ldb     FAC
        cmpb    #8
        bls     FILE_NAME_LENGTH
        tst     8,u
        lbpl     NATIVE_FAIL
        ldb     #8
FILE_NAME_LENGTH
        pshs    x
        leay    -8,u
        ldx     STR_PTR
        tstb
        beq     FILE_NAME_COPIED
FILE_NAME_COPY
        jsr     COPY_BYTES
FILE_NAME_COPIED
        puls    x
        jsr     STRING_RELEASE
FILE_NAME_READY
        clra
        clrb
        std     4,u
FILE_ARGUMENTS_READY
        lda     8,u
        cmpa    #-5
        beq     FILE_READ_ARGUMENTS
        cmpa    #3
        lbhs     FILE_SAVE
FILE_READ_ARGUMENTS
        cmpa    #1
        beq     FILE_READ_UNVALIDATED
        jsr     CMD_REQUIRE_END
FILE_READ_UNVALIDATED
        stx     0,u
        ldd     #$ffff
        std     6,u
        jsr     MOTORON
FILE_SEARCH
        jsr     FILE_SEARCH_DISPLAY
        ldd     #$01da
        std     $7e
        jsr     FILE_READ_BLOCK
FILE_SEARCH_RESULT
        tst     $7c
        bne     FILE_SEARCH
        ldb     $7d
        cmpb    #15
        lbne     FILE_BAD_BLOCK
        jsr     FILE_HEADER_DISPLAY
        tst     -8,u
        beq     FILE_FOUND
        ldx     #$01da
        leay    -8,u
        ldb     #8
FILE_NAME_COMPARE
        jsr     COMPARE_BYTES
        bne     FILE_SEARCH
FILE_FOUND
        jsr     FILE_FOUND_MARKER
        jsr     DATA_BUFFER_RESET
        lda     8,u
        cmpa    #-5
        beq     FILE_READ_SELECTED
        tsta
        bne     FILE_LOAD_SELECTED
        lda     #-4
        sta     8,u
        bra     FILE_READ_SELECTED
FILE_LOAD_SELECTED
        jsr     FILE_LOAD_HEADER
FILE_READ_SELECTED
        ; Header parsing no longer needs the filename or offset. Retain only
        ; cursor4, budget6 and operation8 above the live stack boundary.
        ldd     0,u
        std     4,u
        leas    4,u
        lda     8,u
        adda    #3
        cmpa    #1
        bhi     FILE_READ_FRAME_READY
        ; NEW has invalidated the control records. Reclaim them before any
        ; programme bytes are written, only when this is the executor's own
        ; complete statement boundary. Nested public callers retain their S.
        leax    9,s
        cmpx    STATEMENT_SP
        bne     FILE_READ_FRAME_READY
        leax    14,x
        cmpx    CTL_SP
        bne     FILE_READ_FRAME_READY
        cmpx    CTL_BASE
        bhs     FILE_READ_FRAME_READY
        ldy     CTL_BASE
        ldb     #23
FILE_READ_FRAME_COPY
        lda     ,-x
        sta     ,-y
        decb
        bne     FILE_READ_FRAME_COPY
        leas    ,y
        leau    -4,s
        leay    9,s
        sty     STATEMENT_SP
        ldd     CTL_BASE
        std     CTL_SP
FILE_READ_FRAME_READY
        lda     8,u
        cmpa    #-5
        lbeq     DATA_OPEN_READ
        cmpa    #-3
        lbeq    FILE_ASCII_LOAD
FILE_SKIP
        lda     8,u
        cmpa    #-2
        bne     FILE_LOAD_BLOCK
        ldd     $7e
        ; The legacy loader admits a complete maximum block before reading
        ; its type/length, including EOF. Its public margin is 318 bytes.
        addd    #318
        bcs     FILE_MEMORY_ERROR
        cmpd    STACKTOP
        bhi     FILE_MEMORY_ERROR
        ; Nested public callers may have a lower live stack than STACKTOP.
        ; 37 bytes cover the reader and nested hardware interrupt frames.
        subd    #26
        leax    ,s
        pshs    x
        cmpd    ,s++
        bhi     FILE_MEMORY_ERROR
FILE_LOAD_BLOCK
        bsr     FILE_READ_BLOCK
        lda     $7c
        cmpa    #1
        beq     FILE_DATA
        cmpa    #$ff
        bne     FILE_BAD_BLOCK
        tst     $7d
        bne     FILE_BAD_BLOCK
FILE_STOP
        lda     8,u
        cmpa    #-2
        bne     FILE_LOAD_STOP_MOTOR
        jsr     FILE_BASIC_FINISH
FILE_LOAD_STOP_MOTOR
        ldx     4,u
        bra     FILE_STOP_READY
FILE_STOP_MOTOR
        ldx     0,u
FILE_STOP_READY
        jsr     MOTOROFF
        leas    9,u
        clr     DEVNUM
        clra
        rts
FILE_DATA
        lda     8,u
        cmpa    #-4
        beq     FILE_SKIP
        clra
        ldb     $7d
        addd    $7e
        std     $7e
        bra     FILE_SKIP
; The native caller owns its search/selected continuation. No operation
; discriminator is needed after the public reader returns. Consume the native
; return word into Y before physical input, retaining the existing live-stack
; admission and peak. The public block reader preserves Y on every exit.
FILE_READ_DESTINATION
FILE_READ_BLOCK
        puls    y
        ldx     6,u
        leax    -1,x
        beq     FILE_LIMIT
        stx     6,u
        jsr     TAPE_BLOCK_IN
        bcs     FILE_IO_ERROR
        jmp     ,y
FILE_MEMORY_ERROR
        jsr     MOTOROFF
        jmp     NATIVE_MEMORY_ERROR
FILE_BAD_BLOCK
        lda     #2
        bra     FILE_STATUS_ERROR
FILE_LIMIT
        lda     #3
FILE_STATUS_ERROR
        sta     $81
FILE_IO_ERROR
        lda     8,u
        adda    #3
        cmpa    #1
        bhi     FILE_IO_STOP
        jsr     NEW_PROGRAM
FILE_IO_STOP
        jmp     DATA_IO_ERROR


; Original tokenised BASIC cassette output. Apache-2.0.
; FILE_BEGIN owns the name/cursor frame; reads use only the guest bus.
FILE_SAVE
        stx     0,u
        tst     -8,u
        bne     FILE_SAVE_NAME
        lda     #32
        sta     -8,u
FILE_SAVE_NAME
        leax    -8,u
        ldy     #$01da
        ldb     #8
        jsr     COPY_BYTES
        leas    ,u
        ; BASIC/data headers retain the unused execution/load words.
        ; Machine-code saves replace both words after parsing their addresses.
        ldb     #3
FILE_SAVE_HEADER
        clr     ,y+
        decb
        bne     FILE_SAVE_HEADER
        ldx     0,u
        lda     8,u
        cmpa    #-6
        lbeq     DATA_OPEN_WRITE
        cmpa    #3
        beq     FILE_SAVE_MACHINE
        jsr     VALIDATE_PROGRAM
        bcs     NATIVE_SHARED12_TAPE_SAVE_INC_0
        ldd     TXTTAB
        std     6,u
        ; Retain one exclusive end, not a second moving count. The write
        ; cursor already lives in the public block destination.
        ldd     VARTAB
        std     4,u
        ldx     0,u
        jsr     EX_OPTIONAL_COMMA
        bne     FILE_SAVE_PARSED
        jsr     EX_ALPHA
        cmpa    #'A
        bne     NATIVE_SHARED12_TAPE_SAVE_INC_0
        leax    1,x
        ldd     #$ffff
        std     $01e3
        bra     FILE_SAVE_PARSED
NATIVE_SHARED12_TAPE_SAVE_INC_0
        jmp     NATIVE_FAIL
FILE_SAVE_MACHINE
        jsr     EVAL_NEXT_ADDRESS
        std     6,u
        std     $01e7
        jsr     EVAL_NEXT_ADDRESS
        cmpd    6,u
        blo     NATIVE_SHARED12_TAPE_SAVE_INC_0
        addd    #1
        std     4,u
        jsr     EVAL_NEXT_ADDRESS
        std     $01e5
        lda     #2
        sta     $01e2
FILE_SAVE_PARSED
        jsr     CMD_REQUIRE_END
        stx     0,u
        ldd     #128
        std     $90
        ldd     #$01da
        std     $7e
        ldd     #15
        std     $7c
        bsr     FILE_WRITE_BLOCK
        lda     8,u
        cmpa    #-6
        lbeq     DATA_OPEN_WRITTEN
        tst     $01e3
        bne     FILE_SAVE_ASCII
        ldd     6,u
        std     $7e
FILE_SAVE_DATA
        ldd     4,u
        subd    $7e
        beq     FILE_SAVE_FULL
        tsta
        beq     FILE_SAVE_LENGTH
FILE_SAVE_FULL
        ldb     #255
FILE_SAVE_LENGTH
        lda     #1
        std     $7c
        bsr     FILE_WRITE_BLOCK
        clra
        ldb     $7d
        addd    $7e
        std     $7e
        cmpd    4,u
        bne     FILE_SAVE_DATA
FILE_SAVE_EOF
        bsr     FILE_WRITE_EOF
        jmp     FILE_STOP_MOTOR
FILE_WRITE_EOF
        ldd     #$ff00
        std     $7c
FILE_WRITE_BLOCK
        jsr     TAPE_PREP
        jsr     TAPE_BLOCK_OUT
        ; Only reads can abandon a partially loaded BASIC programme. Writes
        ; share this frame-independent failure path, including data-file CLOSE.
        lbcs     DATA_IO_ERROR
        rts

; ASCII saves open the public output file and LIST through the same live
; device route as PRINT. A hook can redirect listing; only a retained cassette
; selection closes that file. Successful ASCII saves end the current execution.
FILE_SAVE_ASCII
        jsr     FILE_ASCII_RUN
        bcc     FILE_ASCII_FINISH
        ; Only this save opened a temporary file. Releasing its mode permits
        ; a fresh save after writer failure; PRINT/CLOSE keep their file state.
        clr     $78
        jmp     NATIVE_FAIL
FILE_ASCII_RUN
        pshs    d,y,u
        jsr     NATIVE_INLINE
        jsr     DATA_BUFFER_RESET
        lda     #2
        sta     $78
        lda     #$ff
        sta     DEVNUM
        lda     #13
        jsr     CHROUT
        ldx     #EMPTY_TEXT
        jsr     CMD_LIST
        ldb     DEVNUM
        incb
        bne     FILE_ASCII_RETURN_MODE
        jsr     DATA_CLOSE
FILE_ASCII_RETURN_MODE
        clra
        rts
FILE_ASCII_FINISH
        lda     #$ff
        sta     EXEC_MODE
        jmp     FILE_STOP_MOTOR
; Both ASCII programme saves and data-file CLOSE flush their final buffered
; record, leave the established inter-block gap, then emit an empty EOF block.
FILE_ASCII_CLOSE
        ldb     $79
        beq     FILE_ASCII_EOF
        bsr     FILE_ASCII_BLOCK
FILE_ASCII_EOF
        bsr     FILE_ASCII_PAUSE
        bra     FILE_WRITE_EOF
FILE_ASCII_BYTE
        pshs    d,x
        ; Keep the full pending block intact until its writer succeeds. The
        ; next character is accepted only after that block has been drained.
        ldb     $79
        cmpb    #255
        bne     FILE_ASCII_ACCEPT
        bsr     FILE_ASCII_BLOCK
        lda     ,s
FILE_ASCII_ACCEPT
        ldx     $7a
        sta     ,x+
        stx     $7a
        inc     $79
FILE_ASCII_RETURN
        puls    d,x,pc
; ASCII files are gapped. The original finite delay precedes the leader so a
; reader can finish its line processing before the next physical data block.
FILE_ASCII_BLOCK
        lda     #1
        std     $7c
        ldd     #$01da
        std     $7e
        bsr     FILE_ASCII_PAUSE
        jsr     TAPE_PREP
        jsr     TAPE_BLOCK_OUT
        bcs     FILE_ASCII_FAILED
        jsr     MOTOROFF
        jsr     DATA_BUFFER_RESET
        rts
FILE_ASCII_FAILED
NATIVE_TAIL_4
        jsr     MOTOROFF
        jmp     NATIVE_FAIL
FILE_ASCII_PAUSE
        pshs    y
        ldy     #55930
FILE_ASCII_GAP
        leay    -1,y
        bne     FILE_ASCII_GAP
        puls    y,pc

; Explicit PRINT selection validates the public file lifecycle once before
; entering the common value and format controller.
PRINT_DEVICE
        leax    1,x
        lbsr     EVAL_NUMBER
        jsr     DATA_DEVICE_FAC
        cmpb    #-1
        beq     DATA_PRINT_DEVICE
PRINT_DEVICE_READY
        lbsr     MULTI_COMMA
        ldu     #PRINT_FIRST
        jmp     PRINT_SELECTED

DATA_PRINT_DEVICE
        jsr     MULTI_COMMA
        ldu     #DATA_PRINT
        jmp     PRINT_SELECTED

; Programme-only INPUT keeps its bounded reply in the public input area.
; The stack frame retains programme cursors; $35 publishes the CSV reply cursor.
CMD_INPUT equ DATA_INPUT_ENTRY
CMD_LINE_INPUT equ DATA_LINE_INPUT_ENTRY

INPUT_PROGRAMME
        ldy     CURLINE
        cmpy    #$ffff
        lbeq    DATA_DIRECT_ERROR
        rts
INPUT_BEGIN
        ; Retain the established preflight margin even where a specialised
        ; continuation needs less transient state than the old nine-byte frame.
        leau    -137,s
        cmpu    STREND
        lbls    NATIVE_FAIL
        lsra
        tst     DEVNUM
        bne     INPUT_FILE_BEGIN
        pshs    cc
        jsr     EX_SPACE
        cmpa    #'"
        bne     INPUT_PROMPT_READY
        pshs    u
        jsr     STRING_LITERAL
        puls    u
        jsr     ST_CHECK
        jsr     STRING_PRINT
        lda     #';
        jsr     EX_EXPECT
INPUT_PROMPT_READY
        puls    cc
        bcs     INPUT_LINE_KEYBOARD

; Keyboard CSV keeps its variable-list continuations in the native frame.
; The public $35 delimiter is the authoritative reply cursor, including after
; a subscript callback. Refill retains an already resolved destination.
INPUT_CSV_BEGIN
        leas    -4,s
        leau    ,s
        stx     ,u
INPUT_CSV_REPLY
        stx     2,u
        bsr     INPUT_CSV_LINE
        bcs     INPUT_CSV_RETURN
        ldx     2,u
INPUT_CSV_VARIABLE
        jsr     VAR_FIND_NATIVE
        stx     2,u
INPUT_CSV_FIELD
        ldx     $35
        tst     ,x+
        bne     INPUT_CSV_PARSE
        lda     #'?
        jsr     CHROUT
        bsr     INPUT_CSV_LINE
        bcs     INPUT_CSV_RETURN
        bra     INPUT_CSV_FIELD
INPUT_CSV_PARSE
        jsr     DATA_KEYBOARD_VALUE
        bcs     INPUT_RETRY
        stx     $35
        jsr     VALUE_STORE
        ldx     2,u
        jsr     EX_OPTIONAL_COMMA
        beq     INPUT_CSV_VARIABLE
        ldy     $35
        tst     ,y
        beq     INPUT_CSV_RETURN
        pshs    x
        ldx     #INPUT_EXTRA
        jsr     PUTS
        puls    x
INPUT_CSV_RETURN
        leas    4,s
        bra     INPUT_RETURN
INPUT_RETRY
        ldx     #INPUT_REDO
        jsr     PUTS
        ldx     ,u
        bra     INPUT_CSV_REPLY
INPUT_CSV_LINE
        pshs    y
        ldy     #INPUTBUF-1
        sty     $35
        lda     #'?
        jsr     CHROUT
        lda     #32
        jsr     CHROUT
        bsr     INPUT_READ_REPLY
        bcs     INPUT_CSV_LINE_RETURN
        lda     #',
        sta     INPUTBUF-1
INPUT_CSV_LINE_RETURN
        puls    y,pc
INPUT_RETURN
        clr     DEVNUM
        clra
        rts

; Break saves the INPUT token for CONT before the caller reclaims its own
; continuation. No destination is created until a keyboard reply has arrived.
INPUT_READ_REPLY
        ldx     #INPUTBUF
        jsr     READLINE_INTO
        bcc     INPUT_READ_RETURN
        lda     LINE_INVALID
        cmpa    #1
        lbne    NATIVE_FAIL
        ldx     TXT_PTR
        jsr     BREAK_MESSAGE
        jsr     CMD_END_VALIDATED
        coma
INPUT_READ_RETURN
        rts

INPUT_FILE_BEGIN
        lbcc    INPUT_FILE_CSV
        bsr     INPUT_LINE_VARIABLE
        pshs    x
        jsr     DATA_INPUT_RECORD
        bsr     INPUT_LINE_VALUE
        puls    x,pc
INPUT_LINE_KEYBOARD
        pshs    x
        bsr     INPUT_READ_REPLY
        bcs     INPUT_LINE_BREAK
        puls    x
        pshs    b
        bsr     INPUT_LINE_VARIABLE
INPUT_LINE_STORE
        puls    a
INPUT_LINE_VALUE
        pshs    u
        ldu     #INPUTBUF
        jsr     STRING_VALUE
        puls    u
        jsr     VALUE_STORE
        bra     INPUT_RETURN
INPUT_LINE_BREAK
        leas    2,s
        bra     INPUT_RETURN
INPUT_LINE_VARIABLE
        jsr     VAR_FIND_NATIVE
        tst     $06
        lbeq    NATIVE_FAIL
        jmp     CMD_REQUIRE_END

; File fields are distinct records. Resolve their destination before consuming
; any record, then retain the next token cursor only for that one field.
INPUT_FILE_CSV
        ldy     #INPUTBUF-1
        sty     $35
        jsr     VAR_FIND_NATIVE
        pshs    x
        jsr     DATA_INPUT_RECORD
        jsr     DATA_FILE_VALUE
        stx     $35
        jsr     VALUE_STORE
        jsr     EX_END
        beq     INPUT_FILE_STORED
        cmpa    #',
        lbne    DATA_BAD_VALUE
INPUT_FILE_STORED
        puls    x
        jsr     EX_OPTIONAL_COMMA
        beq     INPUT_FILE_CSV
        bra     INPUT_RETURN
INPUT_REDO
        fcc     "?REDO"
        fcb     13,0
INPUT_EXTRA
        fcc     "?EXTRA IGNORED"
        fcb     13,0

BASIC_ERROR
        jsr     LEGACY_ERROR_HOOK
        clr     DEVNUM
        lda     CURSOR+1
        anda    #31
        beq     BASIC_ERROR_START
        jsr     ERROR_NEWLINE
BASIC_ERROR_START
        lda     #'?
        jsr     CHROUT
        ldb     ERROR_CODE
        cmpb    #50
        bhi     ERROR_DEFAULT
        bitb    #1
        beq     ERROR_SELECTED
ERROR_DEFAULT
        ldb     #2
ERROR_SELECTED
        ldx     #ERROR_CODES
        abx
        lda     ,x+
        jsr     CHROUT
        lda     ,x
        jsr     CHROUT
        ldx     #ERROR_SUFFIX
        jsr     PUTS
ERROR_LOCATION
        ldd     CURLINE
        cmpd    #$ffff
        beq     ERROR_NEWLINE
        pshs    d
        ldx     #ERROR_IN
        jsr     PUTS
        puls    d
        jsr     PRINT_UNSIGNED
ERROR_NEWLINE
        lda     #13
        jmp     CHROUT

; Original diagnostic data, packed in a separate fixed-entry gap. Apache-2.0.
ERROR_CODES
        fcc     "NFSNRGODFCOVOMULBSDD/0IDTMOSLSSTCNUFFDAODNIOFMNOIEDS"
ERROR_SUFFIX
        fcc     " ERROR"
        fcb     0
ERROR_IN
        fcc     " IN "
        fcb     0

; Original binary cassette loader. Apache-2.0.
; Shared file search supplies checked 15-byte headers and data blocks. Offset
; arithmetic wraps at the published 16-bit boundary; the public block input
; checks each RAM extent before streaming bytes directly to their destination.
CMD_CLOAD
        ldb     #2
        bra     FILE_SUFFIX
NATIVE_SHARED12_TAPE_LOAD_INC_0
        jmp     DATA_HEADER_MODE_ERROR
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
        lda     8,u
        cmpa    #1
        bne     FILE_BASIC_HEADER_READ
        ; CLOADM defers its optional offset and suffix until a header matches.
        ; The selected name is visible and the motor is off during evaluation.
        jsr     MOTOROFF
        ldx     0,u
        jsr     EX_OPTIONAL_COMMA
        bne     FILE_MACHINE_NO_OFFSET
        jsr     EVAL_ADDRESS
FILE_OFFSET_POSITIVE
FILE_OFFSET_READY
        std     4,u
        bra     FILE_MACHINE_END
FILE_MACHINE_NO_OFFSET
        jsr     CMD_REQUIRE_END
FILE_MACHINE_END
        stx     0,u
        lda     $01e2
        cmpa    #2
        bne     NATIVE_SHARED12_TAPE_LOAD_INC_0
        jsr     MOTORON
        ldd     $01e5
        addd    4,u
        std     $9d
        ldd     $01e7
        addd    4,u
        std     $7e
        neg     8,u
        rts
FILE_BASIC_HEADER_READ
        lda     $01e2
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
        std     $7e
        neg     8,u
        tst     ,s+
        beq     FILE_HEADER_RETURN
        dec     8,u
FILE_HEADER_RETURN
        rts

; Search status belongs only to direct mode. The public block reader toggles
; the marker while reading, independently of the ordinary character sink.
FILE_SEARCH_DISPLAY
        ldd     CURLINE
        cmpd    #$ffff
        bne     FILE_DISPLAY_RETURN
        jsr     CLS
        lda     #'S
        jmp     SCREENOUT
FILE_HEADER_DISPLAY
        ldd     CURLINE
        cmpd    #$ffff
        bne     FILE_DISPLAY_RETURN
        ldd     #$0402
        std     CURSOR
        ldx     #$01da
        ldb     #8
FILE_FOUND_NAME
        lda     ,x+
        jsr     SCREENOUT_HOOKED
        decb
        bne     FILE_FOUND_NAME
FILE_DISPLAY_RETURN
        rts
FILE_FOUND_MARKER
        ldd     CURLINE
        cmpd    #$ffff
        bne     FILE_DISPLAY_RETURN
        lda     #'F
        sta     $0400
        rts
