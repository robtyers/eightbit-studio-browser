"""Ordinary parser tests use only original synthetic RAM, never OEM firmware."""
import importlib.util
from pathlib import Path
import unittest
from unittest.mock import patch
import tempfile

spec = importlib.util.spec_from_file_location('lawful_abi', Path(__file__).with_name('dragon32-lawful-abi.py'))
abi = importlib.util.module_from_spec(spec)
spec.loader.exec_module(abi)


def ram():
    data = bytearray(32768)
    for address, value in ((0x19, 0x1e01), (0x1b, 0x1e03), (0x1d, 0x1e03),
                           (0x1f, 0x1e03), (0x21, 0x7f36), (0x23, 0x7ffe),
                           (0x27, 0x7ffe), (0x6200, 0x7f33)):
        data[address:address + 2] = value.to_bytes(2, 'big')
    return data


class AbiObservations(unittest.TestCase):
    def test_empty_synthetic_workspace(self):
        observed = abi.observe(ram())
        self.assertEqual(observed['arrays'], [])
        self.assertEqual(observed['controlFrames'], [])
        self.assertEqual(observed['workspace']['0027'], 0x7ffe)

    def test_array_length_must_match_extents(self):
        data = ram()
        data[0x1f:0x21] = (0x1e0f).to_bytes(2, 'big')
        data[0x1e03:0x1e0f] = bytes([65, 0, 0, 12, 1, 0, 1, 129, 0, 0, 0, 0])
        self.assertEqual(abi.observe(data)['arrays'][0]['values'][0]['packed'], [129, 0, 0, 0, 0])
        data[0x1e09] = 2
        with self.assertRaises(ValueError):
            abi.observe(data)

    def test_rejects_truncated_workspace_and_unknown_stack(self):
        for size in (0, 32767, 32769):
            with self.assertRaises(ValueError):
                abi.observe(bytes(size))
        data = ram()
        data[0x1b:0x1d] = b'\0\0'
        with self.assertRaises(ValueError):
            abi.observe(data)
        data = ram()
        data[0x6200:0x6202] = (0x7f2c).to_bytes(2, 'big')
        data[0x7f2e] = 0x12
        with self.assertRaises(ValueError):
            abi.observe(data)

    def test_rom_string_pointer_is_never_followed(self):
        data = ram()
        data[0x1d:0x1f] = (0x1e0a).to_bytes(2, 'big')
        data[0x1f:0x21] = (0x1e0a).to_bytes(2, 'big')
        data[0x1e03:0x1e0a] = bytes([65, 128, 5, 0, 0x80, 0, 0])
        with self.assertRaisesRegex(ValueError, 'within RAM'):
            abi.observe(data)

    def test_unreviewed_firmware_is_rejected_before_process_launch(self):
        with tempfile.TemporaryDirectory(prefix='eightbit-abi-gate-test-') as directory:
            rom = Path(directory) / 'generated.rom'
            rom.write_bytes(bytes(16384))
            with patch('sys.argv', ['lawful-abi', '--rom', str(rom), '--xroar', '/usr/bin/true']), patch.object(abi.subprocess, 'run') as run:
                with self.assertRaisesRegex(ValueError, 'fingerprint'):
                    abi.main()
                run.assert_not_called()

    def test_synthetic_graphics_digest_never_follows_an_unreviewed_extent(self):
        data = ram()
        data[0xba:0xbc] = (0x600).to_bytes(2, 'big')
        for mode, size in enumerate((1536, 3072, 3072, 6144, 6144)):
            data[0xb6] = mode
            data[0xb7:0xb9] = (0x600 + size).to_bytes(2, 'big')
            result = abi.observe_synthetic_graphics(data)
            self.assertEqual((result['mode'], result['bytes']), (mode, size))
            self.assertEqual(len(result['sha256']), 64)
        for address, value in ((0xba, 0x8000), (0xb7, 0x8000), (0x19, 0x600)):
            invalid = bytearray(data)
            invalid[address:address + 2] = value.to_bytes(2, 'big')
            with self.assertRaisesRegex(ValueError, 'reviewed pages'):
                abi.observe_synthetic_graphics(invalid)
        data[0xb6] = 255
        with self.assertRaisesRegex(ValueError, 'graphics mode'):
            abi.observe_synthetic_graphics(data)

    def test_sprite_observation_is_limited_to_the_reviewed_synthetic_record(self):
        data = ram()
        data[0x1d:0x1f] = (0x1e0a).to_bytes(2, 'big')
        data[0x1f:0x21] = (0x1e45).to_bytes(2, 'big')
        data[0x1e03] = 65
        data[0xba:0xbc] = (0x600).to_bytes(2, 'big')
        data[0xb7:0xb9] = (0xc00).to_bytes(2, 'big')
        data[0x1e0a:0x1e13] = bytes((65, 0, 0, 59, 2, 0, 10, 0, 0))
        observed = abi.observe_synthetic_sprite(data)
        self.assertEqual(observed['storedExtents'], [10, 0])
        self.assertEqual(observed['syntheticPayload'], [0] * 50)
        for address, value in ((0x1e12, 2), (0x1e0e, 3), (0x1e0d, 60)):
            invalid = bytearray(data)
            invalid[address] = value
            with self.assertRaisesRegex(ValueError, 'synthetic sprite array'):
                abi.observe_synthetic_sprite(invalid)
        data[0xba:0xbc] = (0x8000).to_bytes(2, 'big')
        with self.assertRaisesRegex(ValueError, 'reviewed pages'):
            abi.observe_synthetic_sprite(data)



class SyntheticCassetteObservationTests(unittest.TestCase):
    def test_only_reviewed_original_marker_and_ram_entry_are_reported(self):
        data = bytearray(32768)
        data[0x1b:0x1f] = bytes.fromhex('1e031e18')
        data[0x1e03:0x1e18] = bytes((65,0,136,37,0,0,0,66,0,143,64,0,0,0,67,0,0,0,0,0,0))
        self.assertEqual(abi.observe_synthetic_cassette(data), {
            'markerByte': 165, 'executionAddress': 24576, 'cassetteStatus': 0,
            'fastTapeTrapsDisabled': True})
        data[0x1e0c:0x1e11] = bytes((144,37,90,0,0))
        with self.assertRaisesRegex(ValueError, 'Unexpected synthetic cassette'):
            abi.observe_synthetic_cassette(data)

    def test_basic_file_observer_requires_our_exact_relinked_programme(self):
        data = bytearray(32768)
        data[0x1b:0x1f] = bytes.fromhex('1e111e26')
        data[0x1e01:0x1e11] = bytes.fromhex('1e0f000a93') + b'25346,85' + bytes(3)
        data[0x1e11:0x1e26] = bytes((65,0,135,42,0,0,0,66,0,143,64,0,0,0,67,0,0,0,0,0,0))
        self.assertEqual(abi.observe_synthetic_cassette(data, basic=True)['markerByte'], 85)
        data[0x1e08] ^= 1
        with self.assertRaisesRegex(ValueError, 'tokenised programme'):
            abi.observe_synthetic_cassette(data, basic=True)

    def test_bad_layout_length_and_unreviewed_marker_fail_closed(self):
        for data in (bytearray(0), bytearray(32768)):
            with self.assertRaises(ValueError):
                abi.observe_synthetic_cassette(data)


if __name__ == '__main__':
    unittest.main()
