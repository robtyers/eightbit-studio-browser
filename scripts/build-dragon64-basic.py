#!/usr/bin/env python3
"""Reproduce original Dragon 64 Studio firmware, without changing D32 artefacts.

Only original project assembly is consumed. LWASM is a separately installed build
instrument; no assembler, external firmware or host interpreter is distributed.
"""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
DESTINATION = ROOT / 'families/dragon/firmware/basic64'
SOURCE = ROOT / 'families/dragon/firmware/basic-complete'
PAGE_START = 0xd000
PAGE_BYTES = 4096
MAX_PAGES = 128
COMMON = {
    'console.inc', 'commands.inc', 'control.inc', 'fp-program.inc', 'keywords.inc',
    'number.inc', 'scanner.inc', 'native-frame.inc', 'common-data.inc',
    'exec.inc', 'break.inc', 'errors.inc', 'function-call.inc', 'function-return.inc',
    'value-functions.inc', 'byte-templates.inc', 'tape-read.inc', 'tape-write.inc',
    'tape-block-in.inc', 'tape-block-out.inc', 'native-transition.inc', 'peripherals.inc',
}
ARTEFACTS = ('compatibility.rom', 'native.rom', 'native.sym', 'native-controls.rs', 'manifest.json')
_spec = importlib.util.spec_from_file_location('dragon32_extended', ROOT / 'scripts/build-dragon-basic-extended.py')
shared = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(shared)
D64_FIXED_ENTRIES = {**shared.FIXED_ENTRIES, 0x802a: 'D64_SERIN', 0x802d: 'D64_SEROUT', 0x8030: 'D64_SERSET'}


def digest(data):
    return hashlib.sha256(data).hexdigest()


def assemble(directory, name, assembler):
    result = subprocess.run([
        str(assembler), '--format=raw', '--define=STUDIO_EXTENDED=1',
        f'--includedir={directory}', f'--output={directory}/{name}.bin',
        f'--symbol-dump={directory}/{name}.sym', str(directory / (name + '.asm')),
    ], cwd=directory, env={'PATH': '/usr/bin:/bin', 'LANG': 'C'},
        capture_output=True, text=True, timeout=30)
    if result.returncode:
        raise ValueError(result.stderr)
    return shared.symbols_from((directory / (name + '.sym')).read_bytes())


def peripheral_source():
    if (DESTINATION / 'source').is_symlink():
        raise ValueError('Dragon 64 source directory must not use symbolic links')
    return shared.regular_bytes(DESTINATION / 'source/peripherals.inc', 16384).decode('ascii')


def source_hashes():
    return {**shared.source_hashes(), 'dragon64/peripherals.inc': digest(peripheral_source().encode())}


def d64_abi(source):
    # D64 owns 03FD/03FE for serial line delay. Our diagnostic byte moves to
    # unused interpreter scratch, after the bounded keyboard/token input area.
    return source.replace('ERROR_CODE equ   $03fd', 'ERROR_CODE equ   $03d8')


def d64_console(source, native):
    source = source.replace('\nKEY_SAVE\n', '\nKEY_SAVE\n        jsr D64_KEY_EVENT\n', 1)
    source = source.replace('        ldd     #FRAME_IRQ', '        ldd     #D64_REPEAT_IRQ' if native else '        ldd     #FRAME_IRQ')
    source = source.replace('        ldx     #$0134\nUSR_TABLE_INIT', '        lda #5\n        sta $011f\n        lda #$0b\n        sta $ff06\n        lda #$98\n        sta $ff07\n        ldx     #$0134\nUSR_TABLE_INIT')
    source = source.replace('\nPRINT_BYTE\n', '\nPRINT_BYTE\n        tst $03ff\n        lbeq D64_PRINT_PARALLEL\n        jmp D64_SERIAL_BYTE\nD64_PRINT_PARALLEL\n', 1)
    source = source.replace('\nPRINT_READY\n', '\nPRINT_READY\n        tst $03ff\n        lbeq D64_READY_PARALLEL\n        ldb $ff05\n        bitb #16\n        lbeq PRINT_READY\n        rts\nD64_READY_PARALLEL\n', 1)
    return source


def d64_dload(source):
    return source.replace('        jsr     EVAL_NUMBER\n        jmp     NATIVE_RANGE_ERROR',
        '        jsr EVAL_ADDRESS\n        tsta\n        lbne NATIVE_RANGE_ERROR\n        jsr D64_SERSET\n        lbcs NATIVE_RANGE_ERROR\n        lbra DLOAD_END')


