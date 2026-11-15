#!/usr/bin/env python3
"""Reproduce the complete original BASIC test candidate; never promote firmware.

The default candidate is confined to the dedicated test bus. The explicit
expanded build option is consumed only by build-dragon-basic-extended.py, which
supplies the separate ADR 0057 profile, map, container and qualification record.
"""
import argparse
import hashlib
import json
from pathlib import Path
import random
import re
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
SOURCE = ROOT / 'families/dragon/firmware/basic-complete'
QUALIFICATION = SOURCE / 'qualification'
ARTEFACTS = ('candidate.bin', 'first16.bin', 'candidate.sym', 'measurement.json')


def digest(data):
    return hashlib.sha256(data).hexdigest()


def source_hashes():
    paths = [SOURCE/'fixed-layout.asm', SOURCE/'layout.asm', *sorted((SOURCE/'source').glob('*.inc'))]
    if any(path.is_symlink() for path in paths):
        raise ValueError('Candidate source must not use symbolic links')
    return {str(path.relative_to(SOURCE)): digest(path.read_bytes()) for path in paths}


def read_modules():
    layout = (SOURCE/'layout.asm').read_text()
    names = re.findall(r'include "([^\"]+)"', layout)
    modules = {p.name: p.read_text() for p in (SOURCE/'source').glob('*.inc')}
    if len(names) != len(set(names)) or set(names) != set(modules) or 'abi.inc' not in names:
        raise ValueError('Candidate layout must include each source module exactly once')
    if any('K_RELAX' in source for source in modules.values()):
        raise ValueError('This packer handles native modules only')
    names.remove('abi.inc')
    return layout, modules, names


def assemble_candidate(stage, assembler, *, expanded=False):
    layout, modules, names = read_modules()
    # Selectively relax within each module so packing cannot invalidate a branch.
    owners = {m[1]: name for name in names for m in re.finditer(
        r'^(\w+)$', modules[name], re.M
    )}
    aliases = {m[1]: m[2] for name in names for m in re.finditer(
        r'^(\w+)\s+equ\s+(\w+)\s*$', modules[name], re.M
    )}
    for alias in aliases:
        target = alias
        seen = set()
        while target in aliases and target not in seen:
            seen.add(target)
            target = aliases[target]
        if target in owners:
            owners[alias] = owners[target]
    long_branch = re.compile(r'(?m)^(\s+)(b(?:ra|sr|eq|ne|cc|cs|hi|ls|lo|hs|pl|mi|gt|ge|le|lt|vc|vs))(\s+)')
    for name in ['abi.inc', *names]:
        (stage / name).write_text(long_branch.sub(r'\1l\2\3', modules[name]))
    measure = layout
    for i, name in enumerate(names):
        measure = measure.replace(f'        include "{name}"',
            f'MODULE_{i}_START\n        include "{name}"\nMODULE_{i}_END')
    (stage / 'measure.asm').write_text(measure)

    def assemble(name):
        result = subprocess.run([
            str(assembler), '--format=raw', f'--includedir={stage}',
            f'--output={stage}/candidate.bin', f'--symbol-dump={stage}/candidate.sym',
            f'--list={stage}/candidate.lst',
            *(['--define=STUDIO_EXTENDED=1'] if expanded else []), str(stage / name),
        ], capture_output=True, text=True, timeout=30, cwd=stage,
            env={'PATH': '/usr/bin:/bin', 'LANG': 'C'})
        if result.returncode:
            raise SystemExit(result.stderr)
        return {k: int(v, 16) for k, v in re.findall(
            r'^(\w+) EQU \$(\w+)$', (stage / 'candidate.sym').read_text(), re.M
        )}

    for attempt in range(32):
        syms = assemble('measure.asm')
        edits = {}
        for line in (stage / 'candidate.lst').read_text().splitlines():
            forms = r'lb\w+|jsr|jmp'
            m = re.match(r'^([A-F0-9]{4}) ([A-F0-9]+)\s+\(([^)]+)\):(\d+)\s+(' + forms + r')\s+(\w+)\s*$', line)
            if not m or m[6] not in syms:
                continue
            # The listing truncates long paths on the left; match the suffix.
            filename = m[3].strip().split('/')[-1]
            matches = [name for name in names if name.endswith(filename)]
            name = filename if filename in names else matches[0] if len(matches) == 1 else None
            if name is None or owners.get(m[6]) != name:
                continue
            if m[5] in ('jsr', 'jmp') and (name.startswith('tape-') or name in (
                    'sound.inc', 'play.inc', 'devices.inc', 'console.inc', 'scanner.inc',
                    'complete-cassette.inc')):
                # Signal paths include framing overhead, not just their inner loops.
                # The scanner also copies absolute-target instructions into RAM.
                continue
            if -125 <= syms[m[6]] - int(m[1], 16) - len(m[2]) // 2 <= 125:
                lines = edits.setdefault(name, (stage / name).read_text().splitlines())
                i = int(m[4]) - 1
                replacement = {'jsr': 'bsr', 'jmp': 'bra'}.get(m[5], m[5][1:])
                lines[i] = lines[i].replace(m[5], replacement, 1)
        if not edits:
            break
        for name, lines in edits.items():
            (stage / name).write_text('\n'.join(lines) + '\n')
    else:
        raise SystemExit('Module branch relaxation did not converge')

    sizes = {name: syms[f'MODULE_{i}_END'] - syms[f'MODULE_{i}_START'] for i, name in enumerate(names)}
    ends = [0x84d6, 0x8b2d, 0x8c37, 0xa000, 0xb3b4, 0xb54a, 0xb93e, 0xb999,
            0xbb40, 0xbbb5, 0xbbbb, 0xbbe5, 0xbd52, 0xbda5, 0xbdad, 0xbe12, 0xbe6a, 0xbff0]
    starts = [0x802a, 0x84d9, 0x8b33, 0x8c3a, 0xa00e, 0xb3b7, 0xb54d, 0xb941, 0xb99c,
              0xbb43, 0xbbb8, 0xbbbe, 0xbbe8, 0xbd55, 0xbda8, 0xbdb0, 0xbe15, 0xbe6d]
    caps = [b - a for a, b in zip(starts, ends)]
    remaining = [name for name in names if name != 'scanner.inc']
    bins = [[] for _ in caps]
    # The copied RAM scanner template must remain in the actual 16 KiB ROM span.
    bins[0] = ['scanner.inc']
    rng = random.Random(32017)
    for b in sorted(range(len(caps)), key=lambda b: caps[b]):
        rng.shuffle(remaining)
        reachable = {0: 0}
        for i, name in enumerate(remaining):
            for total, mask in list(reachable.items()):
                target = total + sizes[name]
                if target <= caps[b] - sum(sizes[n] for n in bins[b]) and target not in reachable:
                    reachable[target] = mask | (1 << i)
        mask = reachable[max(reachable)]
        bins[b] += [name for i, name in enumerate(remaining) if mask >> i & 1]
        remaining = [name for i, name in enumerate(remaining) if not mask >> i & 1]

    source = (SOURCE / 'fixed-layout.asm').read_text()
    source = re.sub(r'^\s*include "(?!abi.inc)[^"]+"\n', '', source, flags=re.M)
    for i, selected in enumerate(bins, 1):
        source = source.replace(f'ROM_GAP_{i}\n', ''.join(
            f'        include "{name}"\n' for name in selected
        ) + f'ROM_GAP_{i}\n')
    source += '\nCANDIDATE_OVERFLOW\n' + ''.join(f'        include "{name}"\n' for name in remaining)
    if expanded:
        source += '\nSTUDIO_CODE_END\n'
    (stage / 'candidate.asm').write_text(source)
    syms = assemble('candidate.asm')
    image = (stage / 'candidate.bin').read_bytes()
    if ((len(image) > 0x7f00 if expanded else len(image) >= 0x7f00)
            or syms['CANDIDATE_OVERFLOW'] != 0xc000):
        raise SystemExit('Candidate escaped its isolated address region')
    (stage / 'first16.bin').write_bytes(image[:16384])
    report = {'bytesWithFixedEntries': len(image), 'overflowBytes': len(image) - 16384,
              'codeAndDataBytes': sum(sizes.values()) + 16384 - sum(caps),
              'unusedFixedRegionBytes': sum(caps) - sum(sizes[n] for b in bins for n in b),
              'modules': sizes, 'overflowModules': remaining,
              'boardRomOrDistributionInput': False}
    report.update({
        'schemaVersion': 1,
        'kind': 'original-basic-test-candidate',
        'sourceSha256': source_hashes(),
        'builderSha256': digest(Path(__file__).read_bytes()),
        'imageSha256': digest(image),
        'symbolsSha256': digest((stage/'candidate.sym').read_bytes()),
    })
    if expanded:
        report['kind'] = 'original-basic-expanded-build'
        report['assemblyDefines'] = ['STUDIO_EXTENDED=1']
    (stage/'measurement.json').write_text(json.dumps(report, indent=2)+'\n')
    return report


