; Explicit screen/printer PRINT selection. Cassette/file devices remain closed
; until their guest file lifecycle and legacy buffers are implemented.
PRINT_DEVICE
        leax    1,x
        bsr     EVAL_NUMBER
        jsr     FAC_TO_INT
        bsr     ST_CHECK
        cmpd    #0
        beq     PRINT_DEVICE_READY
        cmpd    #-2
        bsr     NATIVE_CHECK_NE
PRINT_DEVICE_READY
        bsr     MULTI_COMMA
        ldu     #CMD_PRINT_NUMBER
        jmp     PRINT_SELECTED

