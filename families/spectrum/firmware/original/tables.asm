ORG $3a00
INITIAL_SYSTEM_VARIABLES:
DB 0,0,0,0,0,0,0,0,0,35,5,0,0,0,0,0
; Streams -3..15, followed by CHARS and the published variable block.
DW 1,6,11,1,1,6,16
DS 24,0
DW $3c00
DB 64,0,255,0,0
DW $ff50,0
DB 0
DW 0
DB 0
DW 0
DB 0,56
DW 0,$5ccb,0,$5cb6,$5cbb,$5ccb,$5ccb,$5ccb,$5ccc,$5ccc,$5ccc,0,$5ccc,$5ccc,$5ccc
DB 0
DW $5c92
DB 0,2
DW 0,0
DB 0,0
DW 0,0,0
DB 0,0,0
DW $ff58
DB 0,0,33
DW $5b00,$1821,$4000,$50e0,$1821,$0221
DB 1,56,0,56,0,0
DS 30,0
DW 0,$ff57,$ffff
; The bootstrap copies only the published 203-byte system-variable extent.
ORG $3b00
INITIAL_CHANNELS:
DW SCREEN_CHAR,READ_KEY
DB 'K'
DW SCREEN_CHAR,CHANNEL_INPUT_ERROR
DB 'S'
DW SCREEN_CHAR,CHANNEL_INPUT_ERROR
DB 'R'
DW PRINTER_CHAR,CHANNEL_INPUT_ERROR
DB 'P'
DB $80
KEYS_LETTERS:
DB 0,"zxcvasdfgqwert1234509876poiuy",13,"lkjh ",0,"mnb"
KEYS_SYMBOLS:
DB 0,':',96,'?','/',226,195,205,204,203,199,201,200,'<','>','!','@','#','$','%','_',')','(',39,'&','"',';',172,198,197,13,'=','+','-','^',' ',0,'.',',','*'
KEYS_CURSOR:
DB 8,10,11,9
BOOT_TEXT:
DB "EIGHTBIT SPECTRUM 48K",13,0
BOOT_128_TEXT:
DB "EIGHTBIT SPECTRUM 128K",13,0
BOOT_PLUS2_TEXT:
DB "EIGHTBIT SPECTRUM +2",13,0
READY_TEXT:
DB "> ",0
ERROR_TEXT:
DB "ERROR",13,0
ALLOC_POINTERS:
DW $5c4b,$5c4d,$5c4f,$5c51,$5c53,$5c55,$5c57,$5c59,$5c5b,$5c5d,$5c5f,$5c61,$5c63,$5c65,$5c0b,0

ORG $3c00
; Quadrant bits from the original manual: top right, top left, bottom right, bottom left.
BLOCK_GLYPHS:
DB $00,$00,$00,$00,$00,$00,$00,$00
DB $0f,$0f,$0f,$0f,$00,$00,$00,$00
DB $f0,$f0,$f0,$f0,$00,$00,$00,$00
DB $ff,$ff,$ff,$ff,$00,$00,$00,$00
DB $00,$00,$00,$00,$0f,$0f,$0f,$0f
DB $0f,$0f,$0f,$0f,$0f,$0f,$0f,$0f
DB $f0,$f0,$f0,$f0,$0f,$0f,$0f,$0f
DB $ff,$ff,$ff,$ff,$0f,$0f,$0f,$0f
DB $00,$00,$00,$00,$f0,$f0,$f0,$f0
DB $0f,$0f,$0f,$0f,$f0,$f0,$f0,$f0
DB $f0,$f0,$f0,$f0,$f0,$f0,$f0,$f0
DB $ff,$ff,$ff,$ff,$f0,$f0,$f0,$f0
DB $00,$00,$00,$00,$ff,$ff,$ff,$ff
DB $0f,$0f,$0f,$0f,$ff,$ff,$ff,$ff
DB $f0,$f0,$f0,$f0,$ff,$ff,$ff,$ff
DB $ff,$ff,$ff,$ff,$ff,$ff,$ff,$ff
