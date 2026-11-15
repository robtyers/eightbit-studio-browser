#!/usr/bin/env python3
"""Original exact-rational binary numeric vectors; no ROM, host float or BASIC evaluator."""
from fractions import Fraction
from math import isqrt
from decimal import Decimal, localcontext
from pathlib import Path
import argparse

ROOT = Path(__file__).resolve().parent.parent
OUTPUT = ROOT / 'families/commodore/crates/eightbit-commodore-machine/tests/fixtures/numeric-binary.txt'
CONSTANTS = ROOT / 'families/commodore/firmware/original/math-constants.inc'


def value(packed):
    exponent, *mantissa = bytes.fromhex(packed)
    if not exponent:
        return Fraction(0)
    magnitude = int.from_bytes(bytes([mantissa[0] | 128, *mantissa[1:]]), 'big')
    return Fraction((-1 if mantissa[0] & 128 else 1) * magnitude) * Fraction(2) ** (exponent - 160)


def encode(number):
    if not number:
        return '0000000000', 0
    sign = 128 if number < 0 else 0
    magnitude = abs(number)
    exponent = magnitude.numerator.bit_length() - magnitude.denominator.bit_length()
    if magnitude < Fraction(2) ** exponent:
        exponent -= 1
    if exponent < -129:
        return '0000000000', 0
    if exponent > 126:
        return '0000000000', 1
    scaled = magnitude * Fraction(2) ** (31 - exponent)
    rounded, remainder = divmod(scaled.numerator, scaled.denominator)
    if remainder * 2 >= scaled.denominator:
        rounded += 1
    if rounded == 1 << 32:
        rounded >>= 1
        exponent += 1
    if exponent < -128:
        return '0000000000', 0
    if exponent > 126:
        return '0000000000', 1
    mantissa = rounded.to_bytes(4, 'big')
    return bytes([exponent + 129, (mantissa[0] & 127) | sign, *mantissa[1:]]).hex(), 0


def generated():
    rows = ['# Original binary32-significand FAC vectors: op lhs rhs expected status',
            '# Exact Fraction arithmetic, half-magnitude-up rounding, underflow to zero.']
    inputs = ['0000000000', '8100000000', '8180000000', '8000000000', '8080000000',
              '8140000000', '81c0000000', '0100000000', '0180000000', 'ff7fffffff',
              'ffffffffff', '817fffffff', '8200000000', '6100000000', '6180000000',
              '627fffffff', '80ffffffff', 'ff00000000']
    pairs = [(left, right) for left in inputs for right in inputs]
    state = 0x6510c64
    for _ in range(256):
        parts = []
        for _ in range(2):
            state = (1664525 * state + 1013904223) & 0xffffffff
            exponent = 1 + state % 255
            state = (1664525 * state + 1013904223) & 0xffffffff
            parts.append(bytes([exponent]).hex() + state.to_bytes(4, 'big').hex())
        pairs.append(tuple(parts))
    for operation in range(4):
        for left, right in pairs:
            a, b = value(left), value(right)
            if operation == 3 and not b:
                result, status = '0000000000', 2
            else:
                result, status = encode([lambda: a+b, lambda: a-b, lambda: a*b, lambda: a/b][operation]())
            rows.append(f'{operation} {left} {right} {result} {status}')
    return '\n'.join(rows) + '\n'


def decimal_vectors():
    literals = ['0', '000.000', '1', '.5', '0.5', '1.', '1.5', '2', '32767', '32768',
                '65535', '123456789', '4294967295', '4294967296', '.1', '.2', '.3', '.01',
                '.001', '999999999', '1E9', '1E-3', '1E+38', '1.70141183E38',
                '1.701411834E38', '1.701411835E38', '2.93873588E-39',
                '2.938735877E-39', '2.938735876E-39', '1E-39',
                '2.9E-39', '3E-39', '1E-40', '1E-9999', '0E99999', '1E99999',
                '1.00000000023283064365386962890625',
                '1.00000000023283064365386962890624',
                '1.00000000023283064365386962890626',
                '1' + '0' * 79 + 'E-79', '9' * 80 + 'E-80']
    for exponent in [-38, -20, -3, 0, 8, 20, 38]:
        for digits in ['123456789', '3141592653589793238462643383279']:
            literals.append(digits + 'E' + str(exponent - len(digits) + 1))
    rows = ['# Original decimal -> packed value/status, exact Fraction oracle.']
    for literal in literals:
        if literal.endswith('-9999') or literal == '0E99999':
            packed, status = '0000000000', 0
        elif literal == '1E99999':
            packed, status = '0000000000', 1
        else:
            packed, status = encode(Fraction(literal))
        rows.append(f'{literal} {packed} {status}')
    for literal in ['.', '1..2', '1E', '1E+Z', '1Z']:
        rows.append(f'{literal} 0000000000 6')
    rows.append('1' * 81 + ' 0000000000 5')
    return '\n'.join(rows) + '\n'


