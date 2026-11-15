; Original Dragon 32 BASIC development ROM. Apache-2.0.
; Published ABI addresses are interfaces, not copied implementation code.
; Language engine and peripheral ROM routines are under construction.
        pragma  6809
        include "abi.inc"
        org     $8000
        jmp     HW_INIT
        jmp     SETUP
        jmp     KEYIN
        jmp     BLINK
        jmp     SCREENOUT
        jmp     PRINTEROUT
        jmp     JOYIN
        jmp     MOTORON
        jmp     MOTOROFF
        jmp     TAPE_PREP
        jmp     TAPE_BYTE_OUT
        jmp     TAPE_SYNC
        jmp     TAPE_BYTE_IN
        jmp     TAPE_BIT_IN

; Internal routines are packed between fixed published entry points.
; No bank, host trap, decompression area or user-RAM reservation is used.
ROM_GAP_1
        fill    $12,$84d6-*
STATEMENT
        jmp     EXECUTE_STATEMENT
ROM_GAP_2
        fill    $12,$8b2d-*
        jmp     PUBLIC_FAC_TO_INT
        jmp     PUBLIC_FAC_WORD
ROM_GAP_3
        fill    $12,$8c37-*
        jmp     INT_TO_FAC
ROM_GAP_4
        fill    $12,$a000-*
        fdb     KEYIN,CHROUT,TAPE_SYNC,TAPE_BLOCK_IN,TAPE_BLOCK_OUT,JOYIN,TAPE_PREP
ROM_GAP_5
        fill    $12,$b3b4-*
        jmp     BOOT
ROM_GAP_6
        fill    $12,$b54a-*
        jmp     CHROUT
ROM_GAP_7
        fill    $12,$b93e-*
        jmp     TAPE_BLOCK_IN
ROM_GAP_8
        fill    $12,$b999-*
        jmp     TAPE_BLOCK_OUT
ROM_GAP_9
        fill    $12,$bb40-*
        jmp     HW_INIT
ROM_GAP_10
        fill    $12,$bbb5-*
        jmp     BLINK

ROM_GAP_11
        fill    $12,$bbbb-*
        jmp     SETUP
ROM_GAP_12
        fill    $12,$bbe5-*
        jmp     KEYIN
ROM_GAP_13
        fill    $12,$bd52-*
        jmp     JOYIN
ROM_GAP_14
        fill    $12,$bda5-*
        jmp     TAPE_BIT_IN
ROM_GAP_15
        fill    $12,$bdad-*
        jmp     TAPE_BYTE_IN
ROM_GAP_16
        fill    $12,$be12-*
        jmp     TAPE_BYTE_OUT
ROM_GAP_17
        fill    $12,$be6a-*
        jmp     TAPE_PREP
ROM_GAP_18
        fill    $12,$bff0-*
        fdb     $0100,$0100,$0103,$010f,$010c,$0106,$0109,$b3b4
