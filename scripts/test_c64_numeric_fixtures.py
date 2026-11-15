#!/usr/bin/env python3
"""Independent exact-rational oracle boundaries; no guest or reference implementation."""
import importlib.util
from fractions import Fraction
from pathlib import Path
import unittest

SPEC = importlib.util.spec_from_file_location('c64_numeric', Path(__file__).with_name('generate-c64-numeric-fixtures.py'))
NUMERIC = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(NUMERIC)


class NumericFixtures(unittest.TestCase):
    def test_public_representation_and_exact_integer_round_trip(self):
        for number, packed in [(0, '0000000000'), (1, '8100000000'), (-1, '8180000000'),
                               (32767, '8f7ffe0000'), (-32768, '9080000000')]:
            self.assertEqual(NUMERIC.encode(Fraction(number)), (packed, 0))
            self.assertEqual(NUMERIC.value(packed), number)
        for number in range(-32768, 32768):
            packed, status = NUMERIC.encode(Fraction(number))
            self.assertEqual(status, 0)
            self.assertEqual(NUMERIC.value(packed), number)

    def test_half_rounding_sign_overflow_and_lower_boundary(self):
        half = Fraction(1) + Fraction(1, 2**32)
        for sign, packed in [(1, '8100000001'), (-1, '8180000001')]:
            self.assertEqual(NUMERIC.encode(sign * half), (packed, 0))
        minimum = Fraction(1, 2**128)
        self.assertEqual(NUMERIC.encode(minimum), ('0100000000', 0))
        self.assertEqual(NUMERIC.encode(Fraction('2.938735877E-39')), ('0100000000', 0))
        self.assertEqual(NUMERIC.encode(Fraction('2.938735876E-39')), ('0000000000', 0))
        self.assertEqual(NUMERIC.encode(Fraction(2**127)), ('0000000000', 1))

    def test_nine_digit_display_and_scientific_thresholds(self):
        for number, expected in [('0', ' 0 '), ('-.5', '-.5 '), ('.01', ' .01 '),
                                 ('.001', ' 1E-03 '), ('999999999', ' 999999999 '),
                                 ('1000000000', ' 1E+09 '), ('1.234567895', ' 1.2345679 ')]:
            self.assertEqual(NUMERIC.display(Fraction(number)), expected)

    def test_checked_in_vectors_are_reproducible(self):
        self.assertEqual(NUMERIC.CONSTANTS.read_text(), NUMERIC.math_constants())
        for name, generator in [('numeric-binary.txt', NUMERIC.generated),
                                ('numeric-decimal.txt', NUMERIC.decimal_vectors),
                                ('numeric-display.txt', NUMERIC.display_vectors),
                                ('numeric-integer.txt', NUMERIC.integer_vectors),
                                ('numeric-control.txt', NUMERIC.control_vectors)]:
            self.assertEqual(NUMERIC.OUTPUT.with_name(name).read_text(), generator())

    def test_control_oracle_handles_order_bits_and_signed_bitwise_values(self):
        vectors = [row.split() for row in NUMERIC.control_vectors().splitlines()[1:]]
        equal = next(row for row in vectors if row[:3] == ['8', '8100000000', '8100000000'])
        self.assertEqual(equal[3:], ['8100000000', '0', '2'])
        for operation, left, right, expected, status, _ in vectors:
            if operation not in ['9', '10', '11'] or status != '0':
                continue
            result = NUMERIC.value(expected)
            self.assertTrue(-32768 <= result <= 32767 and result.denominator == 1)


if __name__ == '__main__':
    unittest.main()
