#!/usr/bin/env python3
"""Build or verify the original 32 KiB Studio BASIC profile (ADR 0057).

The canonical complete-BASIC source and its native packer remain authoritative.
A build creates a new snapshot; it never promotes development qualification.
"""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import shutil
import tempfile

ROOT = Path(__file__).resolve().parent.parent
FIRMWARE = ROOT / 'families/dragon/firmware/basic-extended'
PROFILE = 'eightbit-dragon32-basic-extended-1'
PADDING = 0x12
ARTEFACTS = ('basic.rom', 'basic.sym', 'manifest.json', 'qualification.json')
MEMORY_MAP = {
    'ramStart': 0, 'ramEndExclusive': 0x8000,
    'romStart': 0x8000, 'romEndExclusive': 0xff00,
    'ioStart': 0xff00, 'ioEndExclusive': 0xffe0,
    'vectorAliasStart': 0xffe0, 'vectorAliasEndExclusive': 0x10000,
    'vectorAliasSourceStart': 0xbfe0, 'cartridges': False,
    'samAllRam': 'unchanged',
}
GAP_ENDS = (0x84d6, 0x8b2d, 0x8c37, 0xa000, 0xb3b4, 0xb54a, 0xb93e,
            0xb999, 0xbb40, 0xbbb5, 0xbbbb, 0xbbe5, 0xbd52, 0xbda5,
            0xbdad, 0xbe12, 0xbe6a, 0xbff0)
FIXED_ENTRIES = {
    0x8000: 'HW_INIT', 0x8003: 'SETUP', 0x8006: 'KEYIN', 0x8009: 'BLINK',
    0x800c: 'SCREENOUT', 0x800f: 'PRINTEROUT', 0x8012: 'JOYIN',
    0x8015: 'MOTORON', 0x8018: 'MOTOROFF', 0x801b: 'TAPE_PREP',
    0x801e: 'TAPE_BYTE_OUT', 0x8021: 'TAPE_SYNC', 0x8024: 'TAPE_BYTE_IN',
    0x8027: 'TAPE_BIT_IN', 0x84d6: 'EXECUTE_STATEMENT',
    0x8b2d: 'PUBLIC_FAC_TO_INT', 0x8b30: 'PUBLIC_FAC_WORD',
    0x8c37: 'INT_TO_FAC', 0xb3b4: 'BOOT', 0xb54a: 'CHROUT',
    0xb93e: 'TAPE_BLOCK_IN', 0xb999: 'TAPE_BLOCK_OUT', 0xbb40: 'HW_INIT',
    0xbbb5: 'BLINK', 0xbbbb: 'SETUP', 0xbbe5: 'KEYIN', 0xbd52: 'JOYIN',
    0xbda5: 'TAPE_BIT_IN', 0xbdad: 'TAPE_BYTE_IN', 0xbe12: 'TAPE_BYTE_OUT',
    0xbe6a: 'TAPE_PREP',
}
POINTER_ENTRIES = ('KEYIN', 'CHROUT', 'TAPE_SYNC', 'TAPE_BLOCK_IN',
                   'TAPE_BLOCK_OUT', 'JOYIN', 'TAPE_PREP')
IDENTITY_FIELDS = ('schemaVersion', 'id', 'status', 'licence', 'bytes',
                   'sourceRoot', 'sourceSha256', 'builderSha256',
                   'assemblyDefines', 'memoryMap', 'romSha256', 'symbolsSha256')

_spec = importlib.util.spec_from_file_location(
    'complete_basic_packer', Path(__file__).with_name('qualify-dragon-basic.py'))
packer = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(packer)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def regular_bytes(path, maximum):
    if path.is_symlink() or not path.is_file() or path.stat().st_size > maximum:
        raise ValueError('Expanded BASIC assets must be bounded regular files')
    return path.read_bytes()


def canonical_json(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':'))


def builders():
    return {name: digest(Path(__file__).with_name(name).read_bytes()) for name in
            ('build-dragon-basic-extended.py', 'qualify-dragon-basic.py')}


def source_hashes():
    for directory in (packer.SOURCE, packer.SOURCE / 'source'):
        if directory.is_symlink() or not directory.is_dir():
            raise ValueError('Expanded BASIC source directories must not use symbolic links')
    packer.read_modules()
    return packer.source_hashes()