def modules(relaxed):
    _, originals, names = shared.packer.read_modules()
    originals.update(relaxed)
    originals.pop('abi.inc')
    sources = {}
    for name in names:
        text = originals[name].replace('jsr     $bbe5', 'jsr     $fbe5')
        if name == 'console.inc':
            text = d64_console(text, native=True)
        if name == 'dload.inc':
            text = d64_dload(text)
        # Native user RAM reaches BFFF. Numeric signed-word checks are unrelated.
        if name == 'console.inc':
            text = text.replace('#$7fff', '#$bfff').replace('#$7ffe', '#$bffe')
            text = text.replace('"32K RAM - EXPANDED ROM"', '"DRAGON 64 - NATIVE BASIC"')
            text = text.replace('DIRECT\n', 'DIRECT\n        sta     $ff42\n', 1)
        if name == 'tape-block-in.inc':
            text = text.replace('cmpd    #$8000', 'cmpd    #$c000')
        if name == 'commands.inc':
            text = text.replace('        tsta\n        lbmi    NATIVE_MEMORY_ERROR\n        subd    #1', '        cmpd    #$c000\n        lbhs    NATIVE_MEMORY_ERROR\n        subd    #1')
        if name == 'expression.inc':
            text = text.replace('EX_MEM\n        ldd     FRETOP\n        subd    STREND\n        jmp     INT_TO_FAC', 'EX_MEM\n        ldd     FRETOP\n        subd    STREND\n        jmp     UNSIGNED_TO_FAC')
        sources[name] = text
    data = 'ERROR_IN\n        fcc     " IN "\n        fcb     0\n'
    if sources['complete-cassette.inc'].count(data) != 1:
        raise ValueError('Native diagnostic data boundary changed')
    sources['complete-cassette.inc'] = sources['complete-cassette.inc'].replace(data, '')
    tail = 'NATIVE_TAIL_1\n        puls    x\n        clra\n        rts\n'
    if sources['expression.inc'].count(tail) != 1:
        raise ValueError('Native stack-return boundary changed')
    sources['expression.inc'] = sources['expression.inc'].replace(tail, '        jmp NATIVE_TAIL_1\n')
    sources['common-data.inc'] = data + tail
    sources['peripherals.inc'] = peripheral_source()
    sources['native-transition.inc'] = '''NATIVE_TRANSITION
        orcc #$50
        clra
        tfr a,dp
        lds #$bfff
        ldd TXTTAB
        pshs d
        ldd VARTAB
        pshs d
        ldd $1e00
        pshs d
        lda $1e02
        pshs a
        jsr SETUP
        puls a
        sta $1e02
        puls d
        std $1e00
        puls d
        std VARTAB
        std ARYTAB
        std STREND
        puls d
        std TXTTAB
        lds STACKTOP
        leas 1,s
        jsr CLS
        ldx #BANNER
        jsr PUTS
        jmp WARM
'''
    return sources


