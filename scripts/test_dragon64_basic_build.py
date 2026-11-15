#!/usr/bin/env python3
"""Original D64 source, image, page-control and map regressions; no lawful ROM."""
import importlib.util
import json
from pathlib import Path
import shutil
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('dragon64_build', Path(__file__).with_name('build-dragon64-basic.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class Dragon64BuildTests(unittest.TestCase):
    def snapshot(self, directory):
        target = Path(directory) / 'firmware'
        shutil.copytree(module.DESTINATION, target)
        return target

    def test_exact_sources_images_symbols_controls_and_maps(self):
        record = module.verify(module.DESTINATION)
        self.assertEqual(record['status'], 'development')
        self.assertEqual(record['profiles']['compatibility']['programEndExclusive'], 0x8000)
        self.assertEqual(record['profiles']['native']['programEndExclusive'], 0xc000)
        self.assertGreater(record['profiles']['native']['bytes'], 16384)
        self.assertEqual(record['sourceSha256'], module.source_hashes())

    def test_all_truncated_artefacts_and_changed_bytes_fail(self):
        for name in module.ARTEFACTS:
            with self.subTest(name=name), tempfile.TemporaryDirectory() as temporary:
                target = self.snapshot(temporary)
                file = target / name
                data = file.read_bytes()
                file.write_bytes(data.rstrip()[:-1] if name == "manifest.json" else data[:-1])
                with self.assertRaises((ValueError, KeyError)):
                    module.verify(target)

    def test_map_types_extra_fields_and_unqualified_identity_are_rejected(self):
        cases = [('id','eightbit-dragon32-basic-extended-1'), ('pages',True),
                 ('pageStart',0xc000), ('programEndExclusive',65536),
                 ('controls',{}), ('entries',{}), ('extra','unreviewed')]
        for key, value in cases:
            with self.subTest(key=key), tempfile.TemporaryDirectory() as temporary:
                target = self.snapshot(temporary)
                file = target / 'manifest.json'
                record = json.loads(file.read_text())
                record['profiles']['native'][key] = value
                file.write_text(json.dumps(record))
                with self.assertRaises(ValueError):
                    module.verify(target)

    def test_d32_qualification_is_not_modified_or_inherited(self):
        record = module.shared.verify(module.shared.FIRMWARE)
        self.assertEqual(record['status'],'qualified')
        self.assertNotIn('dragon64/peripherals.inc', record['sourceSha256'])
        self.assertEqual(record['id'],'eightbit-dragon32-basic-extended-1')

    def test_symlinked_artefacts_are_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            target = self.snapshot(temporary)
            path = target / 'native.rom'
            path.unlink()
            path.symlink_to(module.DESTINATION / 'native.rom')
            with self.assertRaises(ValueError):
                module.verify(target)


if __name__ == '__main__':
    unittest.main()