def symbols_from(data):
    text = data.decode('ascii')
    result = {}
    for line in text.splitlines():
        match = re.fullmatch(r'(\w+) (?:EQU|SET) \$([A-Fa-f0-9]+)', line)
        if not match or match[1] in result:
            raise ValueError('Expanded BASIC symbols must be unique assembler definitions')
        result[match[1]] = int(match[2], 16)
    return result


def footprint(image, symbol_bytes):
    if len(image) != 32768:
        raise ValueError('Expanded BASIC must occupy exactly 32768 bytes')
    syms = symbols_from(symbol_bytes)
    end = syms.get('STUDIO_CODE_END', 0)
    if not 0xc000 <= end <= 0xff00 or syms.get('STUDIO_EXTENDED') != 1:
        raise ValueError('Expanded BASIC code must end before the I/O window')
    if image[end - 0x8000:0x7fe0] != bytes([PADDING]) * (0xffe0 - end):
        raise ValueError('Expanded BASIC unused space must be deterministic NOP padding')
    if image[0x7fe0:] != image[0x3fe0:0x4000]:
        raise ValueError('Expanded BASIC vector alias must match BFE0-BFFF')
    for address, label in FIXED_ENTRIES.items():
        target = syms.get(label, 0)
        if not 0x8000 <= target < end:
            raise ValueError('Expanded BASIC fixed entry target is outside guest code')
        if image[address - 0x8000:address - 0x8000 + 3] != b'\x7e' + target.to_bytes(2, 'big'):
            raise ValueError('Expanded BASIC fixed entry does not match its symbol')
    for index, label in enumerate(POINTER_ENTRIES):
        address = 0x2000 + index * 2
        if image[address:address + 2] != syms[label].to_bytes(2, 'big'):
            raise ValueError('Expanded BASIC published pointer table does not match')
    vectors = (0x0100, 0x0100, 0x0103, 0x010f, 0x010c, 0x0106, 0x0109, 0xb3b4)
    if image[0x3ff0:0x4000] != b''.join(v.to_bytes(2, 'big') for v in vectors):
        raise ValueError('Expanded BASIC interrupt vector table does not match')
    firq = syms.get('CARTRIDGE_FIRQ', 0) - 0x8000
    if image[firq:firq + 8] != bytes.fromhex('3402b6ff2235023b'):
        raise ValueError('Expanded BASIC default cartridge interrupt must return safely')
    gaps = []
    for index, stop in enumerate(GAP_ENDS, 1):
        start = syms.get(f'ROM_GAP_{index}', 0)
        if not 0x8000 <= start <= stop <= 0xc000:
            raise ValueError('Expanded BASIC fixed entry gap is out of bounds')
        if image[start - 0x8000:stop - 0x8000] != bytes([PADDING]) * (stop - start):
            raise ValueError('Expanded BASIC fixed entry gaps must contain NOP padding')
        gaps.append({'start': start, 'bytes': stop - start})
    free = sum(gap['bytes'] for gap in gaps) + 0xff00 - end
    return {'payloadEndExclusive': end, 'payloadBytes': end - 0x8000,
            'usedBytes': 0x7f00 - free, 'freeBytes': free,
            'paddingByte': PADDING, 'containerReservedBytes': 256, 'gaps': gaps}


def identity(receipt):
    return {key: receipt[key] for key in IDENTITY_FIELDS}


def qualification(directory, receipt):
    """A reviewed record may promote only its exact image/source/map/builders."""
    record = json.loads(regular_bytes(directory / 'qualification.json', 65536))
    if not isinstance(record, dict) or record.get('status') not in ('development', 'qualified'):
        raise ValueError('Expanded BASIC qualification status is invalid')
    expected = identity(receipt)
    expected['status'] = record['status']
    # Canonical JSON distinguishes booleans from integers and rejects extra keys.
    if canonical_json(record) != canonical_json(expected):
        raise ValueError('Expanded BASIC qualification does not match this exact build')
    return record