def native(stage, assembler, relaxed):
    sources = modules(relaxed)
    owners = {m[1]: name for name, text in sources.items()
              for m in re.finditer(r'^(\w+)\s*$', text, re.M)}
    aliases = {m[1]: m[2] for text in sources.values()
               for m in re.finditer(r'^(\w+)\s+equ\s+(\w+)\s*$', text, re.M)}
    canonical = {}
    for alias, target in aliases.items():
        seen = {alias}
        while target in aliases:
            if target in seen:
                raise ValueError('Cyclic firmware alias')
            seen.add(target)
            target = aliases[target]
        canonical[alias] = target
        if target in owners:
            owners[alias] = owners[target]
    groups = {}
    for name in sorted(set(sources) - COMMON):
        key = 'array-expression' if name in ('arrays.inc', 'expression.inc') else name
        groups.setdefault(key, []).append(name)
    pages = {name: i for i, members in enumerate(groups.values(), 1) for name in members}
    code_labels = set()
    for text in sources.values():
        pending = []
        for line in text.splitlines():
            code = line.split(';')[0].strip()
            if not code:
                continue
            if re.fullmatch(r'\w+', code) and not line[0].isspace():
                pending.append(code)
                continue
            operation = code.split()[0].lower()
            if operation in ('ifdef', 'ifndef', 'else', 'endc'):
                continue
            if operation not in ('fcb', 'fdb', 'fcc', 'fill', 'rmb') and line[0].isspace():
                code_labels.update(pending)
            pending.clear()

    if len(groups) > MAX_PAGES:
        raise ValueError('Private firmware page limit exceeded')
    # Every reference crossing into a private module goes through a common gate.
    # Local code remains local; address-taking uses the same canonical gate even
    # inside its owning module, so callbacks and pointer comparisons agree.
    gates = set()
    transformed = {}
    for name, text in sources.items():
        lines = []
        for line in text.splitlines():
            code, separator, comment = line.partition(';')
            definition = re.match(r'^(\w+)', code)
            def replace(match):
                word = match[0]
                if definition and word == definition[1]:
                    return word
                word = canonical.get(word, word)
                owner = owners.get(word)
                if not owner or owner in COMMON or (definition and word == definition[1]):
                    return word
                if word not in code_labels:
                    if pages.get(owner) != pages.get(name):
                        raise ValueError('Private data crosses a firmware page: ' + word)
                    return word
                if pages.get(owner) != pages.get(name) or re.match(r'\s*(?:fdb|ld\w+\s+#)', code):
                    gates.add(word)
                    return 'GATE_' + word
                return word
            code = re.sub(r'\b[A-Z][A-Z_0-9]+\b', replace, code)
            if 'GATE_' in code:
                code = re.sub(r'^(\s+)(b(?:ra|sr|eq|ne|cc|cs|hi|ls|lo|hs|pl|mi|gt|ge|le|lt|vc|vs))(\s+)', r'\1l\2\3', code)
            lines.append(code + (separator + comment if separator else ''))
        transformed[name] = '\n'.join(lines) + '\n'
    # Published native entries are the relocated public D32 interfaces, plus RUN.
    entries = {address + 0x4000: label for address, label in D64_FIXED_ENTRIES.items()}
    entries[0xc49f] = 'CMD_RUN'
    for label in [*entries.values(), *shared.POINTER_ENTRIES]:
        if owners[label] not in COMMON:
            gates.add(label)
    units = {name: text for name, text in transformed.items() if name in COMMON}
    for label in sorted(gates):
        page = pages[owners[label]]
        units['gate-' + label] = f'''GATE_{label}
        pshs    cc,a
        lda     #{page}
PAGE_PUSH_{label}
        sta     $ff40
        puls    cc,a
        jsr     {label}
        pshs    cc
PAGE_POP_{label}
        sta     $ff41
        puls    cc
        rts
'''
    abi = d64_abi((SOURCE / 'source/abi.inc').read_text())
    (stage / 'abi.inc').write_text(abi)
    # Unresolved external names are extended addresses during measurement, so
    # LWASM cannot shorten an operand to direct-page addressing by accident.
    constants = {m[1]: m[2] for text in sources.values() for m in re.finditer(r'^(\w+)\s+equ\s+([^;\n]+)', text, re.M) if m[1] not in owners}
    constants['STATEMENT'] = '$c4d6'
    all_labels = set(owners) | {'GATE_' + label for label in gates}
    def externals(text, symbols):
        defined = set(re.findall(r'^(\w+)', text, re.M))
        return ''.join(f'{label} equ {value.strip()}\n' for label, value in constants.items() if label not in defined) + ''.join(f'{label} equ ${symbols.get(label, 0xe800):04x}\n'
                       for label in sorted(all_labels - defined))
    sizes = {}
    measured = {}
    for index, (name, text) in enumerate(units.items()):
        source = '        pragma 6809\n        include "abi.inc"\n'
        source += externals(text, {}) + '        org $e000\n' + text + 'UNIT_END\n'
        filename = 'measure-' + str(index)
        (stage / (filename + '.asm')).write_text(source)
        measured[name] = assemble(stage, filename, assembler)
        sizes[name] = measured[name]['UNIT_END'] - 0xe000
    # Common islands exclude both the page aperture and the hardware I/O window.
    spans = [(0xc033, 0xc49f), (0xc4a2, 0xc4d6), (0xc4d9, 0xcb2d),
             (0xcb33, 0xcc37), (0xcc3a, 0xd000), (0xe00e, 0xf3b4),
             (0xf3b7, 0xf54a), (0xf54d, 0xf93e), (0xf941, 0xf999),
             (0xf99c, 0xfb40), (0xfb43, 0xfbb5), (0xfbb8, 0xfbbb),
             (0xfbbe, 0xfbe5), (0xfbe8, 0xfd52), (0xfd55, 0xfda5),
             (0xfda8, 0xfdad), (0xfdb0, 0xfe12), (0xfe15, 0xfe6a),
             (0xfe6d, 0xff00)]
    placed = {}
    cursors = [start for start, _ in spans]
    for name in sorted(units, key=lambda name: (-sizes[name], name)):
        candidates = [i for i, (_, end) in enumerate(spans) if sizes[name] <= end - cursors[i]]
        if not candidates:
            raise ValueError(f'Common firmware does not fit: {name}, {sizes[name]} bytes; free {sum(end-cursors[i] for i, (_,end) in enumerate(spans))}')
        slot = min(candidates, key=lambda i: spans[i][1] - cursors[i] - sizes[name])
        placed[name] = cursors[slot]
        cursors[slot] += sizes[name]
    symbols = {}
    for name, at in placed.items():
        for label, address in measured[name].items():
            if 0xe000 <= address < 0xe000 + sizes[name]:
                symbols[label] = at + address - 0xe000
    for page, (name, members) in enumerate(groups.items(), 1):
        text = ''.join(transformed[member] for member in members)
        source = '        pragma 6809\n        include "abi.inc"\n'
        source += externals(text, symbols) + '        org $d000\n' + text + 'PAGE_END\n'
        filename = f'page-{page}'
        (stage / (filename + '.asm')).write_text(source)
        values = assemble(stage, filename, assembler)
        if not PAGE_START <= values['PAGE_END'] <= PAGE_START + PAGE_BYTES:
            raise ValueError('Private module exceeds its page: ' + name)
        for label in re.findall(r'^(\w+)', text, re.M):
            if label in values:
                symbols[label] = values[label]
    def address(label):
        return 'GATE_' + label if label in gates else label
    common = '        pragma 6809\n        include "abi.inc"\n'
    common_labels = set().union(*(set(re.findall(r'^(\w+)', text, re.M)) for text in units.values()))
    common += ''.join(f'{label} equ {value.strip()}\n' for label, value in constants.items() if label not in common_labels)
    common += ''.join(f'{label} equ ${symbols[label]:04x}\n' for label in sorted(all_labels - common_labels))
    pieces = {at: f'        org ${at:04x}\n' + units[name] for name, at in placed.items()}
    pieces.update({at: f'        org ${at:04x}\n        jmp {address(label)}\n' for at, label in entries.items()})
    pieces[0xe000] = '        org $e000\n        fdb ' + ','.join(address(label) for label in shared.POINTER_ENTRIES) + '\n'
    pieces[0xffe0] = '        org $ffe0\n        sta $ff43\n        jmp NATIVE_TRANSITION\n'
    pieces[0xfff0] = '        org $fff0\n        fdb $0100,$0100,$0103,$010f,$010c,$0106,$0109,$f3b4\n'
    common += '        org $c000\n' + ''.join(re.sub(r'        org \$[0-9a-f]+\n', f'        fill $12,${at:04x}-*\n', pieces[at], count=1) for at in sorted(pieces))
    (stage / 'common.asm').write_text(common)
    final = assemble(stage, 'common', assembler)
    # Reassemble private pages with final common symbols, including aliases.
    image = bytearray((stage / 'common.bin').read_bytes())
    if len(image) != 16384:
        raise ValueError('Common image must span C000-FFFF')
    for page, (name, members) in enumerate(groups.items(), 1):
        text = ''.join(transformed[member] for member in members)
        source = '        pragma 6809\n        include "abi.inc"\n'
        source += externals(text, final) + '        org $d000\n' + text + 'PAGE_END\n'
        (stage / f'page-{page}.asm').write_text(source)
        assemble(stage, f'page-{page}', assembler)
        data = (stage / f'page-{page}.bin').read_bytes()
        image.extend(data + bytes([0x12]) * (PAGE_BYTES - len(data)))
    # Every common entry and private strobe is bound to the assembled symbols.
    controls = {str(final[label]): 0xff40 if label.startswith('PAGE_PUSH_') else 0xff41
                for label in final if label.startswith(('PAGE_PUSH_', 'PAGE_POP_'))}
    controls[str(final['DIRECT'])] = 0xff42
    sym = '\n'.join(f'{label} EQU ${value:04X}' for label, value in sorted(final.items())) + '\n'
    return bytes(image), sym.encode(), {'pages': len(groups), 'pageStart': PAGE_START,
            'pageBytes': PAGE_BYTES, 'commonBytes': 16384, 'controls': controls,
            'entries': {str(at): final[address(label)] for at, label in entries.items()},
            'commonUsedBytes': sum(sizes.values())}


