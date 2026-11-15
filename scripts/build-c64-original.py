#!/usr/bin/env python3
"""Reproducible original C64 foundation firmware; explicitly selected external cc65."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct
import subprocess
import tempfile
import selectors
import signal
import stat
import time

ROOT = Path(__file__).resolve().parent.parent
SOURCE = ROOT / 'families/commodore/firmware/original'
KEYWORDS = ROOT / 'families/commodore/crates/eightbit-commodore-core/tests/fixtures/basic-v2-keywords.txt'
INPUTS = [SOURCE / name for name in ['arrays.inc', 'basic.s', 'branches.inc', 'characters.json', 'channels.inc', 'control.inc', 'continue.inc', 'data.inc', 'editor.inc', 'expressions.inc', 'files.inc', 'functions.inc', 'input.inc', 'kernal.s', 'linker.cfg', 'loops.inc', 'math-constants.inc', 'memory.inc', 'numbers.s', 'pages.s', 'square-root.inc', 'strings.inc', 'time.inc', 'transcendentals.inc']] + [KEYWORDS]


def digest(data):
    return hashlib.sha256(data).digest()


def font(document):
    if set(document) != {'schemaVersion', 'provenance', 'glyphs'} or document['schemaVersion'] != 1:
        raise ValueError('Invalid original character source schema')
    def letter(char):
        rows = document['glyphs'].get(char)
        if rows is None:
            raise ValueError('Missing independently drawn original character')
        if len(rows) != 7 or any(type(value) is not int or not 0 <= value < 32 for value in rows):
            raise ValueError('Invalid original five-column character')
        return bytes([0] + [value << 1 for value in rows])
    def geometry(code):
        # Original continuous cell geometry. The manual supplies symbol roles,
        # never an extracted pixel grid; these strokes/curves are authored here.
        rules = {
            64: lambda x,y: y == 3, 65: lambda x,y: (abs(2*x-7)+abs(2*y-6)<8 and y<6) or (x in [3,4] and y>=5),
            66: lambda x,y: x==3, 67: lambda x,y: y==4, 68: lambda x,y: y==2,
            69: lambda x,y: y==1, 70: lambda x,y: y==6, 71: lambda x,y:x==1,
            72: lambda x,y:x==6, 73: lambda x,y: x*x+(7-y)*(7-y) in range(40,57),
            74: lambda x,y: (7-x)*(7-x)+y*y in range(40,57),
            75: lambda x,y:x*x+y*y in range(40,57), 76: lambda x,y:y==7,
            77: lambda x,y:x==y, 78:lambda x,y:x+y==7, 79:lambda x,y:y==0,
            80:lambda x,y:y==2, 81:lambda x,y:(2*x-7)**2+(2*y-7)**2<38,
            82:lambda x,y:y==5, 83:lambda x,y: (y in [1,2] and x in [1,2,5,6]) or (2<=y<=6 and abs(2*x-7)<2*(7-y)),
            84:lambda x,y:x==2, 85:lambda x,y:(7-x)*(7-x)+(7-y)*(7-y) in range(40,57),
            86:lambda x,y:x==y or x+y==7, 87:lambda x,y:27<=(2*x-7)**2+(2*y-7)**2<=55,
            88:lambda x,y:((x-3)**2+(y-2)**2<5 or (x-1)**2+(y-4)**2<5 or (x-5)**2+(y-4)**2<5) or (x in [3,4] and y>=5),
            89:lambda x,y:x==5, 90:lambda x,y:abs(2*x-7)+abs(2*y-7)<9,
            91:lambda x,y:x in [3,4] or y in [3,4], 92:lambda x,y:x<4 and (x+y)%2==0,
            93:lambda x,y:x==4, 95:lambda x,y:x>=y, 96:lambda x,y:False,
            97:lambda x,y:x<4, 98:lambda x,y:y>=4, 99:lambda x,y:y==0,
            100:lambda x,y:y==7, 101:lambda x,y:x==0, 102:lambda x,y:(x+y)%2==0,
            103:lambda x,y:x==7, 104:lambda x,y:y>=4 and (x+y)%2==0,
            105:lambda x,y:x+y<8, 106:lambda x,y:x>=6,
            107:lambda x,y:x in [3,4] or (y in [3,4] and x>=4),
            108:lambda x,y:x>=4 and y>=4, 109:lambda x,y:(x==4 and y<=4) or (y==4 and x>=4),
            110:lambda x,y:(x==3 and y<=3) or (y==3 and x<=3), 111:lambda x,y:y>=6,
            112:lambda x,y:(x==4 and y>=4) or (y==4 and x>=4),
            113:lambda x,y:y in [3,4] or (x in [3,4] and y<=3),
            114:lambda x,y:y in [3,4] or (x in [3,4] and y>=4),
            115:lambda x,y:x in [3,4] or (y in [3,4] and x<=3),
            116:lambda x,y:x<2, 117:lambda x,y:x<3, 118:lambda x,y:x>=5,
            119:lambda x,y:y<2, 120:lambda x,y:y<3, 121:lambda x,y:y>=5,
            122:lambda x,y:x==7 or y==7, 123:lambda x,y:x<4 and y>=4,
            124:lambda x,y:x>=4 and y<4,
            125:lambda x,y:(x==3 and y<=3) or (y==3 and x<=3),
            126:lambda x,y:x<4 and y<4, 127:lambda x,y:(x//4+y//4)%2==1,
        }
        if code == 94:
            return bytes([0,0,126,36,36,36,36,0])  # original pi lettering
        return bytes(sum((128>>x) for x in range(8) if rules[code](x,y)) for y in range(8))
    sets=[]
    for alternate in [False,True]:
        cells=[]
        for code in range(128):
            if code<32:
                char=chr(code+64)
                if alternate and 1<=code<=26: char=char.lower()
                cell=letter(char)
            elif code<64: cell=letter(chr(code))
            elif alternate and 64<=code<=90: cell=letter(chr(code))
            elif alternate and code in [94,95,105,122]:
                cell=bytes(sum((128>>x) for x in range(8) if (
                    (x//2+y//2)%2==0 if code==94 else
                    (x+y)%4==0 if code==95 else (x-y)%4==0 if code==105 else
                    (x==y-3 and 4<=y<=6) or (x+y==9 and 2<=y<=6))) for y in range(8))
            else: cell=geometry(code)
            cells.append(cell)
        normal=b''.join(cells)
        sets.append(normal+bytes(value^255 for value in normal))
    return b''.join(sets)


def gate_record(entry, target, page, kind):
    return struct.pack('<HHBBH', entry, target, page, kind, 0)


def keywords(text):
    words = text.splitlines()
    if len(words) != 76 or len(set(words)) != 76 or any(
            not 1 <= len(word) <= 7 or any(not 33 <= ord(char) <= 126 for char in word)
            for word in words):
        raise ValueError('Invalid BASIC V2 keyword inventory')
    # Numeric byte operands avoid treating even developer input as assembler code.
    return ''.join('.byte ' + ','.join(str(byte) for byte in bytes([len(word)]) + word.encode('ascii')) + '\n'
                   for word in words)


def container(images, gates, source_hash, builder_hash):
    if len(images) < 3 or len(images) > 18 or any(len(x) != 8192 for x in images[:-1]) or len(images[-1]) != 4096:
        raise ValueError('Invalid original window lengths')
    public = struct.pack('<HBB', 0xA000, 4, 0)
    mapping = b''.join(gates) + public
    metadata = 160 + len(images) * 32 + len(mapping)
    payload = b''.join(images)
    header = bytearray(160)
    header[:8] = b'EBC64R01'
    struct.pack_into('<HBBHH', header, 8, 1, len(images) - 2, len(gates), 1, 0)
    header[16:48] = source_hash
    header[48:80] = builder_hash
    header[80:112] = digest(mapping)
    header[112:144] = digest(payload)
    struct.pack_into('<II', header, 144, metadata, len(payload))
    result = bytes(header) + b''.join(digest(image) for image in images) + mapping + payload
    if metadata > 16384 or len(result) > 159744:
        raise ValueError('Original firmware container limit exceeded')
    return result


def execute(argv, cwd, timeout=30, limit=65536):
    if os.name != 'posix':
        raise ValueError('This build script is qualified for macOS/Linux only')
    process = subprocess.Popen(argv, cwd=cwd, env={'PATH': '/usr/bin:/bin', 'LC_ALL': 'C'},
                               stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                               stderr=subprocess.STDOUT, start_new_session=True)
    output = bytearray()
    deadline = time.monotonic() + timeout
    selector = selectors.DefaultSelector()
    try:
        selector.register(process.stdout, selectors.EVENT_READ)
        while selector.get_map():
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise ValueError('Original firmware tool timed out')
            for key, _ in selector.select(remaining):
                chunk = os.read(key.fd, min(4096, limit + 1 - len(output)))
                if not chunk:
                    selector.unregister(key.fileobj)
                else:
                    output.extend(chunk)
                    if len(output) > limit:
                        raise ValueError('Original firmware tool output limit exceeded')
        try:
            code = process.wait(timeout=max(0.001, deadline - time.monotonic()))
        except subprocess.TimeoutExpired as error:
            raise ValueError('Original firmware tool timed out') from error
        if code:
            raise ValueError('Original firmware tool failed: ' + output[:4000].decode('utf-8', errors='replace'))
        return output.decode('utf-8', errors='replace').strip()
    finally:
        selector.close()
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        process.wait(timeout=5)
        process.stdout.close()


def publish(image, output, check):
    if output.is_symlink():
        raise ValueError('Output must not be a symlink')
    if output.exists():
        fd = os.open(output, os.O_RDONLY | os.O_NONBLOCK | os.O_NOFOLLOW)
        try:
            info = os.fstat(fd)
            if not stat.S_ISREG(info.st_mode) or info.st_size != len(image):
                raise ValueError('Original firmware differs; choose a new output file')
            with os.fdopen(fd, 'rb', closefd=False) as handle:
                if handle.read(len(image) + 1) != image:
                    raise ValueError('Original firmware differs; choose a new output file')
        finally:
            os.close(fd)
    elif check:
        raise ValueError('Original firmware output is missing')
    else:
        output.parent.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(prefix='.c64-original-', dir=output.parent) as folder:
            staged = Path(folder) / 'image.c64rom'
            staged.write_bytes(image)
            # A complete single-file container is published atomically, without
            # replacing a destination created by another operation.
            os.link(staged, output)


def build(ca65, ld65):
    records = []
    inputs = {}
    framed = bytearray()
    for source in INPUTS:
        data = source.read_bytes()
        inputs[source.name] = data
        path = source.relative_to(ROOT).as_posix().encode()
        framed.extend(struct.pack('>I', len(path)) + path + struct.pack('>Q', len(data)) + data)
        records.append({'path': path.decode(), 'sha256': digest(data).hex()})
    source_hash = digest(framed)
    builder_hash = digest(Path(__file__).read_bytes())
    versions = {name: {'version': execute([str(path), '--version'], ROOT), 'sha256': digest(path.read_bytes()).hex()}
                for name, path in [('ca65', ca65), ('ld65', ld65)]}
    with tempfile.TemporaryDirectory(prefix='eightbit-c64-build-') as folder:
        work = Path(folder)
        (work / 'source').mkdir()
        for name, data in inputs.items():
            (work / 'source' / name).write_bytes(data)
        (work / 'font.raw').write_bytes(font(json.loads(inputs['characters.json'])))
        (work / 'source' / 'keywords.inc').write_text(keywords(inputs[KEYWORDS.name].decode('ascii')))
        (work / 'font.s').write_text('.segment "FONT"\n.incbin "font.raw"\n')
        def compile_tool(argv):
            if execute(argv, work):
                raise ValueError('Original firmware build emitted diagnostics; no image published')
        for name in ['kernal', 'pages', 'basic', 'numbers']:
            compile_tool([str(ca65), '-g', '-o', name + '.o', 'source/' + name + '.s'])
        compile_tool([str(ca65), '-o', 'font.o', 'font.s'])
        compile_tool([str(ld65), '-C', 'source/linker.cfg', '-Ln', 'symbols.lbl',
                 '-m', 'layout.map', '-o', 'unused.bin',
                 'kernal.o', 'pages.o', 'basic.o', 'numbers.o', 'font.o'])
        symbols = {}
        for line in (work / 'symbols.lbl').read_text().splitlines():
            fields = line.split()
            if len(fields) == 3 and fields[0] == 'al':
                symbols[fields[2].removeprefix('.')] = int(fields[1], 16)
        gates = [gate_record(symbols[label], symbols[target], page, kind) for label, target, page, kind in [
            ('gate_banner', 'banner_entry', 1, 0), ('gate_irq', 'irq_handler', 0, 1),
            ('gate_nmi', 'nmi_handler', 0, 1), ('gate_recovery', 'main_entry', 0, 2),
            ('gate_math', 'math_entry', 2, 0),
            ('gate_open', 'open_default', 2, 0), ('gate_close', 'close_default', 2, 0),
            ('gate_chkin', 'chkin_default', 2, 0), ('gate_chkout', 'chkout_default', 2, 0)]]
        images = [(work / name).read_bytes() for name in ['basic0.bin', 'basic1.bin', 'basic2.bin', 'kernal.bin', 'characters.bin']]
        image = container(images, gates, source_hash, builder_hash)
        layout = (work / 'layout.map').read_text().split('Segment list:', 1)[1].split('Exports list', 1)[0]
        segments = {}
        for line in layout.splitlines():
            fields = line.split()
            if len(fields) == 5 and fields[0] in {'PAGE0', 'PAGE1', 'PAGE2', 'CODE', 'GATES', 'RODATA', 'EDITOR', 'JUMPS', 'VECTORS', 'FONT'}:
                segments[fields[0]] = {'start': int(fields[1], 16), 'bytes': int(fields[3], 16)}
        manifest = {
            'schemaVersion': 1, 'identity': 'eightbit-c64-original-development-0.5',
            'qualification': 'original guest BASIC V2, editor and KERNAL; named qualification and compatibility limits apply',
            'sourceSha256': source_hash.hex(), 'builderSha256': builder_hash.hex(),
            'imageSha256': digest(image).hex(), 'imageBytes': len(image), 'pageCount': 3,
            'layoutSha256': digest((work / 'layout.map').read_bytes()).hex(),
            'segments': segments,
            'sources': records, 'tools': versions,
            'windows': [{'name': name, 'bytes': len(data), 'sha256': digest(data).hex()} for name, data in zip(
                ['basic0', 'basic1', 'basic2', 'kernal', 'characters'], images, strict=True)],
            'gates': [{'entry': symbols[name], 'target': symbols[target], 'page': page, 'kind': kind} for name, target, page, kind in [
                ('gate_banner', 'banner_entry', 1, 'call'), ('gate_irq', 'irq_handler', 0, 'interrupt'),
                ('gate_nmi', 'nmi_handler', 0, 'interrupt'), ('gate_recovery', 'main_entry', 0, 'recovery'),
                ('gate_math', 'math_entry', 2, 'call'),
                ('gate_open', 'open_default', 2, 'call'), ('gate_close', 'close_default', 2, 'call'),
                ('gate_chkin', 'chkin_default', 2, 'call'), ('gate_chkout', 'chkout_default', 2, 'call')]],
            'publicBasicRanges': [{'address': 40960, 'length': 4}],
            'limits': ['Development runtime; not universal historical software or silicon compatibility',
                       'Decimal literals bounded to 80 mantissa digits; real CPU expression/loop/call stack guarded',
                       'Conservative stack capacity differs from the historical interpreter',
                       'Original transcendental approximations and random sequence; named tolerance corpus',
                       'TI$ assignment requires six digits; undocumented nondigit conversions unsupported',
                       'Independent character drawings preserve symbol roles, not original pixel shapes',
                       'No attached peripheral, cartridge, NTSC or host filesystem',
                       'Studio/package acceptance is recorded separately; no public release or default promotion']}
        return image, manifest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--ca65', type=Path, required=True)
    parser.add_argument('--ld65', type=Path, required=True)
    parser.add_argument('--output', type=Path)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    for tool in [args.ca65, args.ld65]:
        if not tool.is_absolute() or not tool.is_file() or not os.access(tool, os.X_OK):
            parser.error('Select absolute, separately installed ca65 and ld65 executables')
    image, manifest = build(args.ca65.resolve(), args.ld65.resolve())
    output = (args.output or ROOT / '.local/c64-original' / (manifest['imageSha256'] + '.c64rom')).absolute()
    try:
        publish(image, output, args.check)
    except (OSError, ValueError) as error:
        parser.error(str(error))
    print(json.dumps({'schemaVersion': 1, 'reproduced': args.check, 'manifest': manifest}))


if __name__ == '__main__':
    main()
