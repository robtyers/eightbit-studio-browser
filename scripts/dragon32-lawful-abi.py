#!/usr/bin/env python3
"""Opt-in black-box ABI observations; requires an independently provisioned ROM.

Only original synthetic BASIC/6809 programmes and interpreted data records leave
the temporary directory. Never inspect, copy, disassemble or report ROM code.
XRoar's documented -type facility supplies input through its ROM-call intercept;
this gate does not qualify the emulator's keyboard implementation.
"""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROM_SHA256 = 'fc0e900bfec6b52f0f80ba1e65a4712808d2a411b5b00496639ef1a2152351f1'
# Original guest: TFR S,D; STD $6200; JMP $620E. The trap is in our RAM code,
# so boot-time memory-test writes cannot accidentally trigger an observation.
PROBE = bytes.fromhex('1f40fd62007e620e')
# Original CIRCLE canvases cover the polygon, arc quantisation and argument bounds.
CIRCLE_COMMANDS = [
    f"PMODE{mode},1:PCLS0:CIRCLE({x},{y}),{radius},1,{ratio}{arc}"
    for mode in range(5)
    for x, y, radius, ratio, arc in (
        (128, 96, 1, 1, ""), (128, 96, 10, 1, ""),
        (100, 90, 10, .5, ""), (129, 97, 3, 1, ""),
        (128, 96, 20, 1, ""), (128, 96, 60, .6, ""),
        (128, 96, 100, 1, ""), (128, 96, 10, 2, ""),
        (0, 0, 60, .6, ""), (255, 191, 60, .6, ""),
        (128, 96, 300, .2, ",0,.25"), (128, 96, 65535, 0, ""),
        (128, 96, 60, .6, ",.1,.2"), (128, 96, 60, .6, ",.75,.25"))
] + [f"PMODE4,1:PCLS0:CIRCLE(128,96),60,1,.6,{start},{end}"
     for start, end in ((0,.25),(.01,.02),(.1,.11),(.125,.125),(.99,1),
                       (0,.001),(0,.015625),(0,.02),(.1234,.2345))] + [
    f"PMODE4,1:PCLS0:CIRCLE(128,96),{arguments}"
    for arguments in ("-1", "65536", "10,0", "10,,256", "10,,255.999", "10,,1,-.1", "10,,1,0,1.001")
]
FIXTURES = {
    **{f'circle-raster-{index}': 'Z=0:'+source+':Z=85\r'
       for index, source in enumerate(CIRCLE_COMMANDS)},
    'radix-values': 'A=VAL("&HFFFF"):B=VAL("&177777"):C=VAL("&O177777"):D=VAL("-&H1"):E=VAL("  +&H10"):F=VAL("&H10REST")\r',
    'radix-literals': 'A=&HFFFF:B=&O177777:C=&177777:D=-&H1:E=&H8000:F=&H7FFF\r',
    'radix-digits': 'A=&8:B=&19:C=VAL("&19"):D=&O9:E=VAL("&O9"):F=VAL("&H1 0")\r',
    'radix-spaces': 'A=&H 10:B=&H1 0:C=VAL("& 177"):D=VAL("&H 10"):E=VAL("&H1 0")\r',
    'radix-empty-literal': 'A=&H:B=&O:C=&:Z=85\r',
    **{f'radix-quirk-{index}': f'A={expression}:Z=85\r'
       for index, expression in enumerate(('VAL("&19")', '&O9', 'VAL("&O9")',
           '&09', 'VAL("&09")', '&89', 'VAL("&89")', '&H1G', 'VAL("&H1G")'))},
    **{f'bits-{index}': f'A={expression}:Z=85\r'
       for index, expression in enumerate(('NOT 0', 'NOT 65535', '65535 AND 32768',
           '65535 OR 0', '-1 AND 1', 'NOT -1', '32768 OR 1', 'NOT 32768',
           '-32768 OR 32768', '65536 AND 1', '1.9 OR 2.9'))},
    'decimal-spaces': 'A=1 2:B=1 . 5:C=1 E 2:D=1 E - 2:E=VAL("1 2"):F=VAL("1 E - 2")\r',
    **{f'radix-edge-{index}': f'A={expression}:Z=85\r'
       for index, expression in enumerate(('VAL("&H")', 'VAL("&O")',
           'VAL("&8")', 'VAL("&H10000")', '&H10000', '&O200000',
           'VAL("  &H10")', 'VAL("&HFG")', 'VAL("&")', 'VAL("&Q1")'))},
    **{f'hex-boundary-{index}': f'A=HEX$({value})="{expected}":Z=85\r'
       for index, (value, expected) in enumerate((('-1', 'FFFF'), ('-32768', '8000'),
           ('-32768.9', '8000'), ('-32769', ''), ('-1.9', 'FFFF'),
           ('65535.9', 'FFFF'), ('65536', '')))},
    **{f'cassette-basic-{name}': 'CLOAD"BASIC"\rPOKE157,96:POKE158,0:' + command
       + ':POKE25346,85\rA=PEEK(25346):B=PEEK(157)*256+PEEK(158):C=PEEK(129)\r'
       for name,command in (
           ('csavem-full', 'CSAVEM"FILE",24576,24576,24576'),
           ('csavem-missing-entry', 'CSAVEM"FILE",24576,24576'),
           ('csavem-negative-entry', 'CSAVEM"FILE",24576,24576,-1'),
           ('csave-empty-name', 'CSAVE""'))},
    'cassette-basic-colon': 'POKE157,96:POKE158,0:CLOAD"BASIC":RUN\rA=PEEK(25346):B=PEEK(157)*256+PEEK(158):C=PEEK(129):EXEC25000\r',
    'cassette-basic-separate': 'POKE157,96:POKE158,0:CLOAD"BASIC"\rRUN\rA=PEEK(25346):B=PEEK(157)*256+PEEK(158):C=PEEK(129):EXEC25000\r',
    'cassette-checksum': 'SKIPF"SKIP":POKE157,95:POKE158,255:CLOADM"NATIVE"\rA=PEEK(24576):B=PEEK(157)*256+PEEK(158):C=PEEK(129)\r',
    'cassette-load': 'SKIPF"SKIP":CLOADM"NATIVE":EXEC\rA=PEEK(25345):B=PEEK(157)*256+PEEK(158):C=PEEK(129)\r',
    'cassette-offset-negative': 'SKIPF"SKIP":POKE157,96:POKE158,0:CLOADM"NATIVE",-256\rA=PEEK(24320):B=PEEK(157)*256+PEEK(158):C=PEEK(129)\r',
    'cassette-offset-positive': 'SKIPF"SKIP":CLOADM"NATIVE",256\rA=PEEK(24832):B=PEEK(157)*256+PEEK(158):C=PEEK(129)\r',
    'cassette-offset-unsigned': 'SKIPF"SKIP":CLOADM"NATIVE",65535\rA=PEEK(24575):B=PEEK(157)*256+PEEK(158):C=PEEK(129)\r',
    'line-zero': '0 A=7\r10 EXEC 25000\rRUN\r',
    'cassette-buffer-guard': 'POKE512,85\rA=PEEK(512)\r',
    'timer-assignment': 'POKE274,85:TIMER=12:A=PEEK(274):B=TIMER\r',
    'timer-negative': 'TIMER=200:TIMER=-1\rA=TIMER\r',
    'timer-overflow': 'TIMER=200:TIMER=65536\rA=TIMER\r',
    **{f'get-packed-{mode}-{width}-{height}':
       f'DIM A(0,9):PMODE{mode},1:PCLS:LINE(0,0)-({width},{height}),PSET,BF:GET(0,0)-({width},{height}),A,G\r'
       for mode in range(5) for width,height in ((0,0),(6,1),(14,3))},
    **{f'paint-gap-{x}': f'PMODE3,1:PCLS1:LINE(2,2)-(30,24),PSET,B:PSET({x},2,2):PAINT(5,5),2,4\r'
       for x in (4,6,8,10,28)},
    **{f'paint-colour-{mode}-{kind}': f'PMODE{mode},1:PCLS1:LINE(2,2)-(30,24),PSET,B:' + drawing + 'PAINT(5,5),2,4\r'
       for mode in (1,3) for kind,drawing in (
           ('vertical', 'COLOR2:LINE(10,2)-(10,24),PSET:'),
           ('horizontal', 'COLOR2:LINE(2,10)-(30,10),PSET:'),
           ('interior-row', 'COLOR2:LINE(4,10)-(28,10),PSET:'),
           ('top-row', 'COLOR2:LINE(2,2)-(30,2),PSET:'))},
    **{f'paint-before-{kind}': 'PMODE3,1:PCLS1:LINE(2,2)-(30,24),PSET,B:' + drawing + '\r'
       for kind,drawing in (('vertical', 'COLOR2:LINE(10,2)-(10,24),PSET'),
                            ('horizontal', 'COLOR2:LINE(2,10)-(30,10),PSET'))},
    'print-printer-semicolon': 'PRINT#-2,"ABC";:A=POS(-2)\r',
    'print-printer-newline': 'PRINT#-2,"ABC":A=POS(-2)\r',
    'print-printer-zone': 'PRINT#-2,"ABC","X";:A=POS(-2)\r',
    'print-printer-tab': 'PRINT#-2,"ABC";TAB(8);"X";:A=POS(-2)\r',
    'print-screen-device': 'CLS:PRINT#0,"ABC";:A=POS(0)\r',
    **{f'paint-mode-{mode}': f'PMODE{mode},1:PCLS{0 if mode%2==0 else 1}:LINE(2,2)-(30,24),PSET,B:PAINT(5,5),{1 if mode%2==0 else 2},{1 if mode%2==0 else 4}\rA=PPOINT(5,5):B=PPOINT(0,0):C=PPOINT(2,2):D=PPOINT(29,23)\r' for mode in range(5)},
    'paint-horizontal': 'PMODE3,1:PCLS1:LINE(2,2)-(30,24),PSET,B:COLOR2:LINE(2,10)-(30,10),PSET:PAINT(5,5),2,4\rA=PPOINT(5,5):B=PPOINT(20,20):C=PPOINT(10,10):D=PPOINT(0,0)\r',
    'paint-seed-painted': 'PMODE3,1:PCLS1:LINE(2,2)-(30,24),PSET,B:PSET(5,5,2):PAINT(5,5),2,4\rA=PPOINT(5,5):B=PPOINT(20,20):C=PPOINT(2,2):D=PPOINT(0,0)\r',
    'paint-zero': 'PMODE3,1:PCLS1:LINE(2,2)-(30,24),PSET,B:PAINT(5,5),0,4\rA=PPOINT(5,5):B=PPOINT(0,0):C=PPOINT(2,2):D=PPOINT(29,23)\r',
    'paint-zero-border': 'PMODE3,1:PCLS1:LINE(2,2)-(30,24),PSET,B:PAINT(5,5),2,0\rA=PPOINT(5,5):B=PPOINT(0,0):C=PPOINT(2,2):D=PPOINT(29,23)\r',
    **{f'pcls-zero-{mode}': f'PMODE{mode},1:PCLS0\rA=PPOINT(0,0):B=PPOINT(255,191)\r' for mode in (1,3)},
    'paint-default': 'PMODE4,1:PCLS0:LINE(2,2)-(30,24),PSET,B:PAINT(5,5)\rA=PPOINT(5,5):B=PPOINT(0,0):C=PPOINT(2,2):D=PPOINT(29,23)\r',
    'paint-existing': 'PMODE3,1:PCLS1:LINE(2,2)-(30,24),PSET,B:COLOR2:LINE(10,2)-(10,24),PSET:PAINT(5,5),2,4\rA=PPOINT(5,5):B=PPOINT(20,10):C=PPOINT(10,10):D=PPOINT(0,0)\r',
    'defaults': '',
    'compact-fn': '10 DEF FNF(X)=X*X+1\r20 Z=FNF(3)\r30 EXEC 25000\rRUN\r',
    'circle-ellipse': 'PMODE4,1:PCLS0:CIRCLE(128,96),60,1,.6\rA=PPOINT(188,96):B=PPOINT(128,132):C=PPOINT(68,96):D=PPOINT(128,60)\rE=PPOINT(170,121):F=PPOINT(86,121):G=PPOINT(170,71):H=PPOINT(86,71):I=PPOINT(128,96)\r',
    'circle-clipped': 'PMODE4,1:PCLS0:CIRCLE(0,0),60,1,.6\rA=PPOINT(60,0):B=PPOINT(0,36):C=PPOINT(42,25):D=PPOINT(10,10)\r',
    'else-defaults': 'A=0:IF 1 THEN CLS ELSE A=99\rIF 1 THEN LIST ELSE A=99\r',
    'mid-assignment': 'A$="HELLO":B$=A$:MID$(A$,2,2)="ABC":Z=A$="HABLO":W=B$="HELLO":A$="HELLO"\r',
    'mid-default': 'A$="HELLO":MID$(A$,2)="ABC":Z=A$="HABCO":A$="HELLO"\r',
    'mid-tail': 'A$="HELLO":MID$(A$,5)="ABC":Z=A$="HELLA":A$="HELLO"\r',
    'mid-zero': 'A$="HELLO":B$=A$:MID$(A$,2,0)="ABC":Z=A$="HABCH":W=B$="HELLO":A$="HELLO"\r',
    'mid-self': 'A$="HELLO":B$=A$:MID$(A$,2)=A$:Z=A$="HHHHH":W=B$="HELLO":A$="HELLO"\r',
    'mid-empty': 'A$="HELLO":MID$(A$,2)="":Z=A$="HELLO"\r',
    'mid-programme': '10 A$="HELLO":B$=A$:MID$(A$,2,2)="AB":Z=B$="HELLO":A$="HELLO":EXEC 25000\rRUN\r',
    'mid-array': 'DIM A$(1):A$(0)="HELLO":MID$(A$(0),1+UN)=LEFT$("XYZ",1+QX):Z=A$(0)="XELLO":A$(0)="HELLO"\r',
    'mid-zero-extent': 'CLEAR300,16000:A$="HELLO":M=VARPTR(A$):P=PEEK(M+2)*256+PEEK(M+3)\rFOR I=P+5 TO P+270:POKE I,85:NEXT I\rMID$(A$,2,0)="ABC"\rW=PEEK(P+255)=67:X=PEEK(P+256)=72:Y=PEEK(P+257)=85:A$="HELLO"\r',
    **{f'print-tab-{column}': f'CLS:PRINT"ABC";TAB({column});"X";:A=PEEK(136)*256+PEEK(137)\r' for column in (1,8,40,255)},
    **{f'print-at-{position}': f'CLS:PRINT@{position},"X";:A=PEEK(136)*256+PEEK(137)\r' for position in (0,511,512)},
    **{f'rng-{count}': 'A=RND(0)\r' * count for count in range(1,7)},
    'rng-negative': 'A=RND(-1):B=RND(0):C=RND(-1):D=RND(0)\r',
    **{f'rng-seed-{index}': f'A=RND(-{number})\r' for index,number in enumerate(('1','2','1.5','123','256','.5'))},
    # Selected normalisation, halfway-up and near-zero return counterexamples.
    # Only original synthetic inputs are defined here; no ROM data is embedded.
    **{f'rng-return-{seed:08x}':
       ':'.join(f'POKE {278+i},{byte}' for i,byte in enumerate(seed.to_bytes(4, 'big')))
       + ':B=RND(0):Z=85\r'
       for seed in (0x4fc75259, 0x50c22e35, 0x22e933f2, 0x85b0658b,
                    0xc9a142db, 0x8bd378c6, 0x72cc784b,
                    0xec72585f, 0x2c72585f, 0xcc72585f,
                    0x6c72585f, 0x418c9d62)},
    **{f'rng-string-error-{index}-{len(value)}':
       before + ':Q=0\rA=RND("' + value + '"):Q=99\rB=RND(0):Z=85\r'
       for index,before in enumerate(('X=1', 'X=-1', 'X=RND(-1)'))
       for value in ('', 'X', 'HELLO')},
    'rng-positive': 'A=RND(6)\r',
    **{f'rng-positive-boundary-{index}': f'A=RND(-1):B=RND({number}):Z=85\r'
       for index, number in enumerate(('.5', '1.5', '1.9', '2.5', '32768', '65535', '65536', '1E20'))},
    **{f'rng-fractional-boundary-{index}':
       'POKE278,255:POKE279,255:POKE280,255:POKE281,255:' + f'B=RND({number}):Z=85\r'
       for index, number in enumerate(('.5', '1.5', '1.9', '2.5'))},
    'def-fn': '10 DEF FN F(X)=X*X+1\r20 Z=FN F(3)\r30 EXEC 25000\rRUN\r',
    'string-functions': 'A=LEFT$("HELLO",2)="HE":B=RIGHT$("HELLO",2)="LO":C=MID$("HELLO",2,3)="ELL":D=MID$("HELLO",8)=""\rE=INSTR("HELLO","LL"):F=INSTR(3,"HELLO","L"):G=INSTR(8,"HELLO",""):H=STRING$(3,65)="AAA"\rI=HEX$(-1)="FFFF":J=STR$(12)=" 12"\r',
    **{f'graphics-mode-{mode}': f'PMODE {mode},1:PCLS:PSET(255,191):A=PPOINT(255,191)\r' for mode in range(5)},
    'graphics-colours': 'PMODE 4,1:PCLS:PSET(0,0,1):A=PPOINT(0,0):PSET(1,0,2):B=PPOINT(1,0):COLOR 2,3:PSET(2,0):C=PPOINT(2,0)\r',
    'graphics-defaults': '',
    **{f'play-volume-{volume}': f'PLAY"V{volume}"\r' for volume in (0, 15, 31)},
    'graphics-shrink': 'PMODE 4,1:PCLEAR 1\r',
    'graphics-clear-zero': 'PCLEAR 0\r',
    'graphics-clear': 'PCLEAR 2\r',
    'arrays': 'DIM A(2,3),B$(2)\rA(1,2)=123:B$(1)="HELLO":Z=7\r',
    'array-order': 'DIM A(1,2)\rA(0,0)=1:A(1,0)=2:A(0,1)=3:A(1,1)=4:A(0,2)=5:A(1,2)=6\r',
    'implicit-array': 'A(10)=9\r',
    'strings': 'A$="HELLO":B$=A$:C$=A$+"!":Z=7\r',
    'programme-strings': '10 A$="HELLO":B$=A$:C$=A$+"!":EXEC 25000\rRUN\r',
    'clear-reserve': 'CLEAR 50\r',
    'clear-retain': 'CLEAR 50\rCLEAR\r',
    'clear-limit': 'CLEAR 100,20000\r',
    'gosub': '10 GOSUB 100\r20 END\r100 EXEC 25000\r110 RETURN\rRUN\r',
    'for-positive': '10 FOR I=2 TO 5 STEP 2\r20 EXEC 25000\r30 NEXT I\rRUN\r',
    'for-negative': '10 FOR I=5 TO -2 STEP -2\r20 EXEC 25000\r30 NEXT I\rRUN\r',
    'for-zero': '10 FOR I=1 TO 5 STEP 0\r20 EXEC 25000\r30 NEXT I\rRUN\r',
    'for-fraction': '10 FOR I=.5 TO 3.25 STEP .125\r20 EXEC 25000\r30 NEXT I\rRUN\r',
    'nested': '10 FOR I=2 TO 5 STEP 2\r20 GOSUB 100\r30 NEXT I\r40 END\r100 EXEC 25000\r110 RETURN\rRUN\r',
    'stop-for': '10 FOR I=1 TO 2\r20 STOP\r30 NEXT I\r40 END\rRUN\rEXEC 25000\r',
    'stop-gosub': '10 GOSUB 100\r20 END\r100 STOP\r110 RETURN\rRUN\rEXEC 25000\r',
    'continue-for': '10 FOR I=1 TO 2\r20 STOP\r30 I=I+5\r40 NEXT I\r50 EXEC 25000\rRUN\rCONT\r',
    'data-read': '10 DATA 1,"HELLO",2:DATA 3\r20 READ X,Y$\r30 EXEC 25000\rRUN\r',
    'data-restore': '10 DATA 1,"HELLO",2\r20 READ X,Y$\r30 RESTORE\r40 EXEC 25000\rRUN\r',
}


