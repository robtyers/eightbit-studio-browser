#!/usr/bin/env python3
"""Opt-in bounded XRoar observations of our original ROM; no OEM firmware."""
import argparse
from bisect import bisect_right
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent


def keyboard_fixture_image(original, symbols):
    """Instrument only our replaced keyboard body; require no unused ROM gap."""
    key = int(symbols['KEYIN'], 16)
    body_end = int(symbols['KEY_RETURN'], 16)
    boot = int(symbols['BOOT'], 16)
    idle = key + 3
    bootstrap = bytes.fromhex('7fffda7fffddfc7df01083a55a26f77e') + boot.to_bytes(2, 'big')
    if (len(original) != 16384 or not 0x8000 <= key < idle + len(bootstrap) <= body_end < 0xc000
            or original[key - 0x8000:key - 0x8000 + 5] != bytes.fromhex('3434bd0167')
            or original[0x33b4:0x33b7] != b'\x7e' + boot.to_bytes(2, 'big')):
        raise RuntimeError('Original keyboard/reset fixture boundaries differ from the verified layout')
    probe = bytearray(original)
    # The real keyboard body is unreachable in this synthetic supplier fixture.
    # Put its reset wait there, keeping the distributed ROM wholly unchanged.
    at = key - 0x8000
    probe[at:at + 3 + len(bootstrap)] = bytes.fromhex('7e6400') + bootstrap
    probe[0x33b4:0x33b7] = b'\x7e' + idle.to_bytes(2, 'big')
    return bytes(probe)


def cassette_fixture():
    """Original binary and tokenised files; physical input without fast traps."""
    tape = bytearray()
    for name, payload in ((b'SKIP    ', bytes.fromhex('1239')),
                          (b'NATIVE  ', bytes.fromhex('cc00a5fd630039'))):
        header = name + bytes((2, 0, 0, 0x60, 0, 0x60, 0))
        for kind, data in ((0, header), (1, payload), (255, b'')):
            tape.extend(b'\x55' * 128)
            tape.extend(bytes((0x55, 0x3c, kind, len(data))) + data
                        + bytes(((kind + len(data) + sum(data)) & 255, 0x55)))
    # Original tokenised line 10: POKE25346,85. Stale links must be rebuilt.
    program = bytes.fromhex('1234000a93') + b'25346,85' + bytes(3)
    header = b'BASIC   ' + bytes(7)
    for kind, data in ((0, header), (1, program), (255, b'')):
        tape.extend(b'\x55' * 128)
        tape.extend(bytes((0x55, 0x3c, kind, len(data))) + data
                    + bytes(((kind + len(data) + sum(data)) & 255, 0x55)))
    tape.extend(b'\x55' * 16)
    return bytes(tape)


def ascii_cassette_fixture():
    """Original source split inside lines/tokens; no third-party programme."""
    source = b'\r10 POKE25344,85\r20 PRINT"ASCII READY"\r30 END\r99 REM INCOMPLETE'
    header = b'TEXT    ' + bytes((0, 255, 255, 0, 0, 0, 0))
    blocks = [(0, header)] + [(1, source[i:i + 7]) for i in range(0, len(source), 7)] + [(255, b'')]
    tape = bytearray()
    for kind, data in blocks:
        tape.extend(b'\x55' * 128)
        tape.extend(bytes((0x55, 0x3c, kind, len(data))) + data
                    + bytes(((kind + len(data) + sum(data)) & 255, 0x55)))
    tape.extend(b'\x55' * 16)
    return bytes(tape)