def verify_candidate(directory):
    read_modules()
    report = json.loads((directory/'measurement.json').read_text())
    image = (directory/'candidate.bin').read_bytes()
    if (report.get('schemaVersion') != 1
            or report.get('kind') != 'original-basic-test-candidate'
            or report.get('boardRomOrDistributionInput') is not False
            or report.get('sourceSha256') != source_hashes()
            or report.get('builderSha256') != digest(Path(__file__).read_bytes())
            or report.get('imageSha256') != digest(image)
            or report.get('symbolsSha256') != digest((directory/'candidate.sym').read_bytes())
            or report.get('bytesWithFixedEntries') != len(image)
            or report.get('overflowBytes') != len(image)-16384
            or (directory/'first16.bin').read_bytes() != image[:16384]
            or not 16384 <= len(image) < 0x7f00):
        raise ValueError('Complete BASIC candidate provenance or bounds do not match')
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--verify', action='store_true', help='Check source/image provenance without an assembler')
    mode.add_argument('--check', action='store_true', help='Reassemble and compare the checked-in test artefacts')
    mode.add_argument('--output-dir', type=Path, help='Write a new candidate snapshot to a new directory')
    parser.add_argument('--lwasm', default=shutil.which('lwasm'))
    args = parser.parse_args()
    if args.verify:
        report = verify_candidate(QUALIFICATION)
    else:
        if not args.lwasm:
            parser.error('Select separately installed LWASM with --lwasm')
        assembler = Path(args.lwasm).resolve(strict=True)
        if args.output_dir and args.output_dir.exists():
            parser.error('Choose a new output directory; existing snapshots are never overwritten')
        with tempfile.TemporaryDirectory(prefix='eightbit-complete-basic-') as temporary:
            stage = Path(temporary)
            report = assemble_candidate(stage, assembler)
            verify_candidate(stage)
            if args.check:
                verify_candidate(QUALIFICATION)
                for name in ARTEFACTS:
                    if (stage/name).read_bytes() != (QUALIFICATION/name).read_bytes():
                        raise ValueError('Complete BASIC test artefacts do not reproduce: '+name)
            else:
                args.output_dir.mkdir(parents=True)
                for name in ARTEFACTS:
                    shutil.copy2(stage/name, args.output_dir/name)
    print(json.dumps({key: report[key] for key in ('bytesWithFixedEntries','overflowBytes','imageSha256','boardRomOrDistributionInput')}, indent=2))


if __name__ == '__main__':
    main()
