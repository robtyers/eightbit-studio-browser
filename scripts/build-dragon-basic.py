#!/usr/bin/env python3
"""Rebuild the original BASIC development image with separately installed LWASM."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parent.parent
FIRMWARE = ROOT / "families/dragon/firmware/basic"
SOURCES = ("break.inc", "graphics-sprite.inc", "play.inc", "draw.inc", "errors-data.inc", "errors.inc", "tape-ascii-in.inc", "tape-ascii-out.inc", "syntax-open.inc", "tape-save.inc", "exec.inc", "tape-load.inc", "tape-basic.inc", "tape-file.inc", "native-frame.inc", "statement-arguments.inc", "print-device.inc", "print-channel.inc", "command-guards.inc", "graphics-paint.inc", "mid-assignment.inc", "circle.inc", "console.inc", "series.inc", "line-range.inc", "delete.inc", "print-position.inc", "print-tab.inc", "graphics-page.inc", "graphics-copy.inc", "graphics-copy-page.inc", "input-start.inc", "input-variable.inc", "input-value.inc", "input-store.inc", "input-end.inc", "abi.inc", "arrays.inc", "basic.asm", "commands.inc", "control.inc", "data.inc", "devices.inc", "expression.inc", "extended-control.inc", "format.inc", "fp-program.inc", "function-definitions.inc", "function-call.inc", "function-name.inc", "function-return.inc", "unsigned.inc", "functions.inc", "graphics-box.inc", "graphics-fill-box.inc", "trace.inc", "graphics-cls.inc", "graphics-line.inc", "graphics-line-core.inc", "graphics-low.inc", "graphics-mode.inc", "graphics-point.inc", "graphics-screen.inc", "math.inc", "number.inc", "numeric.inc", "random.inc", "random-init.inc", "scanner.inc", "storage.inc", "sound.inc", "string-functions.inc", "strings-gc.inc", "strings-roots.inc", "strings.inc", "tape-write.inc", "tape-read.inc", "tape-block-out.inc", "tape-block-in.inc", "tokenise.inc", "value-functions.inc")
DIALECT = FIRMWARE.parent.parent / "dialects/dragon32-basic/tokens.toml"
GAP_ENDS = (0x84d6, 0x8b2d, 0x8c37, 0xa000, 0xb3b4, 0xb54a, 0xb93e,
            0xb999, 0xbb40, 0xbbb5, 0xbbbb, 0xbbe5, 0xbd52, 0xbda5,
            0xbdad, 0xbe12, 0xbe6a, 0xbff0)


def footprint(image, symbol_bytes):
    symbols = {name: int(value, 16) for name, value in re.findall(
        r'^(\w+) EQU \$(\w+)$', symbol_bytes.decode('ascii'), re.M)}
    gaps = []
    for index, end in enumerate(GAP_ENDS, 1):
        start = symbols[f'ROM_GAP_{index}']
        if not 0x8000 <= start <= end <= 0xc000:
            raise ValueError('Original BASIC segment exceeds its fixed ROM entry point')
        if image[start - 0x8000:end - 0x8000] != b'\x12' * (end - start):
            raise ValueError('Original BASIC free space is not deterministic NOP padding')
        gaps.append({'start': start, 'bytes': end - start})
    free = sum(gap['bytes'] for gap in gaps)
    return {'freeBytes': free, 'usedBytes': len(image) - free,
            'largestGapBytes': max(gap['bytes'] for gap in gaps), 'gaps': gaps}


def qualification_identity(directory, receipt):
    """Admission is a reviewed, exact build record, never inferred from image fit."""
    path = directory / "qualification.json"
    if not os.path.lexists(path):
        return {"id": "eightbit-dragon32-basic-0.1-dev", "status": "development"}
    if path.is_symlink() or not path.is_file() or path.stat().st_size > 65536:
        raise ValueError("Original BASIC qualification must be a bounded regular file")
    qualification = json.loads(path.read_text(encoding="utf-8"))
    expected = {key: receipt[key] for key in (
        "schemaVersion", "licence", "bytes", "romSha256", "symbolsSha256", "sourceSha256")}
    expected.update(id="eightbit-dragon32-basic-1", status="qualified")
    if (qualification != expected or type(qualification.get("schemaVersion")) is not int
            or type(qualification.get("bytes")) is not int):
        raise ValueError("Original BASIC qualification does not match this exact build")
    return {"id": qualification["id"], "status": qualification["status"]}


def keyword_source():
    dialect = tomllib.loads(DIALECT.read_text())
    lines = ["; Generated from the repository's original Dragon token contract. Apache-2.0.",
             "LEGACY_STATEMENT_WORDS"]
    for item in dialect["statement"]:
        values = list(item["text"].encode("ascii"))
        values[-1] |= 128
        lines.append("        fcb     " + ",".join(str(value) for value in values))
    lines.append("LEGACY_FUNCTION_WORDS")
    for item in dialect["function"]:
        values = list(item["text"].encode("ascii"))
        values[-1] |= 128
        lines.append("        fcb     " + ",".join(str(value) for value in values))
    lines += [f"STATEMENT_COUNT equ {len(dialect['statement'])}",
              f"FUNCTION_COUNT equ {len(dialect['function'])}"]
    implemented = {"GET":"CMD_GET", "PUT":"CMD_PUT", "PLAY":"CMD_PLAY", "DRAW":"CMD_DRAW", "CSAVE":"CMD_CSAVE", "CLOAD":"CMD_CLOAD", "SKIPF":"CMD_SKIPF", "PAINT":"CMD_PAINT", "CIRCLE":"CMD_CIRCLE", "DEL":"CMD_DEL", "LLIST":"CMD_LLIST", "PCOPY":"CMD_PCOPY", "INPUT":"CMD_INPUT", "LINE":"CMD_LINE", "SOUND":"CMD_SOUND", "MOTOR":"CMD_MOTOR", "AUDIO":"CMD_AUDIO", "ON":"CMD_ON", "DEF":"CMD_DEF", "TRON":"CMD_TRON", "TROFF":"CMD_TROFF", "SET":"CMD_SET", "RESET":"CMD_RESET", "SCREEN":"CMD_SCREEN", "PCLEAR":"CMD_PCLEAR", "PMODE":"CMD_PMODE", "PCLS":"CMD_PCLS", "PSET":"CMD_PSET", "PRESET":"CMD_PRESET", "COLOR":"CMD_COLOR", "DATA": "CMD_DATA", "READ": "CMD_READ", "RESTORE": "CMD_RESTORE", "FOR": "CMD_FOR", "NEXT": "CMD_NEXT", "RETURN": "CMD_RETURN", "DIM": "CMD_DIM", "GO": "CMD_GO", "IF": "CMD_IF", "ELSE": "CMD_REM", "REM": "CMD_REM", "'": "CMD_REM", "PRINT": "CMD_PRINT",
                   "END": "CMD_END", "LET": "CMD_LET", "RUN": "CMD_RUN", "STOP": "CMD_STOP",
                   "CONT": "CMD_CONT", "LIST": "CMD_LIST", "CLEAR": "CMD_CLEAR", "NEW": "CMD_NEW", "CLS": "CMD_CLS_COLOUR", "POKE": "CMD_POKE", "EXEC": "CMD_EXEC"}
    lines.append("STATEMENT_TABLE")
    lines += ["        fdb     " + implemented.get(item["text"], "CMD_UNSUPPORTED")
              for item in dialect["statement"]]
    lines.append("FUNCTION_TABLE")
    value_functions = ("EX_SGN", "F_INT_VALUE", "EX_ABS", "VALUE_POS", "VALUE_RND",
                       "F_SQR", "F_LOG", "F_EXP", "F_SIN", "F_COS", "F_TAN",
                       "F_ATN", "EX_PEEK", "STRING_LENGTH", "VALUE_STR", "STRING_VAL",
                       "STRING_ASC", "STRING_CHR", "VALUE_EOF", "VALUE_JOY",
                       "F_FIX_VALUE", "VALUE_HEX", "EX_MULTI_STRING", "EX_MULTI_STRING",
                       "EX_MULTI_STRING", "EX_POINT", "EX_INKEY", "EX_MEM", "EX_VARPTR",
                       "EX_MULTI_STRING", "EX_TIMER", "EX_PPOINT", "EX_MULTI_STRING", "EX_USR")
    lines += ["        fdb     " + (value_functions[index] if index < len(value_functions)
                                   else "EX_ERROR")
              for index, _ in enumerate(dialect["function"])]
    lines += ["TOKEN_STUBS_ROM", "        fcb     STATEMENT_COUNT",
              "        fdb     LEGACY_STATEMENT_WORDS,STATEMENT_TABLE",
              "        fcb     FUNCTION_COUNT", "        fdb     LEGACY_FUNCTION_WORDS,FUNCTION_TABLE",
              "        fcb     0", "        fdb     0,EXECUTE_STATEMENT",
              "        fcb     0", "        fdb     0,EX_FUNC_APPLY"]
    return ("\n".join(lines) + "\n").encode()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--lwasm", default=shutil.which("lwasm"))
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    if not args.lwasm:
        parser.error("Select separately installed LWASM with --lwasm")
    tool = Path(args.lwasm).resolve(strict=True)
    hashes = {name: hashlib.sha256((FIRMWARE / name).read_bytes()).hexdigest()
              for name in SOURCES}
    keywords = keyword_source()
    hashes["keywords.inc"] = hashlib.sha256(keywords).hexdigest()
    hashes["../../dialects/dragon32-basic/tokens.toml"] = hashlib.sha256(DIALECT.read_bytes()).hexdigest()
    with tempfile.TemporaryDirectory(prefix="eightbit-basic-") as directory:
        staged = Path(directory) / "source"
        staged.mkdir()
        for name in SOURCES:
            shutil.copyfile(FIRMWARE / name, staged / name)
        (staged / "keywords.inc").write_bytes(keywords)
        output = Path(directory) / "basic.rom"
        symbols = Path(directory) / "basic.sym"
        subprocess.run([str(tool), "--format=raw", f"--includedir={staged}",
                        f"--output={output}", f"--symbol-dump={symbols}",
                        str(staged / "basic.asm")], check=True, timeout=30)
        image = output.read_bytes()
        symbol_bytes = symbols.read_bytes()
        if len(image) != 16384:
            raise ValueError("BASIC must occupy exactly 16384 bytes")
        receipt = {"schemaVersion": 1, "id": "eightbit-dragon32-basic-0.1-dev",
                   "status": "development", "licence": "Apache-2.0",
                   "bytes": len(image), "sourceSha256": hashes,
                   "symbolsSha256": hashlib.sha256(symbol_bytes).hexdigest(),
                   "layout": footprint(image, symbol_bytes),
                   "romSha256": hashlib.sha256(image).hexdigest()}
        receipt.update(qualification_identity(FIRMWARE, receipt))
        expected = [("keywords.inc", keywords), ("basic.rom", image), ("basic.sym", symbol_bytes),
                    ("manifest.json", (json.dumps(receipt, indent=2) + "\n").encode())]
        if args.check:
            if any((FIRMWARE / name).read_bytes() != data for name, data in expected):
                raise ValueError("Original BASIC assets do not match their source")
        else:
            for name, data in expected:
                destination = FIRMWARE / name
                temporary = destination.with_suffix(destination.suffix + ".tmp")
                temporary.write_bytes(data)
                temporary.replace(destination)
        print(f"Original BASIC {receipt['status']} image verified:", receipt["romSha256"])
        print(f"ROM free: {receipt['layout']['freeBytes']} bytes; largest gap: "
              f"{receipt['layout']['largestGapBytes']} bytes")


if __name__ == "__main__":
    main()
