#!/usr/bin/env python3
"""Rebuild only our original monitor using an explicitly installed LWASM."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
FIRMWARE = ROOT / "families/dragon/firmware/monitor"

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--lwasm", default=shutil.which("lwasm"))
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    if not args.lwasm:
        parser.error("Select a separately installed LWASM with --lwasm")
    tool = Path(args.lwasm).resolve(strict=True)
    source = FIRMWARE / "monitor.asm"
    with tempfile.TemporaryDirectory(prefix="eightbit-monitor-") as directory:
        output = Path(directory) / "monitor.rom"
        symbols = Path(directory) / "monitor.sym"
        subprocess.run([str(tool), "--format=raw", f"--output={output}",
                        f"--symbol-dump={symbols}", str(source)], check=True, timeout=30)
        image = output.read_bytes()
        if len(image) != 16384:
            raise ValueError("Monitor must occupy exactly 16384 bytes")
        symbol_text = symbols.read_text()
        receipt = {"schemaVersion": 1, "id": "eightbit-dragon32-monitor-0.1",
                   "licence": "Apache-2.0", "bytes": len(image),
                   "sourceSha256": hashlib.sha256(source.read_bytes()).hexdigest(),
                   "romSha256": hashlib.sha256(image).hexdigest()}
        receipt_text = json.dumps(receipt, indent=2) + "\n"
        expected = [("monitor.rom", image), ("monitor.sym", symbol_text.encode()),
                    ("manifest.json", receipt_text.encode())]
        if args.check:
            if any((FIRMWARE / name).read_bytes() != data for name, data in expected):
                raise ValueError("Original monitor assets do not match their source")
        else:
            for name, data in expected:
                destination = FIRMWARE / name
                temporary = destination.with_suffix(destination.suffix + ".tmp")
                temporary.write_bytes(data)
                temporary.replace(destination)
        print("Original monitor verified:", receipt["romSha256"])

if __name__ == "__main__":
    main()