def build(stage, assembler, qualification_directory=None):
    before = source_hashes()
    assembly = stage / 'assembly'
    assembly.mkdir()
    packer.assemble_candidate(assembly, assembler, expanded=True)
    raw = (assembly / 'candidate.bin').read_bytes()
    image = bytearray(raw + bytes([PADDING]) * (32768 - len(raw)))
    image[0x7fe0:] = image[0x3fe0:0x4000]
    symbol_bytes = (assembly / 'candidate.sym').read_bytes()
    if source_hashes() != before:
        raise ValueError('Expanded BASIC source changed while assembling')
    receipt = {
        'schemaVersion': 1, 'id': PROFILE, 'status': 'development',
        'licence': 'Apache-2.0', 'bytes': len(image),
        'sourceRoot': '../basic-complete', 'sourceSha256': before,
        'builderSha256': builders(), 'assemblyDefines': ['STUDIO_EXTENDED=1'],
        'memoryMap': MEMORY_MAP, 'romSha256': digest(image),
        'symbolsSha256': digest(symbol_bytes), 'layout': footprint(image, symbol_bytes),
    }
    if qualification_directory is not None:
        receipt['status'] = qualification(qualification_directory, receipt)['status']
    (stage / 'basic.rom').write_bytes(image)
    (stage / 'basic.sym').write_bytes(symbol_bytes)
    (stage / 'manifest.json').write_text(json.dumps(receipt, indent=2) + '\n')
    (stage / 'qualification.json').write_text(json.dumps(identity(receipt), indent=2) + '\n')
    verify(stage)
    return receipt


def verify(directory):
    receipt = json.loads(regular_bytes(directory / 'manifest.json', 65536))
    if not isinstance(receipt, dict):
        raise ValueError('Expanded BASIC manifest must be an object')
    image = regular_bytes(directory / 'basic.rom', 32768)
    symbol_bytes = regular_bytes(directory / 'basic.sym', 262144)
    expected = {
        'schemaVersion': 1, 'id': PROFILE, 'status': receipt.get('status'),
        'licence': 'Apache-2.0', 'bytes': 32768,
        'sourceRoot': '../basic-complete', 'sourceSha256': source_hashes(),
        'builderSha256': builders(), 'assemblyDefines': ['STUDIO_EXTENDED=1'],
        'memoryMap': MEMORY_MAP, 'romSha256': digest(image),
        'symbolsSha256': digest(symbol_bytes), 'layout': footprint(image, symbol_bytes),
    }
    if canonical_json(receipt) != canonical_json(expected):
        raise ValueError('Expanded BASIC provenance or memory map does not match')
    record = qualification(directory, receipt)
    if receipt['status'] != record['status']:
        raise ValueError('Expanded BASIC manifest and qualification status disagree')
    return receipt


def publish_snapshot(stage, destination):
    """Only a complete, already verified new directory becomes visible."""
    destination.parent.mkdir(parents=True, exist_ok=True)
    if destination.exists() or destination.is_symlink():
        raise ValueError('Expanded BASIC snapshot destination already exists')
    with tempfile.TemporaryDirectory(prefix='.extended-basic-', dir=destination.parent) as temporary:
        prepared = Path(temporary) / 'snapshot'
        prepared.mkdir()
        for name in ARTEFACTS:
            shutil.copy2(stage / name, prepared / name)
        if destination.exists() or destination.is_symlink():
            raise ValueError('Expanded BASIC snapshot destination already exists')
        prepared.rename(destination)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--verify', action='store_true', help='Verify maintained provenance without assembling')
    mode.add_argument('--check', action='store_true', help='Reassemble and compare maintained artefacts')
    mode.add_argument('--output-dir', type=Path, help='Create a new development snapshot directory')
    parser.add_argument('--lwasm', default=shutil.which('lwasm'))
    args = parser.parse_args()
    if args.verify:
        receipt = verify(FIRMWARE)
    else:
        if not args.lwasm:
            parser.error('Select separately installed LWASM with --lwasm')
        assembler = Path(args.lwasm).resolve(strict=True)
        if args.output_dir and (args.output_dir.exists() or args.output_dir.is_symlink()):
            parser.error('Choose a new output directory; snapshots are never overwritten')
        with tempfile.TemporaryDirectory(prefix='eightbit-extended-basic-') as temporary:
            stage = Path(temporary)
            receipt = build(stage, assembler, FIRMWARE if args.check else None)
            if args.check:
                verify(FIRMWARE)
                for name in ARTEFACTS:
                    if (stage / name).read_bytes() != (FIRMWARE / name).read_bytes():
                        raise ValueError('Expanded BASIC artefacts do not reproduce: ' + name)
            else:
                publish_snapshot(stage, args.output_dir)
    print(json.dumps({key: receipt[key] for key in
                     ('id', 'status', 'bytes', 'romSha256', 'layout')}, indent=2))


if __name__ == '__main__':
    main()
