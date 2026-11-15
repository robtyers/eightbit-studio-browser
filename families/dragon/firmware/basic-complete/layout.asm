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
        include "mid-assignment.inc"
        include "functions.inc"
        include "syntax-open.inc"
        include "arrays.inc"
ROM_GAP_1

STATEMENT
        jmp     EXECUTE_STATEMENT
        include "graphics-sprite.inc"
        include "sound.inc"
        include "string-functions.inc"
        include "circle.inc"
        include "errors.inc"
ROM_GAP_2

        jmp     FAC_TO_INT
        jmp     FAC_TO_INT
        include "tape-block-out.inc"
        include "trace.inc"
        include "graphics-copy.inc"
        include "command-guards.inc"
ROM_GAP_3

        jmp     INT_TO_FAC
        include "extended-control.inc"
        include "statement-arguments.inc"
        include "devices.inc"
        include "expression.inc"
        include "draw.inc"
        include "strings.inc"
        include "function-call.inc"
        include "control.inc"
        include "tape-ascii-in.inc"
        include "value-functions.inc"
        include "graphics-line.inc"
        include "graphics-mode.inc"
        include "math.inc"
        include "play.inc"
        include "tape-basic.inc"
        include "graphics-cls.inc"
ROM_GAP_4

        fdb     KEYIN,CHROUT,TAPE_SYNC,TAPE_BLOCK_IN,TAPE_BLOCK_OUT,JOYIN,TAPE_PREP
        include "console.inc"
        include "graphics-screen.inc"
        include "graphics-paint.inc"
        include "function-definitions.inc"
        include "native-frame.inc"
        include "data.inc"
        include "number.inc"
        include "delete.inc"
        include "tape-read.inc"
        include "graphics-line-core.inc"
        include "graphics-point.inc"
        include "keywords.inc"
        include "commands.inc"
ROM_GAP_5

        jmp     BOOT
        include "numeric.inc"
        include "random-init.inc"
        include "graphics-box.inc"
        include "graphics-fill-box.inc"
        include "random.inc"
ROM_GAP_6

        jmp     CHROUT
        include "storage.inc"
        include "graphics-page.inc"
        include "complete-format.inc"
        include "strings-gc.inc"
ROM_GAP_7

        jmp     TAPE_BLOCK_IN
        include "graphics-copy-page.inc"
ROM_GAP_8

        jmp     TAPE_BLOCK_OUT
        include "tokenise.inc"
ROM_GAP_9

        jmp     HW_INIT
ROM_GAP_10

        jmp     BLINK

ROM_GAP_11

        jmp     SETUP
        include "print-position.inc"
        include "unsigned.inc"
ROM_GAP_12

        jmp     KEYIN
        include "tape-write.inc"
        include "graphics-low.inc"
        include "tape-block-in.inc"
ROM_GAP_13

        jmp     JOYIN
        include "series.inc"
ROM_GAP_14

        jmp     TAPE_BIT_IN
        include "function-name.inc"
ROM_GAP_15

        jmp     TAPE_BYTE_IN
        include "exec.inc"
        include "print-channel.inc"
        include "print-tab.inc"
ROM_GAP_16

        jmp     TAPE_BYTE_OUT
        include "function-return.inc"
        include "scanner.inc"
ROM_GAP_17

        jmp     TAPE_PREP
        include "line-range.inc"
        include "fp-program.inc"
        include "strings-roots.inc"
ROM_GAP_18

        fdb     $0100,$0100,$0103,$010f,$010c,$0106,$0109,$b3b4

        include "complete-cassette.inc"
        include "renum.inc"
        include "edit.inc"
        include "dload.inc"

        include "break.inc"
        include "byte-templates.inc"
