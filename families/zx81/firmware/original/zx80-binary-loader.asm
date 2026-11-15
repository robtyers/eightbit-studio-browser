; Original cassette bootstrap, Apache-2.0. No external assembler/ROM implementation.
; The ZX80 scans REM to newline (76h), so arbitrary code is encoded as nibbles.
; Decode forwards in place without returning to the BASIC line scanner mid-copy.
; COUNT/COUNT_ADJUST_BYTE are bounded original-builder inputs, not source hooks.
ORG $402b
LD HL,$4050
LD DE,$4050
LD BC,COUNT
DB COUNT_ADJUST_BYTE
DECODE:
LD A,(HL)
SUB 28
RLCA
RLCA
RLCA
RLCA
LD (DE),A
INC HL
LD A,(HL)
SUB 28
EX DE,HL
OR (HL)
LD (HL),A
INC HL
EX DE,HL
INC HL
DEC BC
LD A,B
OR C
JR NZ,DECODE
JP $4050
ORG $4050