def compatibility(stage, assembler, relaxed):
    _, originals, names = shared.packer.read_modules()
    originals.update(relaxed)
    originals['abi.inc'] = d64_abi(originals['abi.inc'])
    originals['peripherals.inc'] = peripheral_source()
    names.append('peripherals.inc')
    pieces = {}
    measure = '        pragma 6809\n        include "abi.inc"\nSTATEMENT equ $84d6\n        org $8000\n'
    (stage / 'abi.inc').write_text(originals['abi.inc'])
    for i, name in enumerate(names):
        text = originals[name].replace('"32K RAM - EXPANDED ROM"', '"DRAGON 64 - 32K BASIC"')
        if name == 'console.inc':
            text = d64_console(text, native=False).replace('ldd     #EXEC_ERROR', 'ldd     #$bb80')
        if name == 'dload.inc':
            text = d64_dload(text)
        pieces[name] = text
        measure += f'M{i}_START\n' + text + f'M{i}_END\n'
    (stage / 'measure.asm').write_text(measure)
    symbols = assemble(stage, 'measure', assembler)
    sizes = {name: symbols[f'M{i}_END'] - symbols[f'M{i}_START'] for i, name in enumerate(names)}
    ends = list(shared.GAP_ENDS)
    starts = [0x8033, 0x84d9, 0x8b33, 0x8c3a, 0xa00e, 0xb3b7, 0xb54d, 0xb941, 0xb99c,
              0xbb43, 0xbbb8, 0xbbbe, 0xbbe8, 0xbd55, 0xbda8, 0xbdb0, 0xbe15, 0xbe6d]
    spans = list(zip(starts, ends))
    spans.remove((0xbb43, 0xbbb5))
    spans.extend([(0xbb43, 0xbb80), (0xbb83, 0xbbb5), (0xc000, 0xff00)])
    spans = [(start, min(end, 0xbf20) if start == 0xbe6d else end) for start, end in spans]
    spans.append((0xbf23, 0xbfe0))
    cursors = [start for start, _ in spans]
    placed = {}
    for name in sorted(names, key=lambda name: (-sizes[name], name)):
        slots = [i for i, (_, end) in enumerate(spans) if sizes[name] <= end - cursors[i]]
        if not slots:
            raise ValueError('Compatibility firmware does not fit')
        slot = min(slots, key=lambda i: spans[i][1] - cursors[i] - sizes[name])
        placed[name] = cursors[slot]
        cursors[slot] += sizes[name]
    blocks = {at: pieces[name] for name, at in placed.items() if sizes[name]}
    # Empty alias modules still define symbols and must participate once.
    aliases = ''.join(pieces[name] for name in placed if sizes[name] == 0)
    blocks.update({at: '        jmp ' + label + '\n' for at, label in D64_FIXED_ENTRIES.items()})
    blocks[0xa000] = '        fdb ' + ','.join(shared.POINTER_ENTRIES) + '\n'
    blocks[0xbf20] = '        jmp D64_REPEAT_IRQ\n'
    blocks[0xbb80] = '        jmp $ffe0\n'
    blocks[0xbfe0] = '        sta $ff43\n        jmp $f3b4\n'
    blocks[0xbff0] = '        fdb $0100,$0100,$0103,$010f,$010c,$0106,$0109,$b3b4\n'
    assembly = '        pragma 6809\n        include "abi.inc"\nSTATEMENT equ $84d6\n' + aliases + '        org $8000\n'
    assembly += ''.join(f'        fill $12,${at:04x}-*\n' + blocks[at] for at in sorted(blocks))
    assembly += '        fill $12,$10000-*\n'
    (stage / 'compatibility.asm').write_text(assembly)
    assemble(stage, 'compatibility', assembler)
    image = bytearray((stage / 'compatibility.bin').read_bytes())
    if len(image) != 32768:
        raise ValueError('Compatibility firmware must span 32 KiB')
    image[0x7fe0:] = image[0x3fe0:0x4000]
    return bytes(image)


