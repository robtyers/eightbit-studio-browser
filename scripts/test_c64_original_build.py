#!/usr/bin/env python3
"""Original container build boundary, no assembler or external firmware in ordinary tests."""
import importlib.util
import json
import os
from pathlib import Path
import sys
import tempfile
import time
import struct
import unittest

SPEC = importlib.util.spec_from_file_location('c64_builder', Path(__file__).with_name('build-c64-original.py'))
BUILDER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BUILDER)


class OriginalBuild(unittest.TestCase):
    def test_bundled_original_is_bound_to_current_source_builder_and_container(self):
        manifest = json.loads((BUILDER.SOURCE / 'runtime-manifest.json').read_text())
        image = (BUILDER.SOURCE / 'runtime.c64rom').read_bytes()
        self.assertEqual(BUILDER.digest(image).hex(), manifest['imageSha256'])
        self.assertEqual(len(image), manifest['imageBytes'])
        self.assertEqual(BUILDER.digest(Path(BUILDER.__file__).read_bytes()).hex(), manifest['builderSha256'])
        framed = bytearray()
        for source in BUILDER.INPUTS:
            data = source.read_bytes()
            path = source.relative_to(BUILDER.ROOT).as_posix().encode()
            framed.extend(struct.pack('>I', len(path)) + path + struct.pack('>Q', len(data)) + data)
        self.assertEqual(BUILDER.digest(framed).hex(), manifest['sourceSha256'])
        self.assertEqual(image[16:48].hex(), manifest['sourceSha256'])
        self.assertEqual(image[48:80].hex(), manifest['builderSha256'])
        self.assertEqual(len(manifest['gates']), 9)
    def test_keyword_inventory_is_bounded_and_emits_only_numeric_operands(self):
        text = BUILDER.KEYWORDS.read_text()
        encoded = BUILDER.keywords(text)
        self.assertEqual(len(encoded.splitlines()), 76)
        self.assertTrue(all(set(line) <= set('.byte 0123456789,') for line in encoded.splitlines()))
        for invalid in [text + 'EXTRA\n', text.replace('END\n', 'FOR\n', 1),
                        text.replace('END\n', 'TOO-LONG\n', 1), text.replace('END\n', '\x00\n', 1)]:
            with self.assertRaises(ValueError):
                BUILDER.keywords(invalid)

    def test_character_source_is_original_bounded_and_inverse_cells_are_complementary(self):
        doc = json.loads((BUILDER.SOURCE / 'characters.json').read_text())
        data = BUILDER.font(doc)
        self.assertEqual(len(data), 4096)
        self.assertNotEqual(data[8:16], data[2056:2064])
        self.assertEqual(data[32*8:64*8],data[2048+32*8:2048+64*8])
        self.assertEqual(data[1024:2048], bytes(value ^ 255 for value in data[:1024]))
        self.assertEqual(data[3072:4096], bytes(value ^ 255 for value in data[2048:3072]))
        self.assertEqual(data[96*8:97*8],bytes(8))
        self.assertEqual(data[32 * 8:33 * 8], bytes(8))
        doc['glyphs']['A'] = [32] * 7
        with self.assertRaises(ValueError):
            BUILDER.font(doc)

    def test_container_rejects_wrong_counts_and_lengths(self):
        good = [bytes(8192), bytes(8192), bytes(4096)]
        for images in [[], good[:2], good + [bytes(4096)], [bytes(8191), *good[1:]], [bytes(8192)] * 18 + [bytes(4096)]]:
            with self.assertRaises(ValueError):
                BUILDER.container(images, [], bytes(32), bytes(32))

    def test_single_complete_image_publication_is_no_clobber_and_idempotent(self):
        with tempfile.TemporaryDirectory() as folder:
            output = Path(folder) / 'nested/image.c64rom'
            with self.assertRaises(ValueError):
                BUILDER.publish(b'original', output, True)
            BUILDER.publish(b'original', output, False)
            BUILDER.publish(b'original', output, True)
            with self.assertRaises(ValueError):
                BUILDER.publish(b'changed', output, False)
            self.assertEqual(output.read_bytes(), b'original')
            link = Path(folder) / 'link'
            link.symlink_to(output)
            with self.assertRaises(ValueError):
                BUILDER.publish(b'original', link, False)
            with self.assertRaises(ValueError):
                BUILDER.publish(b'original', Path(folder), False)

    @unittest.skipUnless(os.name == 'posix', 'macOS/Linux build process boundary')
    def test_process_output_and_timeout_are_bounded_and_fail_without_publication(self):
        with tempfile.TemporaryDirectory() as folder:
            self.assertEqual(BUILDER.execute([sys.executable, '-c', 'print("OK")'], folder), 'OK')
            with self.assertRaisesRegex(ValueError, 'output limit'):
                BUILDER.execute([sys.executable, '-c', 'print("A"*10000)'], folder, limit=128)
            start = time.monotonic()
            with self.assertRaisesRegex(ValueError, 'timed out'):
                BUILDER.execute([sys.executable, '-c', 'import time;time.sleep(10)'], folder, timeout=0.1)
            self.assertLess(time.monotonic() - start, 3)


if __name__ == '__main__':
    unittest.main()