def display(number):
    if not number:
        return ' 0 '
    prefix = '-' if number < 0 else ' '
    magnitude = abs(number)
    exponent = 0
    while magnitude < Fraction(10) ** exponent:
        exponent -= 1
    while magnitude >= Fraction(10) ** (exponent + 1):
        exponent += 1
    scaled = magnitude * Fraction(10) ** (8 - exponent)
    rounded, remainder = divmod(scaled.numerator, scaled.denominator)
    rounded += remainder * 2 >= scaled.denominator
    if rounded == 10**9:
        rounded //= 10
        exponent += 1
    digits = str(rounded).rstrip('0')
    if -2 <= exponent <= 8:
        position = exponent + 1
        if position <= 0:
            text = '.' + '0' * -position + digits
        elif position >= len(digits):
            text = digits + '0' * (position - len(digits))
        else:
            text = digits[:position] + '.' + digits[position:]
    else:
        text = digits[0] + ('.' + digits[1:] if len(digits) > 1 else '')
        text += 'E' + ('-' if exponent < 0 else '+') + f'{abs(exponent):02}'
    return prefix + text + ' '


def display_vectors():
    values = sorted({field for line in generated().splitlines() if not line.startswith('#')
                     for field in line.split()[1:4]})
    return '# Original packed value -> nine-digit display (ASCII hex).\n' + ''.join(
        f'{packed} {display(value(packed)).encode("ascii").hex()}\n' for packed in values)


def integer_vectors():
    rows = ['# Original signed integer conversions: op input expected status.']
    inputs = {-32768, -32767, -256, -255, -2, -1, 0, 1, 2, 255, 256, 32766, 32767}
    state = 0x6510
    for _ in range(512):
        state = (25173 * state + 13849) & 65535
        inputs.add(state - 32768)
    for integer in sorted(inputs):
        rows.append(f'6 {integer & 65535:04x} {encode(Fraction(integer))[0]} 0')
    values = {line.split()[0] for line in display_vectors().splitlines()[1:]}
    for number in ['-32768.9', '-32768.00001', '-32768', '-32767.9', '-1.9', '-.1',
                   '.1', '1.9', '32767.9', '32768']:
        values.add(encode(Fraction(number))[0])
    for packed in sorted(values):
        number = value(packed)
        status = 3 if number < -32768 or number >= 32768 else 0
        integer = number.numerator // number.denominator
        rows.append(f'7 {packed} {integer & 65535:04x} {status}')
        unsigned_status = 3 if number < 0 or number >= 65536 else 0
        rows.append(f'15 {packed} {integer & 65535:04x} {unsigned_status}')
    return '\n'.join(rows) + '\n'