def word(data, address):
    return int.from_bytes(data[address:address + 2], 'big')


def value_record(data, address, string):
    raw = data[address:address + 5]
    if string:
        length, pointer = raw[0], word(raw, 2)
        if length and not (0x200 <= pointer <= 0x7fff - length):
            raise ValueError('String data must lie wholly within RAM')
        text = data[pointer:pointer + length]
        # All test strings are original printable ASCII. Refuse an unexpected
        # pointer into code or arbitrary binary state instead of persisting it.
        if text not in (b'', b'HELLO', b'HELLO!'):
            raise ValueError('Unexpected synthetic string contents')
        return {'descriptor': list(raw), 'text': text.decode('ascii')}
    return {'packed': list(raw)}


def observe(data):
    if len(data) != 32768:
        raise ValueError('Expected exactly 32K of volatile RAM')
    start, arrays, end = [word(data, a) for a in (0x1b, 0x1d, 0x1f)]
    if not (0x0603 <= start <= arrays <= end < 0x6000) or (arrays - start) % 7:
        raise ValueError('Unexpected variable-table boundaries')
    scalars = []
    for address in range(start, arrays, 7):
        name = data[address:address + 2]
        if not 65 <= (name[0] & 127) <= 90 or (name[1] & 127 and not chr(name[1] & 127).isalnum()):
            raise ValueError('Unexpected synthetic variable name')
        if name[0] & 128:
            raw = data[address + 2:address + 7]
            # Report RAM metadata only; an unexpected pointer never discloses
            # an OEM internal instruction/return address.
            words = [word(raw, offset) for offset in (0, 2)]
            value = {'functionRamWords': [w if 0x600 <= w < 0x6000 else None for w in words],
                     'tailIsZero': raw[4] == 0}
        else:
            value = value_record(data, address + 2, bool(name[1] & 128))
        scalars.append({'name': list(name), 'value': value})
    records = []
    while arrays < end:
        length, count = word(data, arrays + 2), data[arrays + 4]
        header = 5 + 2 * count
        if not (1 <= count <= 8 and header <= length <= end - arrays):
            raise ValueError('Unexpected array descriptor')
        extents = [word(data, arrays + 5 + i * 2) for i in range(count)]
        elements = 1
        for extent in extents:
            if not 1 <= extent <= 8192:
                raise ValueError('Unexpected array extent')
            elements *= extent
        if elements * 5 != length - header:
            raise ValueError('Array storage does not match its extents')
        name = data[arrays:arrays + 2]
        if not 65 <= name[0] <= 90 or (name[1] & 127 and not chr(name[1] & 127).isalnum()):
            raise ValueError('Unexpected synthetic array name')
        values = [value_record(data, arrays + header + i * 5, bool(name[1] & 128))
                  for i in range(elements)]
        records.append({'name': list(name), 'recordLength': length,
                        'storedExtents': extents, 'values': values})
        arrays += length
    sp, top = word(data, 0x6200), word(data, 0x21)
    if not top - 512 <= sp <= top - 3:
        raise ValueError('Unexpected guest stack boundaries')
    position = sp + 2  # Ignore the implementation-specific EXEC return address.
    frames = []
    while position < top - 1:
        tag = data[position]
        if tag == 0xbd and position + 7 <= top - 1:
            frames.append({'tag': tag, 'size': 7, 'line': word(data, position + 1),
                           'textPointer': word(data, position + 3)})
            # The final two bytes are an implementation-specific return address.
            position += 7
        elif tag == 0x80 and position + 18 <= top - 1:
            frames.append({'tag': tag, 'size': 18, 'variablePointer': word(data, position + 1),
                           'stepMagnitude': list(data[position + 3:position + 8]),
                           'stepSign': data[position + 8],
                           'limit': list(data[position + 9:position + 14]),
                           'line': word(data, position + 14),
                           'textPointer': word(data, position + 16)})
            position += 18
        else:
            raise ValueError('Unexpected control-stack record; no raw stack is retained')
    if data[position:top + 1] != b'\0\0':
        raise ValueError('Expected the two-byte stack sentinel')
    return {'workspace': {f'{a:04x}': word(data, a)
                          for a in (0x19, 0x1b, 0x1d, 0x1f, 0x21, 0x23, 0x27, 0x29, 0x2f, 0x31, 0x33, 0x68)},
            'random': {'seed': list(data[0x115:0x11a]), 'scratch': list(data[0xab:0xaf])},
            'graphics': {'foreground': data[0xb2], 'background': data[0xb3], 'mode': data[0xb6],
                         'last': word(data, 0xb7), 'stride': data[0xb9], 'first': word(data, 0xba),
                         'originHigh': data[0xbc], 'x': word(data, 0xbd), 'y': word(data, 0xbf),
                         'colourSet': data[0xc1], 'drawAngle': data[0xe8], 'drawScale': data[0xe9],
                         'volumeHigh': data[0xdf], 'volumeLow': data[0xe0],
                         'octave': data[0xde], 'noteLength': data[0xe1], 'tempo': data[0xe2]},
            'scalars': scalars, 'arrays': records, 'controlFrames': frames,
            'stackSentinelBytes': 2}