def cassette_data(image):
    """Validate the documented CUE subset emitted by our save fixtures.

    Format facts: the authors' 2017 CUE proposal, not XRoar implementation code:
    https://archive.worldofdragon.org/phpBB3/viewtopic.php?f=8&t=5621
    Data ranges must be ordered and cover every non-padding byte. A silence
    chunk may replace only raw 55 leader padding, as emitted for gapped files.
    Custom-loader reordering and unknown extension types are deliberately closed.
    """
    if len(image) > 65536:
        raise RuntimeError('Original cassette capture exceeds its fixture bound')
    if not image.endswith(b'CUE]'):
        return image, None
    if len(image) < 12:
        raise RuntimeError('Original cassette capture has a truncated CUE footer')
    footer = len(image) - 8
    offset = int.from_bytes(image[footer:footer + 4], 'big')
    if offset > footer - 4 or image[offset:offset + 4] != b'[CUE':
        raise RuntimeError('Original cassette capture has an invalid CUE offset')
    # A run of 55 inside a payload is data, even when it resembles a leader.
    # Index only padding outside complete raw frames, before checking CUE holes.
    padding, raw_cursor = [], 0
    while raw_cursor < offset:
        start = raw_cursor
        while raw_cursor < offset and image[raw_cursor] == 0x55:
            raw_cursor += 1
        if start != raw_cursor:
            padding.append((start, raw_cursor))
        if raw_cursor == offset:
            break
        if raw_cursor + 4 > offset or image[raw_cursor] != 0x3c:
            raise RuntimeError('Original cassette capture has invalid CUE raw framing')
        raw_cursor += 4 + image[raw_cursor + 2]  # sync/type/length/payload/checksum
        if raw_cursor > offset:
            raise RuntimeError('Original cassette capture has truncated CUE raw framing')
    padding_starts = [start for start, _ in padding]

    def leader_padding(start, end):
        index = bisect_right(padding_starts, start) - 1
        return index >= 0 and padding[index][0] <= start <= end <= padding[index][1]

    cursor, covered, chunks, timing, silence = offset + 4, 0, [], False, False
    while cursor < footer:
        kind = image[cursor]
        cursor += 1
        if cursor >= footer:
            raise RuntimeError('Original cassette capture has a truncated CUE length')
        lead = image[cursor]
        extra = 0
        while extra < 5 and lead & (0x80 >> extra):
            extra += 1
        if extra > 4 or cursor + extra >= footer:
            raise RuntimeError('Original cassette capture has an invalid CUE length')
        length = lead & (0x7f >> extra)
        for value in image[cursor + 1:cursor + extra + 1]:
            length = (length << 8) | value
        length &= 0x7fffffff
        cursor += extra + 1
        if kind not in (0xf0, 0, 0x0d) or length != {0xf0: 4, 0: 2, 0x0d: 8}[kind]:
            raise RuntimeError('Original cassette capture has an unsupported CUE chunk')
        if cursor + length > footer:
            raise RuntimeError('Original cassette capture has a truncated CUE chunk')
        payload = image[cursor:cursor + length]
        if kind == 0xf0:
            frequencies = [int.from_bytes(payload[i:i + 2], 'big') for i in (0, 2)]
            if not all(frequencies):
                raise RuntimeError('Original cassette capture has zero CUE frequency')
            timing = True
            chunks.append({'kind': 'timing', 'frequenciesHz': frequencies})
        elif kind == 0:
            silence = True
            chunks.append({'kind': 'silence', 'milliseconds': int.from_bytes(payload, 'big')})
        else:
            start, end = [int.from_bytes(payload[i:i + 4], 'big') for i in (0, 4)]
            if (not timing or not covered <= start < end <= offset
                    or (start != covered and (not silence or not leader_padding(covered, start)))):
                raise RuntimeError('Original cassette capture has invalid CUE data coverage')
            covered = end
            silence = False
            chunks.append({'kind': 'data', 'start': start, 'end': end})
        cursor += length
    if (not covered or not chunks or (covered != offset
            and (not silence or not leader_padding(covered, offset)))):
        raise RuntimeError('Original cassette capture has incomplete CUE data coverage')
    return image[:offset], {'bytes': len(image) - offset, 'chunks': chunks}