def builders():
    return {name: digest((ROOT / 'scripts' / name).read_bytes()) for name in
            ('build-dragon64-basic.py', 'build-dragon-basic-extended.py', 'qualify-dragon-basic.py')}


def control_source(controls):
    return ('pub(crate) const D64_CONTROLS: &[(u16, u16)] = &[\n' +
            ''.join(f'    (0x{int(pc):04x}, 0x{port:04x}),\n'
                    for pc, port in sorted(controls.items(), key=lambda pair: int(pair[0]))) + '];\n')


def build(stage, assembler):
    before = source_hashes()
    relaxation = stage / 'relaxation'
    relaxation.mkdir()
    shared.packer.assemble_candidate(relaxation, assembler, expanded=True)
    relaxed = {p.name: p.read_text() for p in relaxation.glob('*.inc')}
    native_stage = stage / 'native'
    native_stage.mkdir()
    image, symbols, layout = native(native_stage, assembler, relaxed)
    # Compatibility uses the identical original engine and established map.
    compatibility_stage = stage / 'compatibility'
    compatibility_stage.mkdir()
    compatibility_image = compatibility(compatibility_stage, assembler, relaxed)
    (stage / 'compatibility.rom').write_bytes(compatibility_image)
    (stage / 'native.rom').write_bytes(image)
    (stage / 'native.sym').write_bytes(symbols)
    controls = control_source(layout['controls']).encode()
    (stage / 'native-controls.rs').write_bytes(controls)
    if source_hashes() != before:
        raise ValueError('Firmware source changed during build')
    manifest = {'schemaVersion': 1, 'status': 'development', 'licence': 'Apache-2.0',
                'sourceSha256': before, 'builderSha256': builders(),
                'profiles': {
                    'compatibility': {'id': 'eightbit-dragon64-basic-32k-dev-1', 'ramBytes': 65536,
                        'programEndExclusive': 32768, 'bytes': len(compatibility_image), 'sha256': digest(compatibility_image),
                        'nativeTransition': 0xbb80},
                    'native': {'id': 'eightbit-dragon64-basic-64k-dev-1', 'ramBytes': 65536,
                        'programEndExclusive': 49152, 'bytes': len(image), 'sha256': digest(image),
                        'symbolsSha256': digest(symbols), 'controlsSha256': digest(controls), **layout}}}
    (stage / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    return manifest


def verify(directory):
    manifest = json.loads(shared.regular_bytes(directory / 'manifest.json', 262144))
    if (manifest['schemaVersion'] != 1 or manifest['status'] != 'development'
            or manifest['licence'] != 'Apache-2.0'
            or manifest['builderSha256'] != builders()
            or manifest['sourceSha256'] != source_hashes()):
        raise ValueError('Dragon 64 source or builder provenance does not match')
    for mode, name in [('compatibility', 'compatibility.rom'), ('native', 'native.rom')]:
        profile = manifest['profiles'][mode]
        image = shared.regular_bytes(directory / name, 16384 + MAX_PAGES * PAGE_BYTES)
        if len(image) != profile['bytes'] or digest(image) != profile['sha256']:
            raise ValueError('Dragon 64 image identity does not match')
    native_profile = manifest['profiles']['native']
    if (not 1 <= native_profile['pages'] <= MAX_PAGES
            or native_profile['bytes'] != 16384 + PAGE_BYTES * native_profile['pages']
            or native_profile['symbolsSha256'] != digest(shared.regular_bytes(directory / 'native.sym', 262144))):
        raise ValueError('Dragon 64 native layout or symbols do not match')
    if set(manifest) != {'schemaVersion', 'status', 'licence', 'sourceSha256', 'builderSha256', 'profiles'} or type(manifest['schemaVersion']) is not int:
        raise ValueError('Dragon 64 manifest schema does not match')
    if set(manifest['profiles']) != {'compatibility', 'native'}:
        raise ValueError('Dragon 64 profiles do not match')
    compatibility_profile = manifest['profiles']['compatibility']
    if shared.canonical_json(compatibility_profile) != shared.canonical_json({
            'id': 'eightbit-dragon64-basic-32k-dev-1', 'ramBytes': 65536,
            'programEndExclusive': 32768, 'bytes': 32768,
            'sha256': digest((directory / 'compatibility.rom').read_bytes()), 'nativeTransition': 0xbb80}):
        raise ValueError('Dragon 64 compatibility map does not match')
    expected_keys = {'id', 'ramBytes', 'programEndExclusive', 'bytes', 'sha256',
                     'symbolsSha256', 'controlsSha256', 'pages', 'pageStart', 'pageBytes',
                     'commonBytes', 'controls', 'entries', 'commonUsedBytes'}
    if (set(native_profile) != expected_keys or native_profile['id'] != 'eightbit-dragon64-basic-64k-dev-1'
            or native_profile['ramBytes'] != 65536 or native_profile['programEndExclusive'] != 49152
            or native_profile['pageStart'] != PAGE_START or native_profile['pageBytes'] != PAGE_BYTES
            or native_profile['commonBytes'] != 16384
            or any(type(native_profile[key]) is not int for key in ('ramBytes', 'programEndExclusive', 'bytes', 'pages', 'pageStart', 'pageBytes', 'commonBytes', 'commonUsedBytes'))
            or not 0 < native_profile['commonUsedBytes'] <= 0x2f00):
        raise ValueError('Dragon 64 native map does not match')
    symbol_bytes = shared.regular_bytes(directory / 'native.sym', 262144)
    symbols = shared.symbols_from(symbol_bytes)
    controls = {str(address): 0xff40 if label.startswith('PAGE_PUSH_') else 0xff41
                for label, address in symbols.items() if label.startswith(('PAGE_PUSH_', 'PAGE_POP_'))}
    controls[str(symbols['DIRECT'])] = 0xff42
    if shared.canonical_json(native_profile['controls']) != shared.canonical_json(controls):
        raise ValueError('Dragon 64 page controls do not match symbols')
    control_bytes = shared.regular_bytes(directory / 'native-controls.rs', 65536)
    if control_bytes != control_source(controls).encode() or native_profile['controlsSha256'] != digest(control_bytes):
        raise ValueError('Dragon 64 page controls do not match their source')
    image = (directory / 'native.rom').read_bytes()
    for pc, port in controls.items():
        pc = int(pc)
        if not (0xc000 <= pc < 0xd000 or 0xe000 <= pc < 0xff00) or image[pc - 0xc000:pc - 0xc000 + 3] != bytes([0xb7, port >> 8, port & 255]):
            raise ValueError('Dragon 64 page control is not an immutable common STA')
    entries = native_profile['entries']
    if set(entries) != {str(address + 0x4000) for address in D64_FIXED_ENTRIES} | {str(0xc49f)}:
        raise ValueError('Dragon 64 public entry set does not match')
    for at, target in entries.items():
        at = int(at)
        if type(target) is not int or not 0xc000 <= target < 0xff00 or image[at - 0xc000:at - 0xc000 + 3] != b'\x7e' + target.to_bytes(2, 'big'):
            raise ValueError('Dragon 64 public entry bytes do not match')
    return manifest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument('--output-dir', type=Path)
    group.add_argument('--verify', action='store_true')
    group.add_argument('--check', action='store_true')
    parser.add_argument('--lwasm', default=shutil.which('lwasm'))
    args = parser.parse_args()
    if args.verify:
        manifest = verify(DESTINATION)
    else:
        if not args.lwasm:
            parser.error('Select separately installed LWASM')
        destination = args.output_dir
        if destination and (destination.exists() or destination.is_symlink()):
            parser.error('Choose a new snapshot directory')
        with tempfile.TemporaryDirectory(prefix='eightbit-dragon64-') as temporary:
            stage = Path(temporary)
            manifest = build(stage, Path(args.lwasm).resolve(strict=True))
            if args.check:
                verify(DESTINATION)
                for name in ARTEFACTS:
                    if (stage / name).read_bytes() != (DESTINATION / name).read_bytes():
                        raise ValueError('Dragon 64 build does not reproduce: ' + name)
            else:
                destination.parent.mkdir(parents=True, exist_ok=True)
                with tempfile.TemporaryDirectory(prefix='.dragon64-', dir=destination.parent) as temporary_publish:
                    prepared = Path(temporary_publish) / 'snapshot'
                    prepared.mkdir()
                    for name in ARTEFACTS:
                        shutil.copyfile(stage / name, prepared / name)
                    verify(prepared)
                    if destination.exists() or destination.is_symlink():
                        raise ValueError('Snapshot destination already exists')
                    prepared.rename(destination)

    print(json.dumps({mode: {key: p[key] for key in ('id', 'bytes', 'sha256')}
                      for mode, p in manifest['profiles'].items()}, indent=2))


if __name__ == '__main__':
    main()
