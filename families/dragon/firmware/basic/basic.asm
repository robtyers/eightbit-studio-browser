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
        include "string-functions.inc"
        include "circle.inc"
        include "errors.inc"
        include "delete.inc"
        include "series.inc"
        include "graphics-copy.inc"
        include "command-guards.inc"
        include "graphics-box.inc"
        include "graphics-fill-box.inc"
ROM_GAP_1
        fill    $12,$84d6-*
STATEMENT
        jmp     EXECUTE_STATEMENT
        include "number.inc"
        include "function-call.inc"
        include "tape-block-out.inc"
        include "strings.inc"
        include "math.inc"
        include "tape-ascii-in.inc"
ROM_GAP_2
        fill    $12,$8b2d-*
        jmp     FAC_TO_INT
        jmp     FAC_TO_INT
        include "break.inc"
        include "input-start.inc"
        include "numeric.inc"
ROM_GAP_3
        fill    $12,$8c37-*
        jmp     INT_TO_FAC
        include "graphics-line-core.inc"
        include "graphics-point.inc"
        include "graphics-low.inc"
        include "commands.inc"
        include "control.inc"
        include "storage.inc"
        include "graphics-sprite.inc"
        include "draw.inc"
        include "console.inc"
        include "value-functions.inc"
ROM_GAP_4
        fill    $12,$a000-*
        fdb     KEYIN,CHROUT,TAPE_SYNC,TAPE_BLOCK_IN,TAPE_BLOCK_OUT,JOYIN,TAPE_PREP
        include "arrays.inc"
        include "trace.inc"
        include "tokenise.inc"
        include "function-definitions.inc"
        include "native-frame.inc"
        include "data.inc"
        include "sound.inc"
        include "play.inc"
        include "keywords.inc"
        include "format.inc"
        include "fp-program.inc"
        include "extended-control.inc"
        include "statement-arguments.inc"
        include "print-device.inc"
        include "devices.inc"
        include "expression.inc"
        include "mid-assignment.inc"
        include "functions.inc"
ROM_GAP_5
        fill    $12,$b3b4-*
        jmp     BOOT
        include "tape-write.inc"
        include "strings-roots.inc"
        include "tape-read.inc"
ROM_GAP_6
        fill    $12,$b54a-*
        jmp     CHROUT
        include "tape-load.inc"
        include "graphics-paint.inc"
        include "graphics-line.inc"
        include "graphics-screen.inc"
        include "graphics-mode.inc"
ROM_GAP_7
        fill    $12,$b93e-*
        jmp     TAPE_BLOCK_IN
        include "random-init.inc"
        include "print-channel.inc"
        include "print-tab.inc"
        include "graphics-copy-page.inc"
ROM_GAP_8
        fill    $12,$b999-*
        jmp     TAPE_BLOCK_OUT
        include "graphics-cls.inc"
        include "strings-gc.inc"
        include "tape-block-in.inc"
        include "input-variable.inc"
ROM_GAP_9
        fill    $12,$bb40-*
        jmp     HW_INIT
        include "input-store.inc"
        include "input-end.inc"
        include "input-value.inc"
ROM_GAP_10
        fill    $12,$bbb5-*
        jmp     BLINK

ROM_GAP_11
        fill    $12,$bbbb-*
        jmp     SETUP
        include "print-position.inc"
        include "unsigned.inc"
ROM_GAP_12
        fill    $12,$bbe5-*
        jmp     KEYIN
        include "line-range.inc"
        include "tape-save.inc"
        include "tape-ascii-out.inc"
ROM_GAP_13
        fill    $12,$bd52-*
        jmp     JOYIN
        include "tape-basic.inc"
        include "function-name.inc"
ROM_GAP_14
        fill    $12,$bda5-*
        jmp     TAPE_BIT_IN
        include "syntax-open.inc"
ROM_GAP_15
        fill    $12,$bdad-*
        jmp     TAPE_BYTE_IN
        include "errors-data.inc"
        include "exec.inc"
        include "graphics-page.inc"
ROM_GAP_16
        fill    $12,$be12-*
        jmp     TAPE_BYTE_OUT
        include "scanner.inc"
        include "function-return.inc"
ROM_GAP_17
        fill    $12,$be6a-*
        jmp     TAPE_PREP
        include "tape-file.inc"
        include "random.inc"
ROM_GAP_18
        fill    $12,$bff0-*
        fdb     $0100,$0100,$0103,$010f,$010c,$0106,$0109,$b3b4