def cassette_blocks(image):
    """Bounded structural check of our captured output, independent of ROM code."""
    image, _ = cassette_data(image)
    cursor, blocks = 0, []
    while cursor < len(image):
        while cursor < len(image) and image[cursor] == 0x55:
            cursor += 1
        if cursor == len(image):
            break
        if cursor + 4 > len(image) or image[cursor] != 0x3c:
            raise RuntimeError(f'Original cassette capture lost block framing at byte {cursor} after {len(blocks)} blocks')
        kind, length = image[cursor + 1:cursor + 3]
        end = cursor + 3 + length
        if end >= len(image):
            raise RuntimeError('Original cassette capture has a truncated block')
        payload = image[cursor + 3:end]
        if image[end] != (kind + length + sum(payload)) & 255:
            raise RuntimeError('Original cassette capture has an invalid checksum')
        blocks.append((kind, payload))
        cursor = end + 1
    return blocks


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--xroar', required=True)
    parser.add_argument('--execution-probe', action='store_true')
    parser.add_argument('--abi-probe', action='store_true')
    parser.add_argument('--graphics-probe', action='store_true')
    parser.add_argument('--paint-probe', action='store_true')
    parser.add_argument('--paint-gap-probe', action='store_true')
    parser.add_argument('--printer-probe', action='store_true')
    parser.add_argument('--native-compaction-probe', action='store_true')
    parser.add_argument('--numeric-probe', action='store_true')
    parser.add_argument('--cassette-probe', action='store_true')
    parser.add_argument('--cassette-save-probe', action='store_true')
    parser.add_argument('--cassette-ascii-probe', choices=('save', 'load', 'run', 'load-wav', 'run-wav'))
    parser.add_argument('--cassette-save-wav-probe', choices=('basic', 'machine'))
    parser.add_argument('--media-cli', type=Path)
    args = parser.parse_args()
    cassette_output = bool(args.cassette_save_probe or args.cassette_save_wav_probe
                           or args.cassette_ascii_probe == 'save')
    if args.cassette_save_wav_probe and not args.media_cli:
        parser.error('--media-cli is required for the independent WAV decoder')
    if args.cassette_ascii_probe in ('load-wav', 'run-wav') and not args.media_cli:
        parser.error('--media-cli is required for the independent ASCII WAV encoder')
    # Seven-byte blocks deliberately split source lines. Their physical ASCII
    # gaps add several seconds beyond the otherwise identical raw-CAS fixture.
    emulated_seconds = '30' if args.cassette_ascii_probe in ('load-wav', 'run-wav') else '15'
    executable = Path(args.xroar).resolve(strict=True)
    firmware = ROOT / 'families/dragon/firmware/basic'
    receipt = json.loads((firmware / 'manifest.json').read_text())
    original = (firmware / 'basic.rom').read_bytes()
    if (receipt['id'] != 'eightbit-dragon32-basic-0.1-dev'
            or receipt['licence'] != 'Apache-2.0'
            or receipt['status'] != 'development' or len(original) != 16384
            or hashlib.sha256(original).hexdigest() != receipt['romSha256']
            or hashlib.sha256((firmware / 'basic.sym').read_bytes()).hexdigest() != receipt['symbolsSha256']
            or any(hashlib.sha256((firmware / name).read_bytes()).hexdigest() != digest
                   for name, digest in receipt['sourceSha256'].items())):
        raise RuntimeError('Only our exact original development ROM may be tested')
    symbols = dict(line.split(' EQU $') for line in (firmware / 'basic.sym').read_text().splitlines())
    direct = int(symbols['DIRECT'], 16)
    with tempfile.TemporaryDirectory(prefix='eightbit-original-reference-') as directory:
        snapshot = Path(directory) / 'boot.ram'
        image = firmware / 'basic.rom'
        trap = direct
        if sum((args.execution_probe, args.abi_probe, args.graphics_probe, args.paint_probe,
                args.paint_gap_probe, args.printer_probe, args.native_compaction_probe, args.numeric_probe, args.cassette_probe, args.cassette_save_probe, bool(args.cassette_save_wav_probe), bool(args.cassette_ascii_probe))) > 1:
            parser.error('Select one original-ROM execution probe')
        source = None
        if any((args.abi_probe, args.execution_probe, args.graphics_probe, args.paint_probe,
                args.paint_gap_probe, args.printer_probe, args.native_compaction_probe, args.numeric_probe, args.cassette_probe, args.cassette_save_probe, bool(args.cassette_save_wav_probe), bool(args.cassette_ascii_probe))):
            ram_probe = bytes.fromhex('1f40fd62007e620e')
            pokes = ':'.join(f'POKE {25000 + i},{b}' for i, b in enumerate(ram_probe)) + '\r'
            if args.cassette_ascii_probe:
                if args.cassette_ascii_probe == 'save':
                    source = pokes + '10 PRINT"SAVED"\r20 END\rCSAVE"TEXT",A:EXEC25000\r'
                else:
                    load = ('M$="TEXT":RUN LEFT$(M$,4)' if args.cassette_ascii_probe.startswith('run')
                            else 'CLOAD"TEXT":POKE25344,99\rRUN')
                    source = pokes + load + '\rEXEC25000\r'
            elif args.cassette_save_probe or args.cassette_save_wav_probe:
                save = ('CSAVE"NATIVE"' if args.cassette_save_wav_probe == 'basic' else
                        'CSAVEM"BYTES",25000,25007,25000' if args.cassette_save_wav_probe else
                        'CSAVE"NATIVE":CSAVEM"BYTES",25000,25007,25000')
                source = pokes + '10 PRINT"SAVED"\r20 END\r' + save + ':EXEC25000\r'
            elif args.cassette_probe:
                source = (pokes + 'SKIPF"SKIP":CLOADM"NATIVE":EXEC\r'
                          'CLOAD"BASIC":POKE25346,99:RUN\r'
                          'POKE25347,PEEK(25346):RUN\rPRINT"CASSETTE READY":EXEC25000\r')
            elif args.numeric_probe:
                source = (pokes + 'A=&HFFFF:B=VAL("&177777"):C=1 E - 2:D=VAL("&H10REST")\r'
                          'E=RND(-1):F=RND(1E20):G=RND(-1):H=RND(0)\r'
                          'POKE278,201:POKE279,161:POKE280,66:POKE281,219:I=RND(0)\r'
                          'J=0:POKE VARPTR(J),158:POKE VARPTR(J)+1,110:POKE VARPTR(J)+2,107:'
                          'POKE VARPTR(J)+3,39:POKE VARPTR(J)+4,255\r'
                          'PRINT HEX$(-1)\rPRINT"ROUND";STR$(J)\rPRINT"NUMERIC READY":EXEC25000\r')
            elif args.native_compaction_probe:
                source = (pokes + '0 A=A+1:IF A<2 THEN 0\r10 TIMER=60000\r'
                          '20 B=TIMER:PRINT"NATIVE READY";A;B:END\rRUN\r'
                          'TIMER=-1\rTIMER=9:PRINT"TIMER READY":EXEC25000\r')
            elif args.printer_probe:
                source = (pokes + 'PRINT#-2,"ABC";TAB(8);"X";:A=POS(-2)\r'
                          'PRINT#-2,"":B=POS(-2):PRINT#0,"PRINTER READY";A;B:EXEC25000\r')
            elif args.paint_probe or args.paint_gap_probe:
                stripe = 'COLOR2:LINE(10,2)-(10,24),PSET:' if args.paint_gap_probe else ''
                source = (pokes + 'PMODE3,1:PCLS1:LINE(2,2)-(30,24),PSET,B:' + stripe + 'PAINT(5,5),2,4\r'
                          'A=PPOINT(5,5):B=PPOINT(0,0):C=PPOINT(2,2):D=PPOINT(29,23):EXEC25000\r')
            elif args.graphics_probe:
                source = (pokes + 'PMODE4,1:PCLS0:CIRCLE(128,96),60,1,.6:'
                          'A=PPOINT(188,96):B=PPOINT(128,131):C=PPOINT(68,96):'
                          'D=PPOINT(128,61):E=PPOINT(128,96):EXEC25000\r')
            elif args.abi_probe:
                source = (pokes + '5 DIM B$(2):A$="HELLO":B$(1)="WORLD"\r'
                          '10 FOR I=1 TO 30\r20 C$="ABC"+A$+(B$(1)+"!")\r'
                          '30 NEXT I\r'
                          '35 B$(0)="HELLO":MID$(B$(0),1+UN)=LEFT$("XYZ",1+QX):'
                          'Z=B$(0)="XELLO":D$="HELLO":MID$(D$,2)=D$\r'
                          '40 GOSUB 100\r50 END\r100 EXEC 25000\r110 RETURN\rRUN\r')
            else:
                source = pokes + 'A=2+3*4:PRINT "SAMPLE=";A:POKE 25344,165:EXEC 25000\r'
            trap = 0x620e
            # The explicit Intel HEX fixture supplies our RAM keyboard routine
            # and text; it does not qualify the physical keyboard implementation.
            probe = keyboard_fixture_image(original, symbols)
            image = Path(directory) / 'original-probe.rom'
            image.write_bytes(probe)
            records = []
            # Traps snapshot immediately, then run until timeout. Keep that
            # interval in an original idle loop rather than uninitialised RAM.
            for address, data_bytes in ((0x620e, bytes.fromhex('20fe')),
                                        (0x6400, bytes.fromhex('3410be6600a680bf66004d3590')),
                                        (0x6600, bytes.fromhex('6800')),
                                        (0x6800, source.encode('ascii') + b'\0'),
                                        (0x7df0, bytes.fromhex('a55a'))):
                if address + len(data_bytes) > 0x8000:
                    raise RuntimeError('Original RAM input exceeds its bounded fixture')
                for offset in range(0, len(data_bytes), 16):
                    chunk = data_bytes[offset:offset + 16]
                    row = bytes([len(chunk)]) + (address + offset).to_bytes(2, 'big') + b'\0' + chunk
                    records.append(':' + (row + bytes([-sum(row) & 255])).hex().upper())
            fixture = Path(directory) / 'original-input.hex'
            fixture.write_text('\n'.join(records + [':00000001FF']) + '\n')
        command = [str(executable), '-no-c', '-no-config-auto-save', '-ui', 'null',
                   '-ao', 'null', '-m', 'dragon32', '-extbas', str(image),
                   '-ram-init', 'clear',
                   '-rompath', directory, '-machine-cart', 'none', '-no-cart-autorun',
                   '-trap', f'pc={trap}', '-trap-snap', str(snapshot),
                   '-trap-timeout', '0.01', '-timeout', emulated_seconds,
                   '-no-ratelimit']
        if source is not None:
            command += ['-load', str(fixture)]
        if args.cassette_probe:
            cassette = Path(directory) / 'original-input.cas'
            cassette.write_bytes(cassette_fixture())
            command += ['-load-tape', str(cassette), '-no-tape-fast', '-no-tape-pad-auto']
        elif args.cassette_ascii_probe in ('load', 'run', 'load-wav', 'run-wav'):
            cassette = Path(directory) / 'original-ascii.cas'
            cassette.write_bytes(ascii_cassette_fixture())
            if args.cassette_ascii_probe.endswith('-wav'):
                audio = Path(directory) / 'original-ascii.wav'
                converted = subprocess.run([str(args.media_cli.resolve(strict=True)), 'media', 'convert',
                                            str(cassette), '--output', str(audio), '--to', 'dragon-cassette-wav'],
                                           capture_output=True, timeout=30)
                if converted.returncode or not audio.is_file():
                    raise RuntimeError('Independent Rust ASCII WAV encoding failed')
                cassette = audio
            command += ['-load-tape', str(cassette), '-no-tape-fast', '-no-tape-pad-auto']
        if cassette_output:
            captured = Path(directory) / ('original-output.wav' if args.cassette_save_wav_probe else 'original-output.cas')
            command += ['-tape-write', str(captured), '-no-tape-fast', '-no-tape-rewrite']
        if args.printer_probe:
            command += ['-lp-file', str(Path(directory) / 'printer.txt')]
        # The documented RAM trap works without GUI event processing or a GDB socket.
        with open(Path(directory) / 'reference.log', 'wb') as log:
            run = subprocess.run(command, cwd=directory, stdin=subprocess.DEVNULL,
                                 stdout=log, stderr=log, timeout=30, check=False,
                                 env={'PATH': '/usr/bin:/bin', 'LANG': 'C'})
        if run.returncode != 0 or not snapshot.is_file() or snapshot.stat().st_size != 32768:
            raise RuntimeError('Reference did not reach original-ROM direct mode with 32K RAM')
        data = snapshot.read_bytes()
        screen = data[0x400:0x600]
        text = ''.join(chr((b & 63) + 64 if b & 63 < 32 else b & 63) for b in screen)
        if not (args.abi_probe or args.native_compaction_probe or args.numeric_probe or args.cassette_probe or cassette_output or args.cassette_ascii_probe) and 'EIGHTBIT BASIC DEVELOPMENT' not in text:
            raise RuntimeError('Reference did not execute our original console boot')
        pointers = {f'{a:04x}': int.from_bytes(data[a:a + 2], 'big')
                    for a in (0x19, 0x1b, 0x1d, 0x1f, 0x21, 0x23, 0x27, 0x68)}
        if pointers['0019'] != 0x1e01 or (not args.abi_probe and pointers['0068'] != 0xffff):
            raise RuntimeError('Reference workspace differs from the required interface')
        execution = None
        if args.cassette_ascii_probe:
            if args.cassette_ascii_probe == 'save':
                capture = captured.read_bytes()
                expected = [(0, b'TEXT    ' + bytes((0, 255, 255, 0, 0, 0, 0))),
                            (1, b'\r10 PRINT"SAVED"\r20 END\r'), (255, b'')]
                if cassette_blocks(capture) != expected or data[0x81]:
                    raise RuntimeError('Original ASCII cassette output differs from its fixture')
                execution = {'asciiSaveObserved': True, 'capturedBlocks': 3,
                             'captureSha256': hashlib.sha256(capture).hexdigest(),
                             'validatedCue': cassette_data(capture)[1]}
            else:
                position = pointers['0019']
                numbers = []
                while len(numbers) < 4 and int.from_bytes(data[position:position + 2], 'big'):
                    numbers.append(int.from_bytes(data[position + 2:position + 4], 'big'))
                    position = int.from_bytes(data[position:position + 2], 'big')
                    if not pointers['0019'] <= position < pointers['001b']:
                        raise RuntimeError('Original ASCII loaded link escaped its programme')
                if numbers != [10, 20, 30] or data[0x6300] != 85 or data[0x81] or 'ASCII READY' not in text:
                    raise RuntimeError('Original ASCII cassette load/run failed')
                execution = {'asciiLoadObserved': True, 'filenameExpressionRunObserved': args.cassette_ascii_probe.startswith('run'),
                             'independentRustWavEncodeObserved': args.cassette_ascii_probe.endswith('-wav'),
                             'loadedLineNumbers': numbers, 'unterminatedTailDiscarded': True}
            execution.update({'fastTapeTrapsDisabled': True,
                              'keyboardMethod': 'original RAM keyboard supplier via explicit Intel HEX input'})
        if args.cassette_save_probe or args.cassette_save_wav_probe:
            if args.cassette_save_wav_probe:
                if not captured.is_file() or captured.stat().st_size > 1024 * 1024:
                    raise RuntimeError('Original WAV capture exceeds its fixture bound')
                decoded = Path(directory) / 'decoded-output.cas'
                with open(Path(directory) / 'media.log', 'wb') as log:
                    converted = subprocess.run([str(args.media_cli.resolve(strict=True)), 'media', 'convert',
                        str(captured), '--to', 'dragon-cassette', '--output', str(decoded)],
                        stdin=subprocess.DEVNULL, stdout=log, stderr=log, timeout=30, check=False,
                        env={'PATH': '/usr/bin:/bin', 'LANG': 'C'})
                if converted.returncode != 0 or not decoded.is_file() or decoded.stat().st_size > 65536:
                    raise RuntimeError('Independent Rust media conversion did not recover the original save')
                captured = decoded
            capture = captured.read_bytes()
            blocks = cassette_blocks(capture)
            _, cue = cassette_data(capture)
            programme = data[pointers['0019']:pointers['001b']]
            expected = [(0, b'NATIVE  ' + bytes(7)), (1, programme), (255, b''),
                        (0, b'BYTES   ' + bytes((2, 0, 0, 0x61, 0xa8, 0x61, 0xa8))),
                        (1, bytes.fromhex('1f40fd62007e620e')), (255, b'')]
            if args.cassette_save_wav_probe:
                expected = expected[:3] if args.cassette_save_wav_probe == 'basic' else expected[3:]
            if blocks != expected or data[0x81] != 0:
                raise RuntimeError('Original tokenised/binary cassette save capture differs from its fixture')
            execution = {'tokenisedBasicSaveObserved': args.cassette_save_wav_probe != 'machine',
                         'machineCodeSaveObserved': args.cassette_save_wav_probe != 'basic',
                         'independentRustWavDecodeObserved': bool(args.cassette_save_wav_probe),
                         'inclusiveBinaryPayloadBytes': 0 if args.cassette_save_wav_probe == 'basic' else 8, 'cassetteStatus': 0,
                         'capturedBlocks': len(blocks), 'captureSha256': hashlib.sha256(capture).hexdigest(),
                         'fastTapeTrapsDisabled': True,
                         'validatedCue': cue,
                         'keyboardMethod': 'original RAM keyboard supplier via explicit Intel HEX input'}
        if args.cassette_probe:
            if (data[0x6300:0x6302] != bytes((0, 165))
                    or int.from_bytes(data[0x9d:0x9f], 'big') != 0x6000
                    or data[0x6302] != 85 or data[0x6303] != 0 or pointers['001b'] != 0x1e11
                    or int.from_bytes(data[0x1e01:0x1e03], 'big') != 0x1e0f
                    or data[0x81] != 0 or 'CASSETTE READY' not in text):
                raise RuntimeError('Original named skip/load/default EXEC cassette probe failed')
            execution = {'namedSkipObserved': True, 'binaryLoadAndDefaultExecObserved': True,
                         'executionAddress': 24576, 'cassetteStatus': 0,
                         'tokenisedBasicLoadRelinkAndRunObserved': True,
                         'cloadDiscardsCommandLineTailObserved': True,
                         'fastTapeTrapsDisabled': True,
                         'keyboardMethod': 'original RAM keyboard supplier via explicit Intel HEX input'}
        if args.numeric_probe:
            values = {data[p]: data[p + 2:p + 7]
                      for p in range(pointers['001b'], pointers['001d'], 7)}
            expected = {65: bytes((144, 127, 255, 0, 0)), 66: bytes((144, 127, 255, 0, 0)),
                        67: bytes((122, 35, 215, 10, 62)), 68: bytes((133, 0, 0, 0, 0)),
                        69: bytes((128, 5, 176, 101, 139)), 70: bytes((193, 121, 53, 65, 85)),
                        71: bytes((128, 5, 176, 101, 139)), 72: bytes((127, 55, 226, 16, 201)),
                        73: bytes((123, 72, 189, 154, 151)),
                        74: bytes((158, 110, 107, 39, 255))}
            if (values != expected or any(marker not in text for marker in
                                          ('?FC ERROR', 'ROUND 1E+09', 'NUMERIC READY'))):
                raise RuntimeError('Original unsigned radix, VAL, decimal display/spacing or HEX rejection differs')
            execution = {'unsignedRadixLiteralsAndValObserved': True,
                         'decimalInternalWhitespaceObserved': True,
                         'negativeHexRejectedAndRecovered': True,
                         'randomGuardHalfwayAndLargeBoundObserved': True,
                         'exactPackedDecimalCarryObserved': True,
                         'keyboardMethod': 'original RAM keyboard supplier via explicit Intel HEX input'}
        if args.native_compaction_probe:
            values = {data[p]: data[p + 2:p + 7]
                      for p in range(pointers['001b'], pointers['001d'], 7)}
            timer = values.get(66, bytes(5))
            timer_value = ((int.from_bytes(timer[1:], 'big') | 0x80000000)
                           * 2 ** (timer[0] - 160)) if timer[0] else 0
            if (values.get(65) != bytes((130, 0, 0, 0, 0))
                    or not 60000 <= timer_value <= 60003 or data[0x112] != 0
                    or int.from_bytes(data[0x1e03:0x1e05], 'big') != 0
                    or any(marker not in text for marker in ('NATIVE READY', '?FC ERROR', 'TIMER READY'))):
                raise RuntimeError('Original line-zero, unsigned TIMER or assignment recovery failed')
            execution = {'lineZeroAndBranchObserved': True, 'loopIterations': 2,
                         'unsignedTimerReadAbove32767': True,
                         'timerAssignmentResetsUpperByte': True,
                         'invalidAssignmentRecoveryObserved': True,
                         'keyboardMethod': 'original RAM keyboard supplier via explicit Intel HEX input'}
        if args.execution_probe:
            if (data[0x6300] != 0xa5 or data[0x1e03:0x1e0a] != bytes([65, 0, 132, 96, 0, 0, 0])
                    or 'SAMPLE= 14' not in text):
                raise RuntimeError('Reference did not reproduce our guest scalar/arithmetic/PRINT probe')
            execution = {'originalGuestScalarValue': 14, 'printedResultObserved': True}
        if args.printer_probe:
            expected = bytes([65, 0, 132, 16, 0, 0, 0, 66, 0, 0, 0, 0, 0, 0])
            if data[pointers['001b']:pointers['001d']] != expected or 'PRINTER READY 9  0' not in text:
                raise RuntimeError('Original explicit printer/screen selection or column accounting failed')
            printed = (Path(directory) / 'printer.txt').read_bytes()
            if printed != b'ABC     X\r':
                raise RuntimeError('Original printer bus did not reproduce the synthetic output')
            execution = {'printerColumnAfterTab': 9, 'printerColumnAfterNewline': 0,
                         'printerBytesObserved': len(printed),
                         'explicitScreenOutputObserved': True,
                         'keyboardMethod': 'original RAM keyboard supplier via explicit Intel HEX input'}
        if args.paint_probe or args.paint_gap_probe:
            packed = {1: bytes([129, 0, 0, 0, 0]), 2: bytes([130, 0, 0, 0, 0]),
                      4: bytes([131, 0, 0, 0, 0])}
            expected = b''.join(bytes([name, 0]) + packed[value]
                                for name, value in zip(b'ABCD', (2, 1, 4, 2)))
            if data[pointers['001b']:pointers['001d']] != expected:
                raise RuntimeError('Original PAINT did not preserve its selected fill/border/exterior colours')
            execution = {'paintInterior': 2, 'paintExterior': 1, 'paintBorder': 4,
                         'graphicsRamSha256': hashlib.sha256(data[0x600:0x1e00]).hexdigest(),
                         'keyboardMethod': 'original RAM keyboard supplier via explicit Intel HEX input'}
        if args.graphics_probe:
            expected = b''.join(bytes([name, 0, 129, 0, 0, 0, 0]) for name in b'ABCD')
            expected += bytes([69, 0, 0, 0, 0, 0, 0])
            if data[pointers['001b']:pointers['001d']] != expected:
                raise RuntimeError('Original ellipse did not preserve its four cardinal points and clear centre')
            execution = {'ellipseCardinalsObserved': True, 'centreRemainsClear': True,
                         'graphicsRamSha256': hashlib.sha256(data[0x600:0x1e00]).hexdigest(),
                         'keyboardMethod': 'original RAM keyboard supplier via explicit Intel HEX input'}
        if args.abi_probe:
            values = {}
            numeric_values = {}
            for position in range(pointers['001b'], pointers['001d'], 7):
                name = data[position:position + 2]
                if name[1] & 128:
                    length = data[position + 2]
                    pointer = int.from_bytes(data[position + 4:position + 6], 'big')
                    if length and not 0x200 <= pointer <= 32768 - length:
                        raise RuntimeError('Original string pointer escaped RAM')
                    values[name[0]] = data[pointer:pointer + length]
                else:
                    numeric_values[name[0]] = data[position + 2:position + 7]
            sp = int.from_bytes(data[0x6200:0x6202], 'big')
            if (values.get(65) != b'HELLO' or values.get(67) != b'ABCHELLOWORLD!'
                    or values.get(68) != b'HHHHH'
                    or numeric_values.get(90) != bytes([129, 128, 0, 0, 0])
                    or numeric_values.get(73) != bytes([133, 120, 0, 0, 0])
                    or not 0 < pointers['0027'] - pointers['0023'] <= 200
                    or not 0x7f00 <= sp <= 0x7f36 or data[sp + 2] != 0xbd
                    or int.from_bytes(data[sp + 3:sp + 5], 'big') != 40):
                raise RuntimeError('Original array/string/collection/control probe failed')
            execution = {'stringArrayAndConcatenationObserved': True,
                         'midAssignmentArrayAndSelfOverlapObserved': True,
                         'forIterations': 30, 'gosubLine': 40,
                         'finalLoopVariable': 31,
                         'stringHeapCompactionObserved': True,
                         'keyboardMethod': 'original RAM keyboard supplier via explicit Intel HEX input'}
        print(json.dumps({'schemaVersion': 1, 'firmwareId': receipt['id'],
                          'romSha256': receipt['romSha256'],
                          'reference': 'separately installed XRoar', 'trapPc': trap,
                          'probe': (f'guest-cassette-ascii-{args.cassette_ascii_probe}' if args.cassette_ascii_probe else f'guest-cassette-save-wav-{args.cassette_save_wav_probe}' if args.cassette_save_wav_probe else 'guest-cassette-save' if args.cassette_save_probe else 'guest-cassette' if args.cassette_probe else 'guest-numeric' if args.numeric_probe else 'guest-native-compaction' if args.native_compaction_probe else 'guest-printer' if args.printer_probe else 'guest-paint-gap' if args.paint_gap_probe else 'guest-paint' if args.paint_probe else 'guest-graphics' if args.graphics_probe else 'guest-abi' if args.abi_probe
                                    else 'guest-execution' if args.execution_probe else 'boot'),
                          'execution': execution,
                          'workspace': pointers, 'screenRamSha256': hashlib.sha256(screen).hexdigest(),
                          'thirdPartyFirmwareUsed': False}))


if __name__ == '__main__':
    main()