def observe_synthetic_graphics(data):
    """Digest only the canvas cleared/drawn by our original PAINT fixtures."""
    if len(data) != 32768:
        raise ValueError('Expected exactly 32K of volatile RAM')
    mode = data[0xb6]
    if mode > 4:
        raise ValueError('Unexpected synthetic graphics mode')
    start, end = word(data, 0xba), word(data, 0xb7)
    size = (1536, 3072, 3072, 6144, 6144)[mode]
    if start != 0x600 or end != start + size or end > word(data, 0x19):
        raise ValueError('Synthetic graphics canvas escaped its reviewed pages')
    return {'mode': mode, 'bytes': size,
            'sha256': hashlib.sha256(data[start:end]).hexdigest()}


def observe_synthetic_sprite(data):
    """Read only the one bounded array filled from our cleared synthetic canvas.

    GET's G form changes an extent to zero. Keep this observation separate from
    the ordinary variable validator; it is not permission to follow bad records.
    """
    if len(data) != 32768:
        raise ValueError('Expected exactly 32K of volatile RAM')
    start, end = word(data, 0x1d), word(data, 0x1f)
    if (start != 0x1e0a or end != start + 59 or
            word(data, 0x1b) != 0x1e03 or data[0x1e03:0x1e0a] != b'A\0\0\0\0\0\0' or
            data[start:start + 5] != bytes((65, 0, 0, 59, 2)) or
            word(data, start + 5) != 10 or word(data, start + 7) not in (0, 1)):
        raise ValueError('Unexpected reviewed synthetic sprite array')
    observe_synthetic_graphics(data)
    return {'recordLength': 59, 'dimensions': 2, 'createsZeroScalar': True,
            'storedExtents': [10, word(data, start + 7)],
            'syntheticPayload': list(data[start + 9:end])}


