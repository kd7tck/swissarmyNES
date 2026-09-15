// KEEP
use rs6502::OpCode;
use std::collections::{BTreeMap, HashMap, HashSet};

/// Locations are resolved by the same pass that emits the final ROM bytes.
#[derive(Clone, Debug)]
pub struct Location {
    pub bank: u8,
    pub address: u16,
    pub length: usize,
    pub assembly_line: usize,
    pub source_file: String,
    pub source_line: usize,
    pub executable: bool,
}

pub struct Assembler;
struct Instruction {
    bank: u8,
    source_bank: u8,
    address: u16,
    line: usize,
    operation: String,
    operand: String,
    opcode: Option<OpCode>,
    length: usize,
    source_file: String,
    source_line: usize,
}

/// rs6502 supplies the opcode catalog. Our two-pass linker also handles data
/// directives, which its assembler silently discards, and physical PRG banks.
impl Assembler {
    pub fn new() -> Self {
        Self
    }
    pub fn assemble(
        &self,
        asm: &[String],
        chr: Option<&[u8]>,
        injections: Vec<(u16, Vec<u8>)>,
    ) -> Result<Vec<u8>, String> {
        self.assemble_banks(&HashMap::from([(0, asm.to_vec())]), chr, injections)
    }
    pub fn assemble_banks(
        &self,
        banks: &HashMap<u8, Vec<String>>,
        chr: Option<&[u8]>,
        injections: Vec<(u16, Vec<u8>)>,
    ) -> Result<Vec<u8>, String> {
        self.assemble_with_layout(banks, chr, injections)
            .map(|(rom, _)| rom)
    }
    pub fn assemble_with_layout(
        &self,
        banks: &HashMap<u8, Vec<String>>,
        chr: Option<&[u8]>,
        injections: Vec<(u16, Vec<u8>)>,
    ) -> Result<(Vec<u8>, Vec<Location>), String> {
        if chr.is_some_and(|data| data.len() > 8192) {
            return Err("CHR data exceeds 8192 bytes".into());
        }
        let mut symbols = BTreeMap::new();
        let mut instructions = Vec::new();
        let mut long_branches = HashSet::new();
        // Monotonic relaxation preserves short timing where possible and resolves
        // long branches before deriving the source map or placing any bytes.
        loop {
            symbols.clear();
            instructions.clear();
            let ordered: BTreeMap<_, _> = banks.iter().collect();
            for (&bank, lines) in ordered {
                if bank > 7 {
                    return Err(format!("Invalid PRG bank {bank}; expected 0..7"));
                }
                let mut pc = if bank == 7 { 0xc000u32 } else { 0x8000u32 };
                let mut placement_bank = bank;
                let mut source_file = String::new();
                let mut source_line = 0;
                for (line, raw) in lines.iter().enumerate() {
                    if let Some(marker) = raw.trim().strip_prefix(";@source ") {
                        let marker: serde_json::Value = serde_json::from_str(marker)
                            .map_err(|error| format!("Invalid source marker: {error}"))?;
                        source_file = marker["file"].as_str().unwrap_or("").to_string();
                        source_line = marker["line"].as_u64().unwrap_or(0) as usize;
                    }
                    let text = raw.split(';').next().unwrap_or("").trim();
                    if text.is_empty() {
                        continue;
                    }
                    let text = if let Some((label, rest)) = text.split_once(':') {
                        if pc > 0xffff {
                            return Err(format!("Label {label} exceeds CPU address space"));
                        }
                        if symbols
                            .insert(label.trim().to_string(), pc as i64)
                            .is_some()
                        {
                            return Err(format!("Duplicate label {label}"));
                        }
                        rest.trim()
                    } else {
                        text
                    };
                    if text.is_empty() {
                        continue;
                    }
                    if let Some((name, value)) = text.split_once('=') {
                        let value = expression(value.trim(), &symbols)?;
                        if symbols.insert(name.trim().to_string(), value).is_some() {
                            return Err(format!("Duplicate symbol {name}"));
                        }
                        continue;
                    }
                    let (operation, operand) =
                        text.split_once(char::is_whitespace).unwrap_or((text, ""));
                    let operation = operation.to_uppercase();
                    let operand = operand.split_whitespace().collect::<String>();
                    if operation == ".ORG" {
                        let value = expression(&operand, &symbols)?;
                        if !(0x8000..=0xffff).contains(&value) {
                            return Err(format!("ORG {value:X} outside PRG-ROM"));
                        }
                        pc = value as u32;
                        if bank != 0 && bank != 7 && pc >= 0xc000 {
                            return Err(format!("ORG {pc:04X} outside switchable bank {bank}"));
                        }
                        placement_bank = if pc >= 0xc000 { 7 } else { bank };
                        continue;
                    }
                    let opcode = if matches!(operation.as_str(), "DB" | ".BYTE" | "WORD" | ".WORD")
                    {
                        None
                    } else {
                        Some(
                            opcode_for(&operation, &operand)
                                .map_err(|e| format!("Bank {bank}, line {}: {e}", line + 1))?,
                        )
                    };
                    let length = if long_branches.contains(&(bank, line)) {
                        5
                    } else {
                        opcode.map_or_else(
                            || {
                                operand.split(',').count()
                                    * if operation.ends_with("WORD") { 2 } else { 1 }
                            },
                            |op| op.length as usize,
                        )
                    };
                    let effective_bank = placement_bank;
                    if bank == 7 && pc < 0xc000 {
                        return Err("Fixed bank code must start at $C000 or above".into());
                    }
                    let limit = if effective_bank == 7 { 0x10000 } else { 0xc000 };
                    if pc.checked_add(length as u32).is_none_or(|end| end > limit) {
                        return Err(format!(
                            "Code in bank {bank} at {pc:04X} exceeds bank window"
                        ));
                    }
                    instructions.push(Instruction {
                        source_file: source_file.clone(),
                        source_line,
                        bank: effective_bank,
                        source_bank: bank,
                        address: pc as u16,
                        line,
                        operation,
                        operand,
                        opcode,
                        length,
                    });
                    pc += length as u32;
                }
            }
            let mut expanded = false;
            for instruction in &instructions {
                if is_branch(&instruction.operation) && instruction.length == 2 {
                    let target = expression(&instruction.operand, &symbols)?;
                    if !(-128..=127).contains(&(target - i64::from(instruction.address) - 2)) {
                        long_branches.insert((instruction.source_bank, instruction.line));
                        expanded = true;
                    }
                }
            }
            if !expanded {
                break;
            }
        }
        let mut prg = vec![0; 131072];
        let mut occupied = vec![false; prg.len()];
        let mut layout = Vec::new();
        for instruction in instructions {
            let mut data = Vec::new();
            if let Some(opcode) = instruction.opcode {
                if is_branch(&instruction.operation) && instruction.length == 5 {
                    let target = expression(&instruction.operand, &symbols)?;
                    if !(0..=65535).contains(&target) {
                        return Err("Branch target out of range".into());
                    }
                    data.extend([
                        opcode.code ^ 0x20,
                        3,
                        0x4c,
                        target as u8,
                        (target >> 8) as u8,
                    ]);
                } else {
                    data.push(opcode.code);
                    if opcode.length > 1 {
                        let operand = instruction
                            .operand
                            .trim_start_matches('#')
                            .replace(['(', ')'], "");
                        let operand = operand
                            .strip_suffix(",X")
                            .or_else(|| operand.strip_suffix(",Y"))
                            .or_else(|| operand.strip_suffix(",x"))
                            .or_else(|| operand.strip_suffix(",y"))
                            .unwrap_or(&operand);
                        let value = expression(operand, &symbols)?;
                        let value = if is_branch(&instruction.operation) {
                            let displacement = value - (i64::from(instruction.address) + 2);
                            if !(-128..=127).contains(&displacement) {
                                return Err(format!(
                                    "Branch too far at {:04X}: {}",
                                    instruction.address, instruction.operand
                                ));
                            }
                            displacement & 255
                        } else {
                            value
                        };
                        let max = if opcode.length == 2 { 255 } else { 65535 };
                        if !(0..=max).contains(&value) {
                            return Err(format!(
                                "Operand {} outside 0..{max}",
                                instruction.operand
                            ));
                        }
                        data.push(value as u8);
                        if opcode.length == 3 {
                            data.push((value >> 8) as u8);
                        }
                    }
                }
            } else {
                let word = instruction.operation.ends_with("WORD");
                for item in instruction.operand.split(',') {
                    let value = expression(item.trim_start_matches('#'), &symbols)?;
                    let range = if word { -32768..=65535 } else { -128..=255 };
                    if !range.contains(&value) {
                        return Err(format!("Data value {value} out of range"));
                    }
                    data.push(value as u8);
                    if word {
                        data.push((value >> 8) as u8);
                    }
                }
            }
            place(
                &mut prg,
                &mut occupied,
                instruction.bank,
                instruction.address,
                &data,
            )?;
            layout.push(Location {
                source_file: instruction.source_file,
                source_line: instruction.source_line,
                executable: instruction.opcode.is_some(),
                bank: instruction.bank,
                address: instruction.address,
                length: instruction.length,
                assembly_line: instruction.line,
            });
        }
        for (address, data) in injections {
            place(
                &mut prg,
                &mut occupied,
                if address >= 0xc000 { 7 } else { 0 },
                address,
                &data,
            )?;
        }
        let mut rom = vec![
            0x4e, 0x45, 0x53, 0x1a, 8, 1, 0x10, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        rom.extend(prg);
        let start = rom.len();
        rom.resize(start + 8192, 0);
        if let Some(chr) = chr {
            rom[start..start + chr.len()].copy_from_slice(chr);
        }
        Ok((rom, layout))
    }
}
fn place(
    prg: &mut [u8],
    occupied: &mut [bool],
    bank: u8,
    address: u16,
    data: &[u8],
) -> Result<(), String> {
    let base = if bank == 7 { 0xc000usize } else { 0x8000usize };
    let address = usize::from(address);
    if bank > 7
        || address < base
        || address
            .checked_add(data.len())
            .is_none_or(|end| end > base + 16384)
    {
        return Err(format!(
            "Segment at {address:04X} exceeds bank {bank} window"
        ));
    }
    let start = usize::from(bank) * 16384 + address - base;
    let end = start + data.len();
    if occupied[start..end].iter().any(|used| *used) {
        return Err(format!(
            "Segment in bank {bank} at {address:04X} overlaps existing data"
        ));
    }
    prg[start..end].copy_from_slice(data);
    occupied[start..end].fill(true);
    Ok(())
}
fn is_branch(operation: &str) -> bool {
    matches!(
        operation,
        "BCC" | "BCS" | "BEQ" | "BNE" | "BMI" | "BPL" | "BVC" | "BVS"
    )
}
fn opcode_for(operation: &str, operand: &str) -> Result<OpCode, String> {
    let operand = operand.to_uppercase();
    let short = operand
        .strip_prefix('$')
        .is_some_and(|rest| rest.split(',').next().unwrap_or("").len() <= 2);
    // Representative opcodes provide mode values without copying rs6502's table.
    let representative = if is_branch(operation) {
        0xd0
    } else if operand.is_empty() {
        if matches!(operation, "ASL" | "LSR" | "ROL" | "ROR") {
            0x0a
        } else {
            0xea
        }
    } else if operand == "A" && matches!(operation, "ASL" | "LSR" | "ROL" | "ROR") {
        0x0a
    } else if operand.starts_with('#') {
        0xa9
    } else if operand.starts_with('(') {
        if operand.ends_with(",X)") {
            0xa1
        } else if operand.ends_with("),Y") {
            0xb1
        } else {
            0x6c
        }
    } else if operand.ends_with(",X") {
        if short {
            0xb5
        } else {
            0xbd
        }
    } else if operand.ends_with(",Y") {
        if short {
            0xb6
        } else {
            0xb9
        }
    } else if short {
        0xa5
    } else {
        0xad
    };
    let mode = OpCode::from_raw_byte(representative).unwrap().mode;
    OpCode::from_mnemonic_and_addressing_mode(operation, mode)
        .ok_or_else(|| format!("Invalid instruction {operation} {operand}"))
}
fn expression(text: &str, symbols: &BTreeMap<String, i64>) -> Result<i64, String> {
    let text = text.trim();
    if let Some(rest) = text.strip_prefix('<') {
        return Ok(expression(rest, symbols)? & 255);
    }
    if let Some(rest) = text.strip_prefix('>') {
        return Ok((expression(rest, symbols)? >> 8) & 255);
    }
    if let Some(index) = text
        .char_indices()
        .skip(1)
        .filter_map(|(i, c)| matches!(c, '+' | '-').then_some(i))
        .last()
    {
        let lhs = expression(&text[..index], symbols)?;
        let rhs = expression(&text[index + 1..], symbols)?;
        return if &text[index..index + 1] == "+" {
            lhs.checked_add(rhs)
        } else {
            lhs.checked_sub(rhs)
        }
        .ok_or_else(|| "Assembly expression overflow".into());
    }
    if let Some(value) = symbols.get(text) {
        return Ok(*value);
    }
    let value = if let Some(hex) = text.strip_prefix('$') {
        i64::from_str_radix(hex, 16)
    } else if let Some(binary) = text.strip_prefix('%') {
        i64::from_str_radix(binary, 2)
    } else {
        text.parse()
    };
    value.map_err(|_| format!("Unknown symbol or invalid value: {text}"))
}
impl Default for Assembler {
    fn default() -> Self {
        Self::new()
    }
}