def control_vectors():
    rows = ['# Original guest compare/AND/OR/NOT: op lhs rhs expected status order-bit.']
    for line in generated().splitlines()[2:]:
        operation, left, right, _, _ = line.split()
        if operation == '0':
            a, b = value(left), value(right)
            order = 1 if a > b else 4 if a < b else 2
            rows.append(f'8 {left} {right} {left} 0 {order}')
    numbers = [Fraction(x) for x in [-32769, -32768, -32767, -256, -2, -1, 0, 1,
                                    2, 127, 128, 255, 256, 32767, 32768]]
    numbers += [Fraction(x) for x in ['-32768.9', '-1.9', '-.1', '.1', '1.9', '32767.9']]
    inputs = [encode(number)[0] for number in numbers]
    pairs = [(left, right) for left in inputs for right in inputs]
    state = 0xc64
    for _ in range(256):
        operands = []
        for _ in range(2):
            state = (25173 * state + 13849) & 65535
            operands.append(encode(Fraction(state - 32768))[0])
        pairs.append(tuple(operands))
    for operation in [9, 10]:
        for left, right in pairs:
            a, b = value(left), value(right)
            status = 3 if not (-32768 <= a < 32768 and -32768 <= b < 32768) else 0
            x, y = a.numerator // a.denominator, b.numerator // b.denominator
            output = encode(Fraction(x & y if operation == 9 else x | y))[0] if not status else '0000000000'
            rows.append(f'{operation} {left} {right} {output} {status} 0')
    for left in sorted({operand for pair in pairs for operand in pair}):
        number = value(left)
        status = 3 if not -32768 <= number < 32768 else 0
        output = encode(Fraction(~(number.numerator // number.denominator)))[0] if not status else '0000000000'
        rows.append(f'11 {left} 0000000000 {output} {status} 0')
    unary = {line.split()[0] for line in display_vectors().splitlines()[1:]}
    for number in ['-.0001', '-.9999999999', '1.9999999999', '-32768.9',
                   '32767.9', '65535.9', '65536']:
        unary.add(encode(Fraction(number))[0])
    for packed in sorted(unary):
        number = value(packed)
        for operation, result in [(12, (number > 0) - (number < 0)),
                                  (13, abs(number)),
                                  (14, number.numerator // number.denominator)]:
            output, status = encode(Fraction(result))
            rows.append(f'{operation} {packed} 0000000000 {output} {status} 0')
        if number < 0:
            root, status = '0000000000', 3
        elif not number:
            root, status = '0000000000', 0
        else:
            exponent = bytes.fromhex(packed)[0] - 129
            scale = exponent // 2
            exact = number * Fraction(2) ** (62 - 2 * scale)
            assert exact.denominator == 1
            floor = isqrt(exact.numerator)
            rounded = floor + (exact.numerator - floor * floor > floor)
            root, status = encode(Fraction(rounded) * Fraction(2) ** (scale - 31))
        rows.append(f'17 {packed} 0000000000 {root} {status} 0')
    return '\n'.join(rows) + '\n'


def math_constants():
    # Machin's mathematical identity; original bounded Decimal series, neither
    # a recovered coefficient table nor emulator/interpreter implementation code.
    with localcontext() as context:
        context.prec = 100
        def atan_inverse(divisor):
            x = Decimal(1) / divisor
            term, total = x, Decimal(0)
            for index in range(256):
                contribution = term / (2 * index + 1)
                total += contribution
                term *= -x * x
                if abs(contribution) < Decimal('1e-95'):
                    return total
            raise ValueError('Mathematical constant convergence bound')
        pi = 16 * atan_inverse(5) - 4 * atan_inverse(239)
        values = [Decimal(1), Decimal(2), Decimal(2).ln(), pi, pi/2, pi/4, pi*2]
        rows = ['; Independently rounded mathematical constants: 1, 2, ln(2), pi, pi/2, pi/4, 2*pi.', 'math_constants:']
        for number in values:
            packed = bytes.fromhex(encode(Fraction(number))[0])
            unpacked = [packed[0], packed[1] | 128, *packed[2:], 255 if packed[1] & 128 else 0]
            rows.append('.byte ' + ','.join(str(byte) for byte in unpacked))
        return '\n'.join(rows) + '\n'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    outputs = [(OUTPUT, generated()), (OUTPUT.with_name('numeric-decimal.txt'), decimal_vectors()),
               (OUTPUT.with_name('numeric-display.txt'), display_vectors()),
               (OUTPUT.with_name('numeric-integer.txt'), integer_vectors()),
               (OUTPUT.with_name('numeric-control.txt'), control_vectors()),
               (CONSTANTS, math_constants())]
    for path, text in outputs:
        if args.check:
            if not path.is_file() or path.read_text() != text:
                parser.error('Original C64 numeric fixtures differ; regenerate explicitly')
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text)
        print(f'{path.name}: {sum(not line.startswith("#") for line in text.splitlines())} vectors')


if __name__ == '__main__':
    main()