def observe_synthetic_cassette(data, basic=False):
    """Only the reviewed original marker/status/RAM entry values may leave RAM."""
    if len(data) != 32768:
        raise ValueError('Expected the bounded synthetic cassette snapshot')
    start, end = word(data, 0x1b), word(data, 0x1d)
    expected = 0x1e11 if basic else 0x1e03
    if start != expected or end != expected + 21:
        raise ValueError('Unexpected synthetic cassette variable layout')
    if basic and data[0x1e01:0x1e11] != (bytes.fromhex('1e0f000a93') + b'25346,85' + bytes(3)):
        raise ValueError('Unexpected synthetic tokenised programme')
    values = {}
    for position, name in zip(range(start, end, 7), b'ABC'):
        if data[position:position + 2] != bytes((name, 0)):
            raise ValueError('Unexpected synthetic cassette variable name')
        raw = data[position + 2:position + 7]
        value = 0 if raw[0] == 0 else ((int.from_bytes(raw[1:], 'big') | 0x80000000)
                                            * 2 ** (raw[0] - 160))
        if value != int(value) or raw[1] & 128:
            raise ValueError('Unexpected synthetic cassette scalar')
        values[chr(name)] = int(value)
    if (values['A'] not in ((0, 85) if basic else (0, 165, 204))
            or values['B'] not in ((24576,) if basic else (24575, 24576, 24832))
            or values['C'] not in ((0,) if basic else (0, 1, 2, 3))):
        raise ValueError('Unexpected synthetic cassette marker, RAM entry or status')
    return {'markerByte': values['A'], 'executionAddress': values['B'], 'cassetteStatus': values['C'],
            'fastTapeTrapsDisabled': True}


