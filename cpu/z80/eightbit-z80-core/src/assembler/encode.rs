use super::{AssemblyError, Context};

#[derive(Clone, Copy)]
struct R8 {
    code: u8,
    prefix: u8,
    displacement: Option<u8>,
}
fn pair(text: &str) -> Option<(u8, u8)> {
    match text {
        "BC" => Some((0, 0)),
        "DE" => Some((1, 0)),
        "HL" => Some((2, 0)),
        "SP" => Some((3, 0)),
        "IX" => Some((2, 0xdd)),
        "IY" => Some((2, 0xfd)),
        _ => None,
    }
}
fn condition(text: &str) -> Option<u8> {
    ["NZ", "Z", "NC", "C", "PO", "PE", "P", "M"]
        .iter()
        .position(|v| *v == text)
        .and_then(|v| u8::try_from(v).ok())
}
fn indirect(text: &str) -> Option<&str> {
    text.strip_prefix('(')?.strip_suffix(')').map(str::trim)
}
fn prefixed(prefix: u8, bytes: impl IntoIterator<Item = u8>) -> Vec<u8> {
    let mut result = Vec::with_capacity(4);
    if prefix != 0 {
        result.push(prefix);
    }
    result.extend(bytes);
    result
}
fn normalise(text: &str) -> String {
    let mut bytes = text.as_bytes().to_vec();
    let mut cursor = 0;
    while cursor < bytes.len() {
        if bytes[cursor] == b'\'' && bytes.get(cursor + 2) == Some(&b'\'') {
            cursor += 3;
        } else {
            bytes[cursor].make_ascii_uppercase();
            cursor += 1;
        }
    }
    String::from_utf8(bytes).unwrap_or_default()
}
impl Context<'_> {
    fn reg8(&self, text: &str) -> Result<Option<R8>, AssemblyError> {
        let (code, prefix) = match text {
            "B" => (0, 0),
            "C" => (1, 0),
            "D" => (2, 0),
            "E" => (3, 0),
            "H" => (4, 0),
            "L" => (5, 0),
            "(HL)" => (6, 0),
            "A" => (7, 0),
            "IXH" => (4, 0xdd),
            "IXL" => (5, 0xdd),
            "IYH" => (4, 0xfd),
            "IYL" => (5, 0xfd),
            _ => {
                if let Some(inner) = indirect(text) {
                    let compact = inner.trim();
                    let index = if let Some(rest) = compact.strip_prefix("IX") {
                        Some((0xdd, rest))
                    } else {
                        compact.strip_prefix("IY").map(|rest| (0xfd, rest))
                    };
                    if let Some((prefix, rest)) = index {
                        let rest = rest.trim();
                        if !rest.is_empty() && !rest.starts_with(['+', '-']) {
                            return Ok(None);
                        }
                        let displacement = if rest.is_empty() {
                            0
                        } else {
                            self.value(rest)?
                        };
                        if self.final_pass && !(-128..=127).contains(&displacement) {
                            return Err(self.fail("Index displacement exceeds -128..127"));
                        }
                        return Ok(Some(R8 {
                            code: 6,
                            prefix,
                            displacement: Some(displacement as u8),
                        }));
                    }
                }
                return Ok(None);
            }
        };
        Ok(Some(R8 {
            code,
            prefix,
            displacement: None,
        }))
    }
    fn combine(&self, a: R8, b: R8) -> Result<u8, AssemblyError> {
        if a.prefix != 0 && b.prefix != 0 && a.prefix != b.prefix {
            return Err(self.fail("IX and IY cannot share an instruction"));
        }
        if a.code == 6 && b.code == 6 {
            return Err(self.fail("A memory-to-memory LD is unavailable"));
        }
        let prefix = a.prefix | b.prefix;
        if prefix != 0 {
            let memory = a.code == 6 || b.code == 6;
            let half = (a.prefix != 0 && a.code != 6) || (b.prefix != 0 && b.code != 6);
            let ordinary_hl = (a.prefix == 0 && matches!(a.code, 4 | 5))
                || (b.prefix == 0 && matches!(b.code, 4 | 5));
            if (memory && half)
                || (!memory && ordinary_hl)
                || (a.code == 6 && a.prefix == 0)
                || (b.code == 6 && b.prefix == 0)
            {
                return Err(self.fail("Index halves and real H/L memory forms cannot be mixed"));
            }
        }
        Ok(prefix)
    }
    fn load(&self, operands: &[String]) -> Result<Vec<u8>, AssemblyError> {
        let [a, b] = operands else {
            return Err(self.fail("LD requires two operands"));
        };
        let special = match (a.as_str(), b.as_str()) {
            ("A", "I") => Some(0x57),
            ("A", "R") => Some(0x5f),
            ("I", "A") => Some(0x47),
            ("R", "A") => Some(0x4f),
            _ => None,
        };
        if let Some(opcode) = special {
            return Ok(vec![0xed, opcode]);
        }
        if a == "SP"
            && let Some((2, prefix)) = pair(b)
        {
            return Ok(prefixed(prefix, [0xf9]));
        }
        let ar = self.reg8(a)?;
        let br = self.reg8(b)?;
        if let (Some(a), Some(b)) = (ar, br) {
            let prefix = self.combine(a, b)?;
            let mut bytes = prefixed(prefix, [0x40 + a.code * 8 + b.code]);
            bytes.extend(a.displacement.or(b.displacement));
            return Ok(bytes);
        }
        if let Some((register, prefix)) = pair(a) {
            if let Some(address) = indirect(b) {
                let address = self.address(address)?.to_le_bytes();
                return Ok(if register == 2 {
                    prefixed(prefix, [0x2a, address[0], address[1]])
                } else {
                    vec![0xed, 0x4b + register * 16, address[0], address[1]]
                });
            }
            let value = self.word_value(b)?.to_le_bytes();
            return Ok(prefixed(prefix, [1 + register * 16, value[0], value[1]]));
        }
        if let Some(address) = indirect(a)
            && let Some((register, prefix)) = pair(b)
        {
            let address = self.address(address)?.to_le_bytes();
            return Ok(if register == 2 {
                prefixed(prefix, [0x22, address[0], address[1]])
            } else {
                vec![0xed, 0x43 + register * 16, address[0], address[1]]
            });
        }
        if a == "A"
            && let Some(address) = indirect(b)
        {
            return match address {
                "BC" => Ok(vec![0x0a]),
                "DE" => Ok(vec![0x1a]),
                _ => {
                    let address = self.address(address)?.to_le_bytes();
                    Ok(vec![0x3a, address[0], address[1]])
                }
            };
        }
        if b == "A"
            && let Some(address) = indirect(a)
        {
            return match address {
                "BC" => Ok(vec![2]),
                "DE" => Ok(vec![0x12]),
                _ => {
                    let address = self.address(address)?.to_le_bytes();
                    Ok(vec![0x32, address[0], address[1]])
                }
            };
        }
        if let Some(register) = ar {
            if indirect(b).is_some() {
                return Err(self.fail("Unsupported LD memory source"));
            }
            let mut bytes = prefixed(register.prefix, [6 + register.code * 8]);
            bytes.extend(register.displacement);
            bytes.push(self.byte(b)?);
            return Ok(bytes);
        }
        Err(self.fail("Unsupported LD operands"))
    }
    pub(super) fn instruction(&self, op: &str, text: &str) -> Result<Vec<u8>, AssemblyError> {
        let raw = self.operands(text)?;
        let operands: Vec<String> = raw.iter().map(|s| normalise(s)).collect();
        let a = operands.first().map_or("", String::as_str);
        let b = operands.get(1).map_or("", String::as_str);
        let fixed = match op {
            "NOP" => Some(&[0x00][..]),
            "HALT" => Some(&[0x76][..]),
            "DI" => Some(&[0xf3][..]),
            "EI" => Some(&[0xfb][..]),
            "EXX" => Some(&[0xd9][..]),
            "RLCA" => Some(&[0x07][..]),
            "RRCA" => Some(&[0x0f][..]),
            "RLA" => Some(&[0x17][..]),
            "RRA" => Some(&[0x1f][..]),
            "DAA" => Some(&[0x27][..]),
            "CPL" => Some(&[0x2f][..]),
            "SCF" => Some(&[0x37][..]),
            "CCF" => Some(&[0x3f][..]),
            "NEG" => Some(&[0xed, 0x44][..]),
            "RETI" => Some(&[0xed, 0x4d][..]),
            "RETN" => Some(&[0xed, 0x45][..]),
            "RRD" => Some(&[0xed, 0x67][..]),
            "RLD" => Some(&[0xed, 0x6f][..]),
            "LDI" => Some(&[0xed, 0xa0][..]),
            "CPI" => Some(&[0xed, 0xa1][..]),
            "INI" => Some(&[0xed, 0xa2][..]),
            "OUTI" => Some(&[0xed, 0xa3][..]),
            "LDD" => Some(&[0xed, 0xa8][..]),
            "CPD" => Some(&[0xed, 0xa9][..]),
            "IND" => Some(&[0xed, 0xaa][..]),
            "OUTD" => Some(&[0xed, 0xab][..]),
            "LDIR" => Some(&[0xed, 0xb0][..]),
            "CPIR" => Some(&[0xed, 0xb1][..]),
            "INIR" => Some(&[0xed, 0xb2][..]),
            "OTIR" => Some(&[0xed, 0xb3][..]),
            "LDDR" => Some(&[0xed, 0xb8][..]),
            "CPDR" => Some(&[0xed, 0xb9][..]),
            "INDR" => Some(&[0xed, 0xba][..]),
            "OTDR" => Some(&[0xed, 0xbb][..]),
            _ => None,
        };
        if let Some(bytes) = fixed {
            if !operands.is_empty() {
                return Err(self.fail("This instruction has no operands"));
            }
            return Ok(bytes.to_vec());
        }
        match op {
            "LD" => self.load(&operands),
            "INC" | "DEC" if operands.len() == 1 => {
                let subtract = u8::from(op == "DEC");
                if let Some((register, prefix)) = pair(a) { return Ok(prefixed(prefix, [3 + register * 16 + subtract * 8])); }
                let register = self.reg8(a)?.ok_or_else(|| self.fail("INC/DEC requires a register or indexed memory"))?;
                let mut bytes = prefixed(register.prefix, [4 + register.code * 8 + subtract]);
                bytes.extend(register.displacement);
                Ok(bytes)
            }
            "ADD" | "ADC" | "SBC" if operands.len() == 2 && pair(a).is_some() => {
                let (left, prefix) = pair(a).ok_or_else(|| self.fail("Expected a register pair"))?;
                let (right, right_prefix) = pair(b).ok_or_else(|| self.fail("Expected a register pair"))?;
                if left != 2 || (right == 2 && prefix != right_prefix) || (right != 2 && right_prefix != 0) { return Err(self.fail("Invalid 16-bit arithmetic registers")); }
                if op == "ADD" { Ok(prefixed(prefix, [9 + right * 16])) }
                else if prefix == 0 { Ok(vec![0xed, (if op == "ADC" { 0x4a } else { 0x42 }) + right * 16]) }
                else { Err(self.fail("ADC/SBC register-pair destination must be HL")) }
            }
            "ADD" | "ADC" | "SUB" | "SBC" | "AND" | "XOR" | "OR" | "CP" if (1..=2).contains(&operands.len()) => {
                if operands.len() == 2 && a != "A" { return Err(self.fail("Byte arithmetic destination must be A")); }
                let value = if operands.len() == 1 { a } else { b };
                let group = match op { "ADD" => 0, "ADC" => 1, "SUB" => 2, "SBC" => 3, "AND" => 4, "XOR" => 5, "OR" => 6, _ => 7 };
                if let Some(register) = self.reg8(value)? {
                    let mut bytes = prefixed(register.prefix, [0x80 + group * 8 + register.code]);
                    bytes.extend(register.displacement);
                    Ok(bytes)
                } else if indirect(value).is_none() { Ok(vec![0xc6 + group * 8, self.byte(value)?]) }
                else { Err(self.fail("Unsupported arithmetic memory operand")) }
            }
            "PUSH" | "POP" if operands.len() == 1 => {
                let (register, prefix) = if a == "AF" { (3, 0) }
                    else { pair(a).filter(|(r, _)| *r != 3).ok_or_else(|| self.fail("PUSH/POP requires AF, BC, DE, HL, IX or IY"))? };
                Ok(prefixed(prefix, [(if op == "PUSH" { 0xc5 } else { 0xc1 }) + register * 16]))
            }
            "EX" if operands.len() == 2 => match (a, b) {
                ("AF", "AF'") => Ok(vec![8]), ("DE", "HL") => Ok(vec![0xeb]),
                ("(SP)", _) => {
                    let (_, prefix) = pair(b).filter(|(r, _)| *r == 2).ok_or_else(|| self.fail("EX (SP) requires HL, IX or IY"))?;
                    Ok(prefixed(prefix, [0xe3]))
                }
                _ => Err(self.fail("Unsupported EX operands")),
            },
            "JP" | "CALL" if (1..=2).contains(&operands.len()) => {
                if op == "JP" && operands.len() == 1 && let Some(inner) = indirect(a) {
                    let (_, prefix) = pair(inner).filter(|(r, _)| *r == 2).ok_or_else(|| self.fail("Indirect JP requires HL, IX or IY"))?;
                    return Ok(prefixed(prefix, [0xe9]));
                }
                let opcode = if operands.len() == 1 { if op == "JP" { 0xc3 } else { 0xcd } }
                    else { (if op == "JP" { 0xc2 } else { 0xc4 }) + condition(a).ok_or_else(|| self.fail("Invalid condition"))? * 8 };
                let address = self.address(if operands.len() == 1 { a } else { b })?.to_le_bytes();
                Ok(vec![opcode, address[0], address[1]])
            }
            "JR" | "DJNZ" if (1..=2).contains(&operands.len()) => {
                let opcode = if op == "DJNZ" && operands.len() == 1 { 0x10 }
                    else if op == "JR" && operands.len() == 1 { 0x18 }
                    else if op == "JR" { 0x20 + condition(a).filter(|c| *c < 4).ok_or_else(|| self.fail("JR requires NZ, Z, NC or C"))? * 8 }
                    else { return Err(self.fail("DJNZ requires one destination")); };
                Ok(vec![opcode, self.relative(if operands.len() == 1 { a } else { b })?])
            }
            "RET" if operands.len() <= 1 => Ok(vec![if operands.is_empty() { 0xc9 }
                else { 0xc0 + condition(a).ok_or_else(|| self.fail("Invalid return condition"))? * 8 }]),
            "RST" if operands.len() == 1 => {
                let address = self.value(a)?;
                if !(0..=56).contains(&address) || address % 8 != 0 { return Err(self.fail("RST requires 0, 8, 16, 24, 32, 40, 48 or 56")); }
                Ok(vec![0xc7 + address as u8])
            }
            "IM" if operands.len() == 1 => {
                let mode = self.value(a)?;
                match mode { 0 => Ok(vec![0xed, 0x46]), 1 => Ok(vec![0xed, 0x56]), 2 => Ok(vec![0xed, 0x5e]), _ => Err(self.fail("IM requires 0, 1 or 2")) }
            }
            "IN" | "OUT" => self.io(op, &operands),
            "BIT" | "RES" | "SET" if (2..=3).contains(&operands.len()) => {
                let bit = self.value(a)?;
                if self.final_pass && !(0..=7).contains(&bit) { return Err(self.fail("Bit number must be 0..7")); }
                self.cb(op, bit as u8 & 7, b, operands.get(2).map(String::as_str))
            }
            "RLC" | "RRC" | "RL" | "RR" | "SLA" | "SRA" | "SLL" | "SRL" if (1..=2).contains(&operands.len()) => {
                let group = match op { "RLC" => 0, "RRC" => 1, "RL" => 2, "RR" => 3, "SLA" => 4, "SRA" => 5, "SLL" => 6, _ => 7 };
                self.cb(op, group, a, operands.get(1).map(String::as_str))
            }
            _ => Err(self.fail(format!("Unsupported instruction or operands: {op}; includes, macros and scripting are unavailable"))),
        }
    }
    fn cb(
        &self,
        op: &str,
        operation: u8,
        text: &str,
        copy: Option<&str>,
    ) -> Result<Vec<u8>, AssemblyError> {
        let operand = self
            .reg8(text)?
            .ok_or_else(|| self.fail("Expected byte register or memory operand"))?;
        if operand.prefix != 0 && operand.code != 6 {
            return Err(self.fail("CB operations do not support index halves"));
        }
        let group = match op {
            "BIT" => 1,
            "RES" => 2,
            "SET" => 3,
            _ => 0,
        };
        let register = if let Some(copy) = copy {
            if operand.displacement.is_none() || group == 1 {
                return Err(
                    self.fail("Register-copy form requires an indexed memory shift, RES or SET")
                );
            }
            self.reg8(copy)?
                .filter(|r| r.code != 6 && r.prefix == 0)
                .ok_or_else(|| self.fail("Invalid indexed register-copy destination"))?
                .code
        } else {
            operand.code
        };
        let opcode = group * 64 + operation * 8 + register;
        Ok(if let Some(displacement) = operand.displacement {
            vec![operand.prefix, 0xcb, displacement, opcode]
        } else {
            vec![0xcb, opcode]
        })
    }
    fn io(&self, op: &str, operands: &[String]) -> Result<Vec<u8>, AssemblyError> {
        if op == "IN" && operands.len() == 1 && operands[0] == "(C)" {
            return Ok(vec![0xed, 0x70]);
        }
        let [a, b] = operands else {
            return Err(self.fail("IN/OUT requires two operands"));
        };
        let (register, port) = if op == "IN" {
            (a.as_str(), b.as_str())
        } else {
            (b.as_str(), a.as_str())
        };
        let inner = indirect(port).ok_or_else(|| self.fail("I/O port must be parenthesised"))?;
        if inner == "C" {
            let register = if op == "OUT" && register == "0" {
                6
            } else {
                self.reg8(register)?
                    .filter(|r| r.code != 6 && r.prefix == 0)
                    .ok_or_else(|| self.fail("Invalid I/O register"))?
                    .code
            };
            Ok(vec![
                0xed,
                (if op == "IN" { 0x40 } else { 0x41 }) + register * 8,
            ])
        } else if register == "A" {
            Ok(vec![
                if op == "IN" { 0xdb } else { 0xd3 },
                self.byte(inner)?,
            ])
        } else {
            Err(self.fail("Immediate-port I/O requires A"))
        }
    }
}