def run_probe(executable, rom, fixture, graphics=False, sprite=False, cassette=False, corrupt_cassette=False, basic_cassette=False):
    pokes = ':'.join(f'POKE {25000 + i},{b}' for i, b in enumerate(PROBE)) + '\r'
    text = pokes + fixture
    if 'RUN\r' not in fixture:
        text += 'EXEC 25000\r'
    with tempfile.TemporaryDirectory(prefix='eightbit-lawful-abi-') as directory:
        snapshot = Path(directory) / 'state.ram'
        command = [str(executable), '-no-c', '-no-config-auto-save', '-verbose', '0',
                   '-ui', 'null', '-ao', 'null', '-m', 'dragon32', '-extbas', str(rom),
                   '-rompath', directory, '-machine-cart', 'none', '-no-cart-autorun',
                   '-ram-init', 'clear', '-no-ratelimit', '-type', text,
                   '-trap', 'pc=25102', '-trap-snap', str(snapshot),
                   '-trap-timeout', '0.01', '-timeout', '15']
        if cassette:
            spec = importlib.util.spec_from_file_location(
                'original_cassette', Path(__file__).with_name('dragon32-original-reference.py'))
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
            media = Path(directory) / 'original-input.cas'
            content = bytearray(module.cassette_fixture())
            if corrupt_cassette:
                block = bytes.fromhex('553c0107cc00a5fd630039')
                content[content.index(block) + len(block)] ^= 1
            media.write_bytes(content)
            command += ['-load-tape', str(media), '-no-tape-fast', '-no-tape-pad-auto']
        with (Path(directory) / 'reference.log').open('wb') as log:
            result = subprocess.run(command, cwd=directory, stdin=subprocess.DEVNULL,
                                    stdout=log, stderr=log, timeout=30, check=False,
                                    env={'PATH': '/usr/bin:/bin', 'LANG': 'C'})
        if result.returncode or not snapshot.is_file() or snapshot.stat().st_size != 32768:
            raise RuntimeError('The synthetic reference programme did not reach its RAM trap')
        data = snapshot.read_bytes()
        result = ({'syntheticCassette': observe_synthetic_cassette(data, basic=basic_cassette)} if cassette else
                  {'syntheticSprite': observe_synthetic_sprite(data)} if sprite else observe(data))
        if graphics:
            result['syntheticGraphics'] = observe_synthetic_graphics(data)
        return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument('--configured', action='store_true', help='Use the saved macOS Dragon runtime selection')
    source.add_argument('--rom', type=Path, help='Explicit lawfully provisioned Dragon 32 1982 ROM')
    parser.add_argument('--xroar', type=Path)
    parser.add_argument('--probe', choices=tuple(FIXTURES), action='append')
    args = parser.parse_args()
    os.umask(0o077)
    if args.configured:
        settings = Path.home() / 'Library/Application Support/com.eightbitstudio.m6809/runtime-settings.json'
        with settings.open('rb') as stream:
            raw = stream.read(32769)
        if len(raw) > 32768:
            raise ValueError('Runtime selection exceeds its size limit')
        runtime = json.loads(raw)['runtime']
        args.rom = Path(runtime['romPath'])
        args.xroar = args.xroar or Path(runtime['xroarPath'])
    if not args.xroar:
        parser.error('--xroar is required with --rom')
    rom, executable = args.rom.resolve(strict=True), args.xroar.resolve(strict=True)
    with rom.open('rb') as stream:
        image = stream.read(16385)
    if len(image) != 16384 or hashlib.sha256(image).hexdigest() != ROM_SHA256:
        raise ValueError('Only the reviewed exact Dragon 32 1982 fingerprint is permitted')
    del image
    observations = {name: run_probe(executable, rom, FIXTURES[name],
                                     graphics=name.startswith(('paint-', 'pcls-zero-', 'circle-raster-')),
                                     sprite=name.startswith('get-packed-'),
                                     cassette=name.startswith('cassette-') and name != 'cassette-buffer-guard',
                                     corrupt_cassette=name == 'cassette-checksum',
                                     basic_cassette=name.startswith('cassette-basic-'))
                    for name in args.probe or FIXTURES}
    print(json.dumps({'schemaVersion': 1, 'referenceRomSha256': ROM_SHA256,
                      'method': 'original synthetic programmes; XRoar ROM-call input intercept',
                      'romInstructionsInspected': False, 'observations': observations}, indent=2))


if __name__ == '__main__':
    main()
