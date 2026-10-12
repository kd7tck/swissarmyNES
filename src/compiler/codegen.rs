// KEEP
use crate::compiler::ast::{
    BinaryOperator, DataType, Expression, Program, Statement, StatementKind, TopLevel,
    TopLevelKind, UnaryOperator,
};
use crate::compiler::source_map::AssetReuseStats;
use crate::compiler::symbol_table::{SymbolKind, SymbolTable};
use std::collections::HashMap;

pub const NAMETABLE_ADDR: u16 = 0xD500;
pub const SFX_TABLE_ADDR: u16 = 0xD900;
pub const ENVELOPE_TABLE_ADDR: u16 = 0xDA00;
const SOUND_RAM_START: u16 = 0x0300;
const VBLANK_BUFFER_START: u16 = 0x0380;
const STRING_HEAP_START: u16 = 0x03C0;
const VAR_START_RAM: u16 = 0x05C0;

pub type Assembly = Vec<String>;
pub type SourceMap = Vec<(usize, u16)>;
type AnimationDataKey = (bool, Vec<(String, u8)>);

// KEEP
pub struct CodeGenerator {
    pub asset_reuse: AssetReuseStats,
    symbol_table: SymbolTable,
    output: Assembly,
    banks: std::collections::HashMap<u8, Assembly>,
    current_bank: u8,
    pc_per_bank: std::collections::HashMap<u8, u16>,
    #[allow(dead_code)]
    sourcemap: SourceMap,
    #[allow(dead_code)]
    pc: u16,
    current_line: usize,
    ram_pointer: u16,
    label_counter: usize,
    data_table_offsets: HashMap<String, u16>,
    sub_signatures: HashMap<String, (u8, Vec<(u16, DataType)>)>,
    interrupt_targets: Vec<(String, String)>,
    string_literals: HashMap<String, String>,
    select_stack_depth: usize,
}

impl CodeGenerator {
    fn generate_interrupt_scratch(&mut self, event: &str, restore: bool) {
        // Save only transient bytes. Persistent controller, sprite, text,
        // scroll and RNG state intentionally remains visible after a handler.
        let action = if restore { "Restore" } else { "Save" };
        let label = format!("{action}_{event}_Scratch");
        self.emit(format!("{label}:"));
        // Direct zero-page accesses recover over 600 CPU cycles compared with
        // two indexed loops, leaving room for active audio and seam uploads.
        let mut addresses: Vec<u8> = (0..16).chain(0x14..=0x17).chain(0xf0..=0xf7).collect();
        if restore {
            addresses.reverse();
        }
        for address in addresses {
            if restore {
                self.emit("  PLA".into());
                self.emit(format!("  STA ${address:02X}"));
            } else {
                self.emit(format!("  LDA ${address:02X}"));
                self.emit("  PHA".into());
            }
        }
    }
    fn collect_interrupt_bindings(
        block: &[Statement],
        targets: &mut std::collections::BTreeSet<String>,
    ) {
        for statement in block {
            match &statement.kind {
                StatementKind::On(_, handler) => {
                    targets.insert(handler.clone());
                }
                StatementKind::If(_, then_block, else_block) => {
                    Self::collect_interrupt_bindings(then_block, targets);
                    if let Some(block) = else_block {
                        Self::collect_interrupt_bindings(block, targets);
                    }
                }
                StatementKind::While(_, body)
                | StatementKind::DoWhile(body, _)
                | StatementKind::For(_, _, _, _, body) => {
                    Self::collect_interrupt_bindings(body, targets)
                }
                StatementKind::Select(_, cases, otherwise) => {
                    for (_, body) in cases {
                        Self::collect_interrupt_bindings(body, targets);
                    }
                    if let Some(body) = otherwise {
                        Self::collect_interrupt_bindings(body, targets);
                    }
                }
                _ => {}
            }
        }
    }

    fn get_pc(&self) -> u16 {
        *self.pc_per_bank.get(&self.current_bank).unwrap_or(&0x8000)
    }

    fn set_pc(&mut self, addr: u16) {
        self.pc_per_bank.insert(self.current_bank, addr);
    }

    fn switch_bank(&mut self, new_bank: u8) {
        if self.current_bank == new_bank {
            return;
        }
        self.banks.insert(self.current_bank, self.output.clone());
        let current_pc = self.get_pc();
        self.pc_per_bank.insert(self.current_bank, current_pc);
        self.current_bank = new_bank;
        if let Some(existing) = self.banks.get(&new_bank) {
            self.output = existing.clone();
        } else {
            self.output.clear();
        }
    }

    pub fn generate_banks(
        &mut self,
        program: &Program,
    ) -> Result<(std::collections::HashMap<u8, Assembly>, SourceMap), String> {
        self.banks.clear();
        self.output.clear();
        self.sourcemap.clear();
        self.pc_per_bank.clear();
        self.string_literals.clear();
        self.data_table_offsets.clear();
        self.sub_signatures.clear();
        self.interrupt_targets.clear();
        let mut bound_handlers = std::collections::BTreeSet::new();
        for declaration in &program.declarations {
            if let TopLevelKind::Sub(_, _, body) | TopLevelKind::Interrupt(_, body) =
                &declaration.kind
            {
                Self::collect_interrupt_bindings(body, &mut bound_handlers);
            }
        }
        // Selector zero retains the default vector. Other selectors identify
        // immutable linked addresses, so ON publishes a binding in one store.
        for declaration in &program.declarations {
            match &declaration.kind {
                TopLevelKind::Sub(name, params, _)
                    if params.is_empty() && bound_handlers.contains(name) =>
                {
                    self.interrupt_targets
                        .push((name.clone(), format!("Trampoline_{name}")));
                }
                TopLevelKind::Interrupt(name, _) if bound_handlers.contains(name) => {
                    self.interrupt_targets.push((name.clone(), name.clone()));
                }
                _ => {}
            }
        }
        if self.interrupt_targets.len() > 255 {
            return Err("Interrupt dispatch supports at most 255 distinct bound handlers".into());
        }
        self.label_counter = 0;
        self.current_line = 0;
        self.current_bank = 0;
        self.set_pc(0x8000);
        self.emit(".ORG $8000".to_string());
        self.emit("; Generated by SwissArmyNES".to_string());

        self.allocate_memory(program)?;
        // Reset and interrupt entry must stay mapped when the guest switches PRG.
        self.switch_bank(7);
        self.emit(".ORG $C000".to_string());
        self.generate_startup_routine(program)?;
        self.switch_bank(0);

        for decl in &program.declarations {
            {
                self.current_line = decl.line;
                if let TopLevelKind::Bank(b) = decl.kind {
                    if b > 7 {
                        return Err(format!("Invalid PRG bank {b}; expected 0..7"));
                    }
                    self.switch_bank(b);
                    if self.get_pc() == 0x8000 && self.output.is_empty() {
                        self.emit(".ORG $8000".to_string()); // Ensure new banks start at $8000
                    }
                }
                let caller_bank = self.current_bank;
                if matches!(decl.kind, TopLevelKind::Interrupt(_, _)) {
                    self.switch_bank(7);
                }
                self.generate_top_level(decl)?;
                self.switch_bank(caller_bank);
            }
        }

        self.switch_bank(7);
        self.current_line = 0;
        self.output
            .push(";@source {\"file\":\"\",\"line\":0}".to_string());
        if self.get_pc() == 0x8000 && self.output.is_empty() {
            self.set_pc(0xC000);
            self.emit(".ORG $C000".to_string()); // Bank 7 starts at $C000
        }

        // MMC1 BankSwitch Routine
        // Expects the target bank in A register.
        // Interrupts may use the same serial mapper register. They flag the
        // interrupted sequence for a full reset/retry before switchable code runs.
        self.emit("MMC1_SetPrgBank:".to_string());
        self.emit("  STA $07F0".to_string());
        self.emit("MMC1_RetryPrgBank:".to_string());
        self.emit("  LDA #0".to_string());
        self.emit("  STA $07F2".to_string());
        self.emit("  LDA #$80".to_string());
        self.emit("  STA $8000".to_string());
        self.emit("  LDA $07F0".to_string());
        self.emit("  STA $E000".to_string());
        self.emit("  LSR".to_string());
        self.emit("  STA $E000".to_string());
        self.emit("  LSR".to_string());
        self.emit("  STA $E000".to_string());
        self.emit("  LSR".to_string());
        self.emit("  STA $E000".to_string());
        self.emit("  LSR".to_string());
        self.emit("  STA $E000".to_string());
        self.emit("  LDA $07F2".to_string());
        self.emit("  BNE MMC1_RetryPrgBank".to_string());
        self.emit("  RTS".to_string());
        self.generate_trampolines(program);

        let runtime_start = self.output.len();
        self.generate_sound_engine();
        self.generate_math_helpers();
        self.generate_string_helpers();
        self.generate_string_data();
        self.generate_controller_helpers();
        self.generate_text_helpers();
        self.generate_sprite_helpers();
        self.generate_animation_helpers();
        self.generate_pool_helpers();
        self.generate_collision_helpers();
        self.generate_scroll_helpers();
        for line in crate::compiler::world::runtime_assembly() {
            self.emit(line);
        }
        self.retain_referenced_runtime(runtime_start);
        self.generate_user_data(program)?;
        self.generate_data_tables(program)?;
        self.generate_vectors(program)?;

        self.banks.insert(self.current_bank, self.output.clone());

        Ok((self.banks.clone(), self.sourcemap.clone()))
    }

    pub fn new(symbol_table: SymbolTable) -> Self {
        Self {
            asset_reuse: Default::default(),
            symbol_table,
            output: Vec::new(),
            banks: std::collections::HashMap::new(),
            current_bank: 0,
            pc_per_bank: std::collections::HashMap::new(),
            sourcemap: Vec::new(),
            pc: 0x8000,
            current_line: 0,
            ram_pointer: 0x0000,
            label_counter: 0,
            data_table_offsets: HashMap::new(),
            sub_signatures: HashMap::new(),
            interrupt_targets: Vec::new(),
            string_literals: HashMap::new(),
            select_stack_depth: 0,
        }
    }

    /// Keep runtime instructions reachable from guest/startup assembly and
    /// transitive helper references. The fixed bank also contains audio/assets,
    /// so emitting every unused helper can overlap the first data segment.
    /// Symbolic operands (including inline ASM addresses) count as roots; data
    /// directives stay intact. The assembler computes final addresses afterward.
    fn retain_referenced_runtime(&mut self, start: usize) {
        use std::collections::{HashSet, VecDeque};
        let labels: HashMap<String, usize> = self.output[start..]
            .iter()
            .enumerate()
            .filter_map(|(offset, line)| {
                line.trim()
                    .strip_suffix(':')
                    .map(|label| (label.to_string(), start + offset))
            })
            .collect();
        let references = |line: &str| -> Vec<usize> {
            line.split(';')
                .next()
                .unwrap_or("")
                .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
                .filter_map(|word| labels.get(word).copied())
                .collect()
        };
        let mut queue = VecDeque::new();
        for line in self.output[..start]
            .iter()
            .chain(self.banks.values().flatten())
        {
            queue.extend(references(line));
        }
        // String literals are interspersed with helpers and must survive even
        // when their address is consumed through an indirect data-table entry.
        for (index, line) in self.output.iter().enumerate().skip(start) {
            let op = line.split_whitespace().next().unwrap_or("");
            if op.eq_ignore_ascii_case("DB") || op.eq_ignore_ascii_case("WORD") {
                queue.push_back(index);
                queue.extend(references(line));
                if index > start && self.output[index - 1].trim().ends_with(':') {
                    queue.push_back(index - 1);
                }
            }
        }
        let mut reachable = HashSet::new();
        while let Some(index) = queue.pop_front() {
            if index >= self.output.len() || !reachable.insert(index) {
                continue;
            }
            let line = self.output[index].split(';').next().unwrap_or("").trim();
            let op = line.split_whitespace().next().unwrap_or("").to_uppercase();
            queue.extend(references(line));
            if !matches!(op.as_str(), "RTS" | "RTI" | "JMP") {
                queue.push_back(index + 1);
            }
        }
        let removed_bytes: u16 = self
            .output
            .iter()
            .enumerate()
            .skip(start)
            .filter(|(index, line)| {
                !reachable.contains(index)
                    && !line.trim().is_empty()
                    && !line.trim().starts_with(';')
                    && !line.trim().starts_with('.')
                    && !line.trim().ends_with(':')
            })
            .map(|(_, line)| self.estimate_size(line.trim()))
            .sum();
        let mut index = 0;
        self.output.retain(|line| {
            let keep = index < start
                || reachable.contains(&index)
                || line.trim().is_empty()
                || line.trim().starts_with(';')
                || line.trim().starts_with('.');
            index += 1;
            keep
        });
        self.set_pc(self.get_pc().wrapping_sub(removed_bytes));
    }

    fn emit(&mut self, code: String) {
        let trimmed = code.trim();
        if !trimmed.ends_with(':') && !trimmed.starts_with('.') && !trimmed.starts_with(';') {
            let size = self.estimate_size(trimmed);
            if self.current_line > 0 {
                if let Some(last) = self.sourcemap.last() {
                    if last.0 != self.current_line {
                        self.sourcemap.push((self.current_line, self.get_pc()));
                    }
                } else {
                    self.sourcemap.push((self.current_line, self.get_pc()));
                }
            }
            let current_pc = self.get_pc();
            self.set_pc(current_pc.wrapping_add(size));
        } else if trimmed.starts_with(".ORG") {
            if let Some(parts) = trimmed.split_whitespace().nth(1) {
                let addr_str = parts.replace("$", "");
                if let Ok(addr) = u16::from_str_radix(&addr_str, 16) {
                    self.set_pc(addr);
                }
            }
        }
        self.output.push(code);
    }

    fn estimate_size(&self, code: &str) -> u16 {
        let parts: Vec<&str> = code.split_whitespace().collect();
        if parts.is_empty() {
            return 0;
        }

        if parts.len() == 1 {
            return 1;
        }

        let operand = parts[1..].join("");

        if operand.starts_with('#') {
            return 2;
        }
        if operand.starts_with("($") {
            // JMP Indirect is 3 bytes, but (Indirect),Y or (Indirect,X) are 2 bytes
            if parts[0].eq_ignore_ascii_case("JMP") {
                return 3;
            } else {
                return 2;
            }
        }

        if operand.starts_with('$') {
            let hex_part = operand
                .trim_start_matches('$')
                .split(',')
                .next()
                .unwrap_or("");
            if hex_part.len() > 2 {
                return 3;
            } else {
                return 2;
            }
        }

        let opcode = parts[0].to_uppercase();
        match opcode.as_str() {
            "JMP" | "JSR" => 3,
            "BNE" | "BEQ" | "BPL" | "BMI" | "BVC" | "BVS" | "BCC" | "BCS" => 2,
            "DB" => {
                // Count bytes in operand (comma separated)
                operand.split(',').count() as u16
            }
            "WORD" => 2, // Explicit WORD is always 2 bytes
            _ => 3,
        }
    }

    fn new_label(&mut self) -> String {
        self.label_counter += 1;
        format!("GEN_L{}", self.label_counter)
    }

    /// Compatibility entry point for programs using only bank zero and the fixed bank.
    /// All runtime generation goes through the production bank-aware path.
    pub fn generate(&mut self, program: &Program) -> Result<(Assembly, SourceMap), String> {
        let (mut banks, source_map) = self.generate_banks(program)?;
        if banks.keys().any(|bank| *bank != 0 && *bank != 7) {
            return Err("Multiple switchable banks require generate_banks".into());
        }
        let mut assembly = banks.remove(&0).unwrap_or_default();
        assembly.extend(banks.remove(&7).unwrap_or_default());
        Ok((assembly, source_map))
    }
    fn get_type_size(&self, dt: &DataType) -> Result<u16, String> {
        match dt {
            DataType::Byte | DataType::Int | DataType::Bool | DataType::Enum(_) => Ok(1),
            DataType::Word | DataType::String => Ok(2),
            DataType::Struct(name) => self
                .symbol_table
                .resolve(name)
                .and_then(|symbol| symbol.value)
                .and_then(|size| u16::try_from(size).ok())
                .ok_or_else(|| format!("Invalid size for struct {name}")),
            DataType::Array(inner, count) => {
                let count =
                    u16::try_from(*count).map_err(|_| "Array size exceeds address space")?;
                self.get_type_size(inner)?
                    .checked_mul(count)
                    .ok_or_else(|| "Array size exceeds address space".to_string())
            }
        }
    }
    fn generate_startup_routine(&mut self, program: &Program) -> Result<(), String> {
        self.emit("Startup:".to_string());
        self.emit("  SEI".to_string());
        self.emit("  CLD".to_string());
        // MMC1 serial reset selects a fixed upper bank. Establish bank zero
        // before touching data or invoking any switchable user code.
        self.emit("  LDA #$80".to_string());
        self.emit("  STA $8000".to_string());
        self.emit("  LDA #$0C".to_string());
        for _ in 0..5 {
            self.emit("  STA $8000".to_string());
            self.emit("  LSR".to_string());
        }
        self.emit("  LDA #0".to_string());
        self.emit("  STA $07F0".to_string());
        for _ in 0..5 {
            self.emit("  STA $E000".to_string());
        }
        self.emit("  LDX #$40".to_string());
        self.emit("  STX $4017".to_string());
        self.emit("  LDX #$FF".to_string());
        self.emit("  TXS".to_string());
        self.emit("  INX".to_string());
        self.emit("  STX $2000".to_string());
        self.emit("  STX $2001".to_string());
        self.emit("  STX $4010".to_string());

        self.emit("vblankwait1:".to_string());
        self.emit("  BIT $2002".to_string());
        self.emit("  BPL vblankwait1".to_string());

        self.emit("clrmem:".to_string());
        self.emit("  LDA #$00".to_string());
        self.emit("  STA $00,x".to_string());
        self.emit("  STA $0100,x".to_string());
        self.emit("  STA $0300,x".to_string());
        self.emit("  STA $0400,x".to_string());
        self.emit("  STA $0500,x".to_string());
        self.emit("  STA $0600,x".to_string());
        self.emit("  STA $0700,x".to_string());
        self.emit("  STA $0320,x".to_string()); // Clear extra sound ram/string heap gap
        self.emit("  LDA #$FE".to_string());
        self.emit("  STA $0200,x".to_string());
        self.emit("  INX".to_string());
        self.emit("  BNE clrmem".to_string());

        self.emit("  LDA #0".to_string());
        self.emit("  STA $18".to_string()); // Init Text Offset

        self.emit("vblankwait2:".to_string());
        self.emit("  BIT $2002".to_string());
        self.emit("  BPL vblankwait2".to_string());

        self.emit("  LDA #$3F".to_string());
        self.emit("  STA $2006".to_string());
        self.emit("  LDA #$00".to_string());
        self.emit("  STA $2006".to_string());

        self.emit("  LDX #$00".to_string());
        self.emit("LoadPalLoop:".to_string());
        self.emit("  LDA $E000, X".to_string());
        self.emit("  STA $2007".to_string());
        self.emit("  INX".to_string());
        self.emit("  CPX #32".to_string());
        self.emit("  BNE LoadPalLoop".to_string());

        self.emit("  LDA #$20".to_string());
        self.emit("  STA $2006".to_string());
        self.emit("  LDA #$00".to_string());
        self.emit("  STA $2006".to_string());

        self.emit("  LDX #$00".to_string());
        self.emit("LoadNT1:".to_string());
        self.output
            .push(format!("  LDA ${:04X}, X", NAMETABLE_ADDR));
        self.emit("  STA $2007".to_string());
        self.emit("  INX".to_string());
        self.emit("  BNE LoadNT1".to_string());

        self.emit("  LDX #$00".to_string());
        self.emit("LoadNT2:".to_string());
        self.output
            .push(format!("  LDA ${:04X}, X", NAMETABLE_ADDR + 256));
        self.emit("  STA $2007".to_string());
        self.emit("  INX".to_string());
        self.emit("  BNE LoadNT2".to_string());

        self.emit("  LDX #$00".to_string());
        self.emit("LoadNT3:".to_string());
        self.output
            .push(format!("  LDA ${:04X}, X", NAMETABLE_ADDR + 512));
        self.emit("  STA $2007".to_string());
        self.emit("  INX".to_string());
        self.emit("  BNE LoadNT3".to_string());

        self.emit("  LDX #$00".to_string());
        self.emit("LoadNT4:".to_string());
        self.output
            .push(format!("  LDA ${:04X}, X", NAMETABLE_ADDR + 768));
        self.emit("  STA $2007".to_string());
        self.emit("  INX".to_string());
        self.emit("  BNE LoadNT4".to_string());

        self.emit("  JSR Sound_Init".to_string());

        let user_data_addr = self.data_table_offsets.get("USER_DATA_START").cloned();
        if let Some(addr) = user_data_addr {
            self.emit(format!("  LDA ${:04X}", addr));
            self.emit("  STA $04".to_string());
            self.emit(format!("  LDA ${:04X}", addr + 1));
            self.emit("  STA $05".to_string());
        }

        let nmi_label = program.declarations.iter().find_map(|d| {
            if let TopLevelKind::Interrupt(name, _) = &d.kind {
                if name.to_uppercase() == "NMI" {
                    return Some(name.clone());
                }
            }
            None
        });

        let irq_label = program.declarations.iter().find_map(|d| {
            if let TopLevelKind::Interrupt(name, _) = &d.kind {
                if name.to_uppercase() == "IRQ" {
                    return Some(name.clone());
                }
            }
            None
        });

        // Resolve handler addresses in the linker; table offsets vary with assets.
        for (handler, pointer) in [(nmi_label, 0x07F4), (irq_label, 0x07F6)] {
            let label = handler.as_deref().unwrap_or("DefaultRTI");
            self.emit(format!("  LDA #<{}", label));
            self.emit(format!("  STA ${:04X}", pointer));
            self.emit(format!("  LDA #>{}", label));
            self.emit(format!("  STA ${:04X}", pointer + 1));
        }
        for decl in &program.declarations {
            if let TopLevelKind::Dim(name, dtype, Some(expr)) = &decl.kind {
                if let Some(addr) = self.symbol_table.resolve(name).and_then(|s| s.address) {
                    self.output
                        .push(format!("  ; Init {} @ ${:04X}", name, addr));
                    match dtype {
                        DataType::Byte | DataType::Bool | DataType::Int | DataType::Enum(_) => {
                            let _ = self.generate_expression(expr)?;
                            self.emit(format!("  STA ${:04X}", addr));
                        }
                        DataType::Word | DataType::String => {
                            let rtype = self.generate_expression(expr)?;
                            self.emit(format!("  STA ${:04X}", addr));
                            if rtype == DataType::Word
                                || rtype == DataType::Int
                                || rtype == DataType::String
                            {
                                self.emit(format!("  STX ${:04X}", addr + 1));
                            } else {
                                self.emit("  LDA #0".to_string());
                                self.emit(format!("  STA ${:04X}", addr + 1));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        if self
            .symbol_table
            .resolve("Main")
            .and_then(|symbol| symbol.bank)
            .is_some_and(|bank| bank != 0 && bank != 7)
        {
            self.emit("  JSR Trampoline_Main".to_string());
        } else {
            self.emit("  JSR Main".to_string());
        }

        self.emit("forever:".to_string());
        self.emit("  JMP forever".to_string());
        self.emit("".to_string());
        self.emit("DefaultRTI:".to_string());
        self.emit("  RTS".to_string());
        self.emit("".to_string());
        self.emit("TrampolineNMI:".to_string());
        self.emit("  PHA".to_string());
        self.emit("  TXA".to_string());
        self.emit("  PHA".to_string());
        self.emit("  TYA".to_string());
        self.emit("  PHA".to_string());
        // A cross-bank caller may be interrupted while its return value lives
        // in shared scratch. Stack it so nested handler calls cannot replace it.
        self.emit("  LDA $07F1".to_string());
        self.emit("  PHA".to_string());
        // Compact loop leaves space for runtime helpers below the fixed data area.
        self.generate_interrupt_scratch("NMI", false);

        // OAM DMA
        self.emit("  LDA #$00".to_string());
        self.emit("  STA $2003".to_string());
        self.emit("  LDA #$02".to_string());
        self.emit("  STA $4014".to_string());

        // VBlank Buffer Processing
        self.output
            .push(format!("  LDA ${:04X}", VBLANK_BUFFER_START));
        self.emit("  BEQ SkipVBlankBuffer".to_string());
        self.emit("  CMP #2".to_string());
        self.emit("  BEQ VBlankBufferAttributes".to_string());
        self.emit("  LDA $2002".to_string());
        self.output
            .push(format!("  LDA ${:04X}", VBLANK_BUFFER_START + 2));
        self.emit("  STA $2006".to_string());
        self.output
            .push(format!("  LDA ${:04X}", VBLANK_BUFFER_START + 3));
        self.emit("  STA $2006".to_string());
        // Check Type ($0441)
        self.output
            .push(format!("  LDA ${:04X}", VBLANK_BUFFER_START + 1));
        self.emit("  AND #1".to_string());
        self.emit("  BNE VBlankBufferRow".to_string());

        // Type 0: Column (Inc 32, Len 30)
        self.emit("  LDA $F8".to_string());
        self.emit("  ORA #$04".to_string()); // Inc 32
        self.emit("  STA $2000".to_string());
        self.emit("  LDX #0".to_string());
        self.emit("VBlankBufferColLoop:".to_string());
        self.output
            .push(format!("  LDA ${:04X}, X", VBLANK_BUFFER_START + 4));
        self.emit("  STA $2007".to_string());
        self.emit("  INX".to_string());
        self.emit("  CPX #30".to_string());
        self.emit("  BNE VBlankBufferColLoop".to_string());
        self.emit("  JMP VBlankBufferDone".to_string());

        self.emit("VBlankBufferRow:".to_string());
        // Type 1: Row (Inc 1, Len 32)
        self.emit("  LDA $F8".to_string());
        self.emit("  AND #$FB".to_string()); // Inc 1 (Clear Bit 2)
        self.emit("  STA $2000".to_string());
        self.emit("  LDX #0".to_string());
        self.emit("VBlankBufferRowLoop:".to_string());
        self.output
            .push(format!("  LDA ${:04X}, X", VBLANK_BUFFER_START + 4));
        self.emit("  STA $2007".to_string());
        self.emit("  INX".to_string());
        self.emit("  CPX #32".to_string());
        self.emit("  BNE VBlankBufferRowLoop".to_string());

        self.emit("VBlankBufferDone:".to_string());
        // A palette seam follows on the next NMI. Splitting the work keeps
        // uploads bounded; callers wait for the flag to become zero before revealing it.
        self.emit(format!("  LDA ${:04X}", VBLANK_BUFFER_START + 1));
        self.emit("  BPL VBlankBufferComplete".to_string());
        self.emit("  LDA #2".to_string());
        self.emit("  BNE VBlankBufferSetFlag".to_string());

        self.emit("VBlankBufferAttributes:".to_string());
        self.emit("  LDA $2002".to_string());
        self.emit("  LDA $F8".to_string());
        self.emit("  AND #$FB".to_string());
        self.emit("  STA $2000".to_string());
        self.emit("  LDX #0".to_string());
        self.emit(format!("  LDA ${:04X}", VBLANK_BUFFER_START + 1));
        self.emit("  AND #1".to_string());
        self.emit("  BEQ VBlankBufferAttributeColumn".to_string());
        self.emit(format!("  LDA ${:04X}", VBLANK_BUFFER_START + 36));
        self.emit("  STA $2006".to_string());
        self.emit(format!("  LDA ${:04X}", VBLANK_BUFFER_START + 37));
        self.emit("  STA $2006".to_string());
        self.emit("VBlankBufferAttributeRowLoop:".to_string());
        self.emit(format!("  LDA ${:04X},X", VBLANK_BUFFER_START + 38));
        self.emit("  STA $2007".to_string());
        self.emit("  INX".to_string());
        self.emit("  CPX #8".to_string());
        self.emit("  BNE VBlankBufferAttributeRowLoop".to_string());
        self.emit("  JMP VBlankBufferComplete".to_string());
        self.emit("VBlankBufferAttributeColumn:".to_string());
        self.emit(format!("  LDA ${:04X}", VBLANK_BUFFER_START + 36));
        self.emit("  STA $2006".to_string());
        self.emit(format!("  LDA ${:04X}", VBLANK_BUFFER_START + 37));
        self.emit("  STA $2006".to_string());
        self.emit(format!("  LDA ${:04X},X", VBLANK_BUFFER_START + 38));
        self.emit("  STA $2007".to_string());
        self.emit(format!("  LDA ${:04X}", VBLANK_BUFFER_START + 37));
        self.emit("  CLC".to_string());
        self.emit("  ADC #8".to_string());
        self.emit(format!("  STA ${:04X}", VBLANK_BUFFER_START + 37));
        self.emit("  INX".to_string());
        self.emit("  CPX #8".to_string());
        self.emit("  BNE VBlankBufferAttributeColumn".to_string());

        self.emit("VBlankBufferComplete:".to_string());
        self.emit("  LDA #0".to_string());
        self.emit("VBlankBufferSetFlag:".to_string());
        self.emit(format!("  STA ${:04X}", VBLANK_BUFFER_START));
        self.emit("  LDA $F8".to_string());
        self.emit("  STA $2000".to_string());
        self.emit("SkipVBlankBuffer:".to_string());

        self.emit("  LDA $07F4".to_string());
        self.emit("  ORA $07F5".to_string());
        self.emit("  BEQ SkipNMI".to_string());
        self.emit("  JSR CallUserNMI".to_string());
        self.emit("SkipNMI:".to_string());

        self.emit("  LDA $2002".to_string());
        self.emit("  LDA $E0".to_string());
        self.emit("  STA $2005".to_string());
        self.emit("  LDA $E1".to_string());
        self.emit("  STA $2005".to_string());

        // Complete all PPU duties before audio. APU writes may occur during
        // visible scanlines; deferring them keeps a busy sound engine from
        // pushing the final scroll writes beyond the short VBlank interval.
        self.emit("  JSR Sound_Update".to_string());

        // Publish a completed frame so WAIT_VBLANK need not race the handler's
        // status reads, which clear the hardware VBlank flag before main resumes.
        self.emit("  INC $FD".to_string());

        self.generate_interrupt_scratch("NMI", true);
        // Mark even a handler with no bank calls: an interrupted writer can
        // safely restart, and nested bank calls may have reset this flag.
        self.emit("  LDA #1".to_string());
        self.emit("  STA $07F2".to_string());
        self.emit("  PLA".to_string());
        self.emit("  STA $07F1".to_string());
        self.emit("  PLA".to_string());
        self.emit("  TAY".to_string());
        self.emit("  PLA".to_string());
        self.emit("  TAX".to_string());
        self.emit("  PLA".to_string());
        self.emit("  RTI".to_string());

        self.emit("CallUserNMI:".to_string());
        self.emit("  LDX $07F8".to_string());
        self.emit("  BNE DispatchSelectedInterrupt".to_string());
        self.emit("  JMP ($07F4)".to_string());

        self.emit("TrampolineIRQ:".to_string());
        self.emit("  PHA".to_string());
        self.emit("  TXA".to_string());
        self.emit("  PHA".to_string());
        self.emit("  TYA".to_string());
        self.emit("  PHA".to_string());
        // A cross-bank caller may be interrupted while its return value lives
        // in shared scratch. Stack it so nested handler calls cannot replace it.
        self.emit("  LDA $07F1".to_string());
        self.emit("  PHA".to_string());
        // Compact loop leaves space for runtime helpers below the fixed data area.
        self.generate_interrupt_scratch("IRQ", false);

        self.emit("  LDA $07F6".to_string());
        self.emit("  ORA $07F7".to_string());
        self.emit("  BEQ SkipIRQ".to_string());
        self.emit("  JSR CallUserIRQ".to_string());
        self.emit("SkipIRQ:".to_string());

        self.generate_interrupt_scratch("IRQ", true);
        self.emit("  LDA #1".to_string());
        self.emit("  STA $07F2".to_string());
        self.emit("  PLA".to_string());
        self.emit("  STA $07F1".to_string());
        self.emit("  PLA".to_string());
        self.emit("  TAY".to_string());
        self.emit("  PLA".to_string());
        self.emit("  TAX".to_string());
        self.emit("  PLA".to_string());
        self.emit("  RTI".to_string());

        self.emit("CallUserIRQ:".to_string());
        self.emit("  LDX $07F9".to_string());
        self.emit("  BNE DispatchSelectedInterrupt".to_string());
        self.emit("  JMP ($07F6)".to_string());
        self.emit("DispatchSelectedInterrupt:".to_string());
        // Wrappers already saved these scratch bytes and X on the stack.
        self.emit("  LDA InterruptTargetLow,X".to_string());
        self.emit("  STA $00".to_string());
        self.emit("  LDA InterruptTargetHigh,X".to_string());
        self.emit("  STA $01".to_string());
        self.emit("  JMP ($0000)".to_string());
        for (table, prefix) in [("InterruptTargetLow", '<'), ("InterruptTargetHigh", '>')] {
            self.emit(format!("{table}:"));
            self.emit("  DB 0".to_string());
            for (_, target) in self.interrupt_targets.clone() {
                self.emit(format!("  DB {prefix}{target}"));
            }
        }
        self.emit("".to_string());

        Ok(())
    }

    fn generate_trampolines(&mut self, program: &Program) {
        self.emit("; --- Cross-Bank Trampolines ---".to_string());
        let mut trampolines_generated = std::collections::HashSet::new();
        for decl in &program.declarations {
            if let TopLevelKind::Sub(name, _, _) = &decl.kind {
                if trampolines_generated.insert(name.clone()) {
                    let target_bank = self
                        .symbol_table
                        .resolve(name)
                        .and_then(|sym| sym.bank)
                        .unwrap_or(0);
                    self.emit(format!("Trampoline_{}:", name));
                    // Save caller bank to stack
                    self.emit("  LDA $07F0".to_string());
                    self.emit("  PHA".to_string());
                    // Switch to target bank
                    self.emit(format!("  LDA #${:02X}", target_bank));
                    self.emit("  JSR MMC1_SetPrgBank".to_string());
                    // Call the target subroutine
                    self.emit(format!("  JSR {}", name));
                    // Preserve the expression result while restoring the caller's mapping.
                    self.emit("  STA $07F1".to_string());
                    self.emit("  PLA".to_string());
                    self.emit("  JSR MMC1_SetPrgBank".to_string());
                    self.emit("  LDA $07F1".to_string());
                    self.emit("  RTS".to_string());
                }
            }
        }
    }

    fn generate_sound_engine(&mut self) {
        self.output
            .push("; --- Sound Engine (Phase 25a) ---".to_string());

        self.emit("Sound_Init:".to_string());
        // Clearing software state alone leaves APU length counters and DMC DAC audible.
        self.emit("  LDA #0".to_string());
        self.emit("  STA $4015".to_string());
        self.emit("  STA $4008".to_string());
        self.emit("  STA $4011".to_string());
        self.emit("  LDA #$30".to_string());
        self.emit("  STA $4000".to_string());
        self.emit("  STA $4004".to_string());
        self.emit("  STA $400C".to_string());
        self.emit("  LDA #$0F".to_string());
        self.emit("  STA $4015".to_string());
        self.emit("  LDX #$00".to_string());
        self.emit("  LDA #$00".to_string());
        self.emit("SndClear:".to_string());
        self.output
            .push(format!("  STA ${:04X}, X", SOUND_RAM_START));
        self.emit("  INX".to_string());
        self.emit("  CPX #$80".to_string());
        self.emit("  BNE SndClear".to_string());
        self.emit("  RTS".to_string());

        self.emit("Sound_Play:".to_string());
        self.emit("  PHA".to_string());
        self.emit("  LDA $D100".to_string());
        self.emit("  STA $F0".to_string());
        self.emit("  PLA".to_string());
        self.emit("  CMP $F0".to_string());
        self.emit("  BCC SndPlay_Check1".to_string());
        self.emit("  JMP SndPlayEnd".to_string());
        self.emit("SndPlay_Check1:".to_string());
        self.emit("  ASL".to_string());
        self.emit("  TAX".to_string());
        self.emit("  LDA $D101, X".to_string());
        self.emit("  STA $F0".to_string());
        self.emit("  LDA $D102, X".to_string());
        self.emit("  STA $F1".to_string());

        self.emit("  LDY #$00".to_string());
        self.emit("  LDA ($F0), Y".to_string());
        self.emit("  AND #$03".to_string());
        self.emit("  STA $F2".to_string());

        self.emit("  LDA $F2".to_string());
        self.emit("  ASL".to_string());
        self.emit("  ASL".to_string());
        self.emit("  ASL".to_string());
        self.emit("  ASL".to_string());
        self.emit("  ASL".to_string());
        self.emit("  TAX".to_string());

        self.emit("  LDY #$02".to_string());
        self.emit("  LDA ($F0), Y".to_string());
        self.output
            .push(format!("  CMP ${:04X}, X", SOUND_RAM_START + 0x05));
        self.emit("  BCS SndPlay_PriorityOK".to_string());
        self.emit("  JMP SndPlayEnd".to_string());
        self.emit("SndPlay_PriorityOK:".to_string());
        // NMI updates must not consume a half-written channel record.
        self.emit("  PHA".to_string());
        self.emit("  LDA #0".to_string());
        self.emit(format!("  STA ${SOUND_RAM_START:04X}, X"));
        self.emit("  PLA".to_string());
        self.output
            .push(format!("  STA ${:04X}, X", SOUND_RAM_START + 0x05));

        self.emit("  LDY #$01".to_string());
        self.emit("  LDA ($F0), Y".to_string());
        self.output
            .push(format!("  STA ${:04X}, X", SOUND_RAM_START + 0x04));

        self.emit("  LDY #$03".to_string());
        self.emit("  LDA ($F0), Y".to_string());
        self.output
            .push(format!("  STA ${:04X}, X", SOUND_RAM_START + 0x06));

        self.emit("  LDY #$04".to_string());
        self.emit("  LDA ($F0), Y".to_string());
        self.output
            .push(format!("  STA ${:04X}, X", SOUND_RAM_START + 0x09));

        self.emit("  LDY #$05".to_string());
        self.emit("  LDA ($F0), Y".to_string());
        self.output
            .push(format!("  STA ${:04X}, X", SOUND_RAM_START + 0x0D));

        self.emit("  LDA #0".to_string());
        self.output
            .push(format!("  STA ${:04X}, X", SOUND_RAM_START + 0x07));
        self.output
            .push(format!("  STA ${:04X}, X", SOUND_RAM_START + 0x08));
        self.output
            .push(format!("  STA ${:04X}, X", SOUND_RAM_START + 0x0A));
        self.output
            .push(format!("  STA ${:04X}, X", SOUND_RAM_START + 0x0B));
        self.output
            .push(format!("  STA ${:04X}, X", SOUND_RAM_START + 0x0C));
        self.output
            .push(format!("  STA ${:04X}, X", SOUND_RAM_START + 0x0E));
        self.output
            .push(format!("  STA ${:04X}, X", SOUND_RAM_START + 0x0F));
        self.output
            .push(format!("  STA ${:04X}, X", SOUND_RAM_START + 0x10));
        self.output
            .push(format!("  STA ${:04X}, X", SOUND_RAM_START + 0x14));
        self.output
            .push(format!("  STA ${:04X}, X", SOUND_RAM_START + 0x15));

        self.emit(format!("  STA ${:04X}, X", SOUND_RAM_START + 0x16));
        // Music has no duty envelope. Force the initial timer-high write even for high=0.
        self.emit("  LDA #$FF".to_string());
        self.emit(format!("  STA ${:04X}, X", SOUND_RAM_START + 0x13));
        self.emit(format!("  STA ${:04X}, X", SOUND_RAM_START + 0x12));

        self.emit("  LDA $F0".to_string());
        self.emit("  CLC".to_string());
        self.emit("  ADC #6".to_string());
        self.emit("  STA $F0".to_string());
        self.emit("  BCC SndPtrInc2".to_string());
        self.emit("  INC $F1".to_string());
        self.emit("SndPtrInc2:".to_string());

        self.emit("  LDA #$01".to_string());
        self.output
            .push(format!("  STA ${:04X}, X", SOUND_RAM_START + 1));
        self.emit("  LDA $F0".to_string());
        self.output
            .push(format!("  STA ${:04X}, X", SOUND_RAM_START + 2));
        self.emit("  LDA $F1".to_string());
        self.output
            .push(format!("  STA ${:04X}, X", SOUND_RAM_START + 3));
        self.emit("  LDA #1".to_string());
        self.emit(format!("  STA ${SOUND_RAM_START:04X}, X"));
        self.emit("SndPlayEnd:".to_string());
        self.emit("  RTS".to_string());

        self.emit("SFX_Play:".to_string());
        self.emit("  PHA".to_string());
        self.emit(format!("  LDA ${:04X}", SFX_TABLE_ADDR));
        self.emit("  STA $F0".to_string());
        self.emit("  PLA".to_string());
        self.emit("  CMP $F0".to_string());
        self.emit("  BCC SFXPlay_Check1".to_string());
        self.emit("  RTS".to_string());
        self.emit("SFXPlay_Check1:".to_string());
        self.emit("  ASL".to_string());
        self.emit("  TAX".to_string());
        self.output
            .push(format!("  LDA ${:04X}, X", SFX_TABLE_ADDR + 1));
        self.emit("  STA $F0".to_string());
        self.output
            .push(format!("  LDA ${:04X}, X", SFX_TABLE_ADDR + 2));
        self.emit("  STA $F1".to_string());

        self.emit("  LDY #0".to_string());
        self.emit("  LDA ($F0), Y".to_string());
        self.emit("  AND #$03".to_string());
        self.emit("  STA $F2".to_string());

        self.emit("  LDA $F2".to_string());
        self.emit("  ASL".to_string());
        self.emit("  ASL".to_string());
        self.emit("  ASL".to_string());
        self.emit("  ASL".to_string());
        self.emit("  ASL".to_string());
        self.emit("  TAX".to_string());

        self.emit("  LDY #1".to_string());
        self.emit("  LDA ($F0), Y".to_string());
        self.output
            .push(format!("  CMP ${:04X}, X", SOUND_RAM_START + 0x05));
        self.emit("  BCS SFXPlay_Apply".to_string());
        self.emit("  RTS".to_string());
        self.emit("SFXPlay_Apply:".to_string());
        // The last ten bytes of each channel preserve the interrupted music note.
        // Replacing an effect retains this backup rather than backing up the effect.
        self.emit("  PHA".to_string());
        self.emit(format!("  LDA ${SOUND_RAM_START:04X}, X"));
        self.emit("  PHA".to_string());
        self.emit("  LDA #0".to_string());
        self.emit(format!("  STA ${SOUND_RAM_START:04X}, X"));
        self.emit("  PLA".to_string());
        self.emit("  CMP #1".to_string());
        self.emit("  BNE SFXPlay_NoMusicBackup".to_string());
        self.emit("  LDA #1".to_string());
        self.emit(format!("  STA ${:04X}, X", SOUND_RAM_START + 0x16));
        for (backup, live) in [1, 2, 3, 4, 5, 6, 9, 13, 17].into_iter().enumerate() {
            self.emit(format!("  LDA ${:04X}, X", SOUND_RAM_START + live));
            self.emit(format!(
                "  STA ${:04X}, X",
                SOUND_RAM_START + 0x17 + backup as u16
            ));
        }
        self.emit("  JMP SFXPlay_BackupReady".to_string());
        self.emit("SFXPlay_NoMusicBackup:".to_string());
        self.emit("  CMP #0".to_string());
        self.emit("  BNE SFXPlay_BackupReady".to_string());
        self.emit(format!("  STA ${:04X}, X", SOUND_RAM_START + 0x16));
        self.emit("SFXPlay_BackupReady:".to_string());
        self.emit("  PLA".to_string());
        self.output
            .push(format!("  STA ${:04X}, X", SOUND_RAM_START + 0x05));

        self.emit("  LDY #2".to_string());
        self.emit("  LDA ($F0), Y".to_string());
        self.output
            .push(format!("  STA ${:04X}, X", SOUND_RAM_START + 0x06));

        self.emit("  LDY #3".to_string());
        self.emit("  LDA ($F0), Y".to_string());
        self.output
            .push(format!("  STA ${:04X}, X", SOUND_RAM_START + 0x09));

        self.emit("  LDY #4".to_string());
        self.emit("  LDA ($F0), Y".to_string());
        self.output
            .push(format!("  STA ${:04X}, X", SOUND_RAM_START + 0x13));

        self.emit("  LDY #5".to_string());
        self.emit("  LDA ($F0), Y".to_string());
        self.emit(format!("  STA ${:04X}, X", SOUND_RAM_START + 0x0D));

        // Macro pitch offsets start from note 48, matching the editor preview.
        self.emit("  LDA #48".to_string());
        self.emit(format!("  STA ${:04X}, X", SOUND_RAM_START + 0x11));
        self.emit("  LDA #$3F".to_string());
        self.emit(format!("  STA ${:04X}, X", SOUND_RAM_START + 0x04));
        self.emit("  LDA #$FF".to_string());
        self.emit(format!("  STA ${:04X}, X", SOUND_RAM_START + 0x12));

        self.emit("  LDA #0".to_string());
        self.output
            .push(format!("  STA ${:04X}, X", SOUND_RAM_START + 0x07));
        self.output
            .push(format!("  STA ${:04X}, X", SOUND_RAM_START + 0x08));
        self.output
            .push(format!("  STA ${:04X}, X", SOUND_RAM_START + 0x0A));
        self.output
            .push(format!("  STA ${:04X}, X", SOUND_RAM_START + 0x0B));
        self.output
            .push(format!("  STA ${:04X}, X", SOUND_RAM_START + 0x0C));
        self.output
            .push(format!("  STA ${:04X}, X", SOUND_RAM_START + 0x14));
        self.output
            .push(format!("  STA ${:04X}, X", SOUND_RAM_START + 0x15));
        for offset in [0x0e, 0x0f, 0x10] {
            self.emit(format!("  STA ${:04X}, X", SOUND_RAM_START + offset));
        }
        self.emit("  LDA #2".to_string());
        self.emit(format!("  STA ${SOUND_RAM_START:04X}, X"));
        self.emit("  RTS".to_string());

        self.emit("Sound_Update:".to_string());
        self.emit("  PHA".to_string());
        self.emit("  TXA".to_string());
        self.emit("  PHA".to_string());
        self.emit("  TYA".to_string());
        self.emit("  PHA".to_string());

        // An inactive channel has no envelope or note work. In particular,
        // zero-initialized channel data must not be interpreted as envelope 0.
        self.emit("  LDX #0".to_string());
        self.emit("SoundChannelLoop:".to_string());
        self.emit(format!("  LDA ${SOUND_RAM_START:04X},X"));
        self.emit("  BEQ SoundChannelNext".to_string());
        self.emit("  JSR SndChUpdate".to_string());
        self.emit(format!("  LDA ${SOUND_RAM_START:04X},X"));
        self.emit("  BEQ SoundChannelNext".to_string());
        self.emit("  JSR SndEnvUpdate".to_string());
        self.emit("  JSR SndPitchUpdate".to_string());
        self.emit("  JSR SndDutyUpdate".to_string());
        self.emit("  JSR SndArpUpdate".to_string());
        self.emit("  JSR SndCalcPeriod".to_string());
        self.emit("  JSR SndWriteControl".to_string());
        self.emit("  JSR SndEffectComplete".to_string());
        self.emit("SoundChannelNext:".to_string());
        self.emit("  TXA".to_string());
        self.emit("  CLC".to_string());
        self.emit("  ADC #$20".to_string());
        self.emit("  TAX".to_string());
        self.emit("  BPL SoundChannelLoop".to_string());
        self.emit("  PLA".to_string());
        self.emit("  TAY".to_string());
        self.emit("  PLA".to_string());
        self.emit("  TAX".to_string());
        self.emit("  PLA".to_string());
        self.emit("  RTS".to_string());

        self.emit("Runtime_StringLen:".to_string());
        self.emit("  STA $02".to_string());
        self.emit("  STX $03".to_string());
        self.emit("  LDA #0".to_string());
        self.emit("  STA $06".to_string());
        self.emit("  STA $07".to_string());
        self.emit("  TAY".to_string());
        self.emit("Runtime_StringLen_Loop:".to_string());
        self.emit("  LDA ($02), Y".to_string());
        self.emit("  CMP #0".to_string());
        self.emit("  BEQ Runtime_StringLen_Done".to_string());
        self.emit("  INC $06".to_string());
        self.output
            .push("  BNE Runtime_StringLen_SkipIncHigh".to_string());
        self.emit("  INC $07".to_string());
        self.output
            .push("Runtime_StringLen_SkipIncHigh:".to_string());
        self.emit("  INY".to_string());
        self.emit("  BNE Runtime_StringLen_Loop".to_string());
        self.emit("  INC $03".to_string());
        self.emit("  JMP Runtime_StringLen_Loop".to_string());
        self.emit("Runtime_StringLen_Done:".to_string());
        self.emit("  LDA $06".to_string());
        self.emit("  LDX $07".to_string());
        self.emit("  RTS".to_string());

        self.emit("SndChUpdate:".to_string());
        self.output
            .push(format!("  LDA ${:04X}, X", SOUND_RAM_START));
        self.emit("  BNE SndActive".to_string());
        self.emit("  RTS".to_string());
        self.emit("SndActive:".to_string());
        // Macro effects are advanced by their envelopes, not the music note pointer.
        self.emit("  CMP #2".to_string());
        self.emit("  BNE SndMusicActive".to_string());
        self.emit("  RTS".to_string());
        self.emit("SndMusicActive:".to_string());
        self.output
            .push(format!("  DEC ${:04X}, X", SOUND_RAM_START + 1));
        self.emit("  BEQ SndTimerExpired".to_string());
        self.emit("  RTS".to_string());
        self.emit("SndTimerExpired:".to_string());
        self.output
            .push(format!("  LDA ${:04X}, X", SOUND_RAM_START + 2));
        self.emit("  STA $F4".to_string());
        self.output
            .push(format!("  LDA ${:04X}, X", SOUND_RAM_START + 3));
        self.emit("  STA $F5".to_string());

        self.emit("  LDY #$00".to_string());
        self.emit("  LDA ($F4), Y".to_string());
        self.emit("  CMP #$00".to_string());
        self.emit("  BNE SndNextNote".to_string());

        self.output
            .push(format!("  STA ${:04X}, X", SOUND_RAM_START));
        self.emit("  LDA #0".to_string());
        self.output
            .push(format!("  STA ${:04X}, X", SOUND_RAM_START + 0x05));

        self.emit("  CPX #$00".to_string());
        self.emit("  BEQ SilenceP1".to_string());
        self.emit("  CPX #$20".to_string());
        self.emit("  BEQ SilenceP2".to_string());
        self.emit("  CPX #$40".to_string());
        self.emit("  BEQ SilenceTri".to_string());
        self.emit("  CPX #$60".to_string());
        self.emit("  BEQ SilenceDMC".to_string());
        self.emit("  JMP SndChEnd".to_string());

        self.emit("SilenceP1:".to_string());
        self.emit("  LDA #$30".to_string());
        self.emit("  STA $4000".to_string());
        self.emit("  JMP SndSilenceAdvance".to_string());
        self.emit("SilenceP2:".to_string());
        self.emit("  LDA #$30".to_string());
        self.emit("  STA $4004".to_string());
        self.emit("  JMP SndSilenceAdvance".to_string());
        self.emit("SilenceTri:".to_string());
        self.emit("  LDA #$80".to_string());
        self.emit("  STA $4008".to_string());
        self.emit("  JMP SndSilenceAdvance".to_string());
        self.emit("SilenceDMC:".to_string());
        self.emit("  LDA #$0F".to_string());
        self.emit("  STA $4015".to_string());
        self.emit("SndSilenceAdvance:".to_string());
        self.emit(format!("  LDA ${SOUND_RAM_START:04X}, X"));
        self.emit("  BEQ SndChEnd".to_string());
        self.emit("  JMP SndAdvance".to_string());

        self.emit("SndNextNote:".to_string());
        self.output
            .push(format!("  STA ${:04X}, X", SOUND_RAM_START + 1));
        self.emit("  INC $F4".to_string());
        self.emit("  BNE SndPtrInc3".to_string());
        self.emit("  INC $F5".to_string());
        self.emit("SndPtrInc3:".to_string());
        self.emit("  LDA ($F4), Y".to_string());

        self.emit("  CMP #$FF".to_string());
        self.emit("  BEQ SndPlaySilence".to_string());

        self.output
            .push(format!("  STA ${:04X}, X", SOUND_RAM_START + 0x11));

        self.emit("  CPX #$60".to_string());
        self.emit("  BNE SndTonalNote".to_string());
        self.emit("  JMP PlayDMC".to_string());
        self.emit("SndTonalNote:".to_string());
        // Envelope values belong to a note; discard the previous note's offsets.
        self.emit("  LDA #0".to_string());
        for offset in [7, 8, 10, 11, 12, 14, 15, 16, 20, 21] {
            self.emit(format!("  STA ${:04X}, X", SOUND_RAM_START + offset));
        }
        self.emit("  LDA #$FF".to_string());
        self.emit(format!("  STA ${:04X}, X", SOUND_RAM_START + 18));
        self.emit("  JMP SndAdvance".to_string());
        self.emit("SndPlaySilence:".to_string());
        self.emit("  LDA #$FF".to_string());
        self.emit(format!("  STA ${:04X}, X", SOUND_RAM_START + 17));
        self.emit("  CPX #$60".to_string());
        self.emit("  BNE SndRestAdvance".to_string());
        self.emit("  LDA #$0F".to_string());
        self.emit("  STA $4015".to_string());
        self.emit("SndRestAdvance:".to_string());
        self.emit("  JMP SndAdvance".to_string());

        self.emit("PlayDMC:".to_string());
        // DMC pitch is a sample ID, not a period-table index; instrument is its rate.
        self.emit(format!("  LDA ${:04X}, X", SOUND_RAM_START + 0x11));
        self.emit("  ASL".to_string());
        self.emit("  TAY".to_string());
        self.emit("  LDA $D480, Y".to_string());
        self.emit("  STA $4012".to_string());
        self.emit("  INY".to_string());
        self.emit("  LDA $D480, Y".to_string());
        self.emit("  STA $4013".to_string());
        self.emit(format!("  LDA ${:04X}, X", SOUND_RAM_START + 0x04));
        self.emit("  AND #$0F".to_string());
        self.emit("  STA $4010".to_string());
        self.emit("  LDA #$0F".to_string());
        self.emit("  STA $4015".to_string());
        self.emit("  LDA #$1F".to_string());
        self.emit("  STA $4015".to_string());

        self.emit("SndAdvance:".to_string());
        self.emit("  INC $F4".to_string());
        self.emit("  BNE SndPtrInc4".to_string());
        self.emit("  INC $F5".to_string());
        self.emit("SndPtrInc4:".to_string());
        self.emit("  LDA $F4".to_string());
        self.output
            .push(format!("  STA ${:04X}, X", SOUND_RAM_START + 2));
        self.emit("  LDA $F5".to_string());
        self.output
            .push(format!("  STA ${:04X}, X", SOUND_RAM_START + 3));
        self.emit("SndChEnd:".to_string());
        self.emit("  RTS".to_string());

        // Shared envelope timing: one frame is one update; loops add no idle frame.
        for (name, id, index, timer, value) in [
            ("SndEnv", 6, 7, 8, None),
            ("SndPitch", 9, 10, 11, Some(12)),
            ("SndDuty", 19, 20, 21, None),
            ("SndArp", 13, 14, 15, Some(16)),
        ] {
            self.emit(format!("{name}Update:"));
            self.emit(format!("  LDA ${:04X}, X", SOUND_RAM_START + id));
            self.emit("  CMP #$FF".into());
            self.emit(format!("  BEQ {name}Return"));
            self.emit(format!("  LDA ${:04X}, X", SOUND_RAM_START + timer));
            self.emit(format!("  BEQ {name}NextStep"));
            self.emit(format!("  DEC ${:04X}, X", SOUND_RAM_START + timer));
            self.emit(format!("  BNE {name}Return"));
            self.emit(format!("{name}NextStep:"));
            self.emit(format!("  LDA ${:04X}, X", SOUND_RAM_START + id));
            self.emit("  ASL".into());
            self.emit("  TAY".into());
            self.emit(format!("  LDA ${:04X}, Y", ENVELOPE_TABLE_ADDR + 1));
            self.emit("  STA $F0".into());
            self.emit(format!("  LDA ${:04X}, Y", ENVELOPE_TABLE_ADDR + 2));
            self.emit("  STA $F1".into());
            self.emit("  LDY #0".into());
            self.emit("  LDA ($F0), Y".into());
            self.emit("  STA $F5".into()); // Preserve loop index during address calculation.
            self.emit("  INC $F0".into());
            self.emit(format!("  BNE {name}HeaderAdvanced"));
            self.emit("  INC $F1".into());
            self.emit(format!("{name}HeaderAdvanced:"));
            self.emit(format!("  LDA ${:04X}, X", SOUND_RAM_START + index));
            self.emit("  ASL".into());
            self.emit("  CLC".into());
            self.emit("  ADC $F0".into());
            self.emit("  STA $F0".into());
            self.emit(format!("  BCC {name}StepAddressed"));
            self.emit("  INC $F1".into());
            self.emit(format!("{name}StepAddressed:"));
            self.emit("  LDA ($F0), Y".into());
            self.emit("  STA $F3".into());
            self.emit("  INY".into());
            self.emit("  LDA ($F0), Y".into());
            self.emit(format!("  BNE {name}Apply"));
            self.emit("  LDA $F5".into());
            self.emit("  CMP #$FF".into());
            self.emit(format!("  BEQ {name}Finished"));
            self.emit(format!("  STA ${:04X}, X", SOUND_RAM_START + index));
            self.emit(format!("  JMP {name}NextStep"));
            self.emit(format!("{name}Finished:"));
            // Music holds a finite envelope's last value and restarts on each note.
            // Effects mark exhaustion so the longest sequence decides completion.
            self.emit(format!("  LDA ${SOUND_RAM_START:04X}, X"));
            self.emit("  CMP #2".into());
            self.emit(format!("  BNE {name}Return"));
            self.emit("  LDA #$FF".into());
            self.emit(format!("  STA ${:04X}, X", SOUND_RAM_START + id));
            self.emit("  RTS".into());
            self.emit(format!("{name}Apply:"));
            self.emit(format!("  STA ${:04X}, X", SOUND_RAM_START + timer));
            self.emit(format!("  INC ${:04X}, X", SOUND_RAM_START + index));
            if let Some(offset) = value {
                self.emit("  LDA $F3".into());
                self.emit(format!("  STA ${:04X}, X", SOUND_RAM_START + offset));
            } else if name == "SndEnv" {
                self.emit("  LDA $F3".into());
                self.emit("  AND #$0F".into());
                self.emit("  STA $F3".into());
                self.emit(format!("  LDA ${:04X}, X", SOUND_RAM_START + 4));
                self.emit("  AND #$F0".into());
                self.emit("  ORA $F3".into());
                self.emit(format!("  STA ${:04X}, X", SOUND_RAM_START + 4));
            } else {
                self.emit("  LDA $F3".into());
                for _ in 0..6 {
                    self.emit("  ASL".into());
                }
                self.emit("  STA $F3".into());
                self.emit(format!("  LDA ${:04X}, X", SOUND_RAM_START + 4));
                self.emit("  AND #$3F".into());
                self.emit("  ORA $F3".into());
                self.emit(format!("  STA ${:04X}, X", SOUND_RAM_START + 4));
            }
            self.emit(format!("{name}Return:"));
            self.emit("  RTS".into());
        }

        self.emit("SndEffectComplete:".into());
        self.emit(format!("  LDA ${SOUND_RAM_START:04X}, X"));
        self.emit("  CMP #2".into());
        self.emit("  BNE SndEffectReturn".into());
        self.emit(format!("  LDA ${:04X}, X", SOUND_RAM_START + 6));
        for offset in [9, 13, 19] {
            self.emit(format!("  AND ${:04X}, X", SOUND_RAM_START + offset));
        }
        self.emit("  CMP #$FF".into());
        self.emit("  BNE SndEffectReturn".into());
        self.emit(format!("  LDA ${:04X}, X", SOUND_RAM_START + 22));
        self.emit("  BNE SndEffectRestoreMusic".into());
        self.emit(format!("  STA ${SOUND_RAM_START:04X}, X"));
        self.emit(format!("  STA ${:04X}, X", SOUND_RAM_START + 5));
        self.emit("  LDA #$FF".into());
        self.emit(format!("  STA ${:04X}, X", SOUND_RAM_START + 17));
        self.emit("  JMP SndWriteControl".into());
        self.emit("SndEffectRestoreMusic:".into());
        for (backup, live) in [0, 1, 2, 3, 4, 5, 6, 9, 13, 17].into_iter().enumerate() {
            self.emit(format!(
                "  LDA ${:04X}, X",
                SOUND_RAM_START + 22 + backup as u16
            ));
            self.emit(format!("  STA ${:04X}, X", SOUND_RAM_START + live));
        }
        self.emit("  LDA #0".into());
        for offset in [7, 8, 10, 11, 12, 14, 15, 16, 20, 21, 22] {
            self.emit(format!("  STA ${:04X}, X", SOUND_RAM_START + offset));
        }
        self.emit("  LDA #$FF".into());
        self.emit(format!("  STA ${:04X}, X", SOUND_RAM_START + 19));
        self.emit(format!("  STA ${:04X}, X", SOUND_RAM_START + 18));
        self.emit("  JSR SndCalcPeriod".into());
        self.emit("  JMP SndWriteControl".into());
        self.emit("SndEffectReturn:".into());
        self.emit("  RTS".into());

        self.emit("SndWriteControl:".into());
        self.emit("  CPX #$60".into());
        self.emit("  BEQ SndControlReturn".into());
        self.emit(format!("  LDA ${:04X}, X", SOUND_RAM_START + 17));
        self.emit("  CMP #$FF".into());
        self.emit("  BEQ SndControlSilence".into());
        self.emit(format!("  LDA ${:04X}, X", SOUND_RAM_START + 4));
        self.emit("  CPX #$40".into());
        self.emit("  BNE SndControlWrite".into());
        self.emit("  AND #$0F".into());
        self.emit("  BEQ SndControlWrite".into());
        self.emit("  LDA #$FF".into());
        self.emit("  BNE SndControlWrite".into());
        self.emit("SndControlSilence:".into());
        self.emit("  LDA #$30".into());
        self.emit("  CPX #$40".into());
        self.emit("  BNE SndControlWrite".into());
        self.emit("  LDA #0".into());
        self.emit("SndControlWrite:".into());
        self.emit("  CPX #0".into());
        self.emit("  BEQ SndControlP1".into());
        self.emit("  CPX #$20".into());
        self.emit("  BEQ SndControlP2".into());
        self.emit("  STA $4008".into());
        self.emit("  RTS".into());
        self.emit("SndControlP1:".into());
        self.emit("  STA $4000".into());
        self.emit("  RTS".into());
        self.emit("SndControlP2:".into());
        self.emit("  STA $4004".into());
        self.emit("SndControlReturn:".into());
        self.emit("  RTS".into());
        self.emit("SndCalcPeriod:".to_string());
        self.emit("  CPX #$60".to_string());
        self.emit("  BEQ SndCalcRet".to_string());

        self.output
            .push(format!("  LDA ${:04X}, X", SOUND_RAM_START + 0x11)); // BasePitch
        self.emit("  CMP #$FF".to_string());
        self.emit("  BEQ SndCalcRet".to_string());
        self.emit("  CLC".to_string());
        self.output
            .push(format!("  ADC ${:04X}, X", SOUND_RAM_START + 0x10)); // ArpOffset

        // Signed offsets may overflow an eight-bit signed sum (95 + 127).
        // Carry distinguishes negative underflow without misclassifying high notes.
        self.emit("  STA $F3".to_string());
        self.emit(format!("  LDA ${:04X}, X", SOUND_RAM_START + 0x10));
        self.emit("  BPL SndCalcPositiveOffset".to_string());
        self.emit("  BCS SndCalcPos".to_string());
        self.emit("  LDA #0".to_string());
        self.emit("  JMP SndCalcLookup".to_string());
        self.emit("SndCalcPositiveOffset:".to_string());
        self.emit("  BCC SndCalcPos".to_string());
        self.emit("  LDA #95".to_string());
        self.emit("  JMP SndCalcLookup".to_string());
        self.emit("SndCalcPos:".to_string());
        self.emit("  LDA $F3".to_string());
        self.emit("  CMP #96".to_string());
        self.emit("  BCC SndCalcLookup".to_string());
        self.emit("  LDA #95".to_string());
        self.emit("SndCalcLookup:".to_string());

        self.emit("  ASL".to_string());
        self.emit("  TAY".to_string());
        self.emit("  LDA $D000, Y".to_string());
        self.emit("  STA $F6".to_string());
        self.emit("  LDA $D001, Y".to_string());
        self.emit("  STA $F7".to_string());

        self.emit("  LDA $F6".to_string());
        self.emit("  CLC".to_string());
        self.output
            .push(format!("  ADC ${:04X}, X", SOUND_RAM_START + 0x0C));
        self.emit("  STA $F6".to_string());
        self.output
            .push(format!("  LDA ${:04X}, X", SOUND_RAM_START + 0x0C));
        self.emit("  BPL SndCalcPitchPos".to_string());
        self.emit("  LDA $F7".to_string());
        self.emit("  ADC #$FF".to_string());
        self.emit("  STA $F7".to_string());
        self.emit("  JMP SndCalcPitchDone".to_string());
        self.emit("SndCalcPitchPos:".to_string());
        self.emit("  LDA $F7".to_string());
        self.emit("  ADC #0".to_string());
        self.emit("  STA $F7".to_string());
        self.emit("SndCalcPitchDone:".to_string());

        self.emit("  CPX #0".to_string());
        self.emit("  BEQ SndCalcP1".to_string());
        self.emit("  CPX #$20".to_string());
        self.emit("  BEQ SndCalcP2".to_string());
        self.emit("  CPX #$40".to_string());
        self.emit("  BEQ SndCalcTri".to_string());
        self.emit("SndCalcRet:".to_string());
        self.emit("  RTS".to_string());

        self.emit("SndCalcP1:".to_string());
        self.emit("  LDA $F6".to_string());
        self.emit("  STA $4002".to_string());
        self.emit("  LDA $F7".to_string());
        self.output
            .push(format!("  CMP ${:04X}, X", SOUND_RAM_START + 0x12));
        self.emit("  BEQ SndCalcRet".to_string());
        self.emit("  STA $4003".to_string());
        self.output
            .push(format!("  STA ${:04X}, X", SOUND_RAM_START + 0x12));
        self.emit("  RTS".to_string());

        self.emit("SndCalcP2:".to_string());
        self.emit("  LDA $F6".to_string());
        self.emit("  STA $4006".to_string());
        self.emit("  LDA $F7".to_string());
        self.output
            .push(format!("  CMP ${:04X}, X", SOUND_RAM_START + 0x12));
        self.emit("  BEQ SndCalcRet".to_string());
        self.emit("  STA $4007".to_string());
        self.output
            .push(format!("  STA ${:04X}, X", SOUND_RAM_START + 0x12));
        self.emit("  RTS".to_string());

        self.emit("SndCalcTri:".to_string());
        self.emit("  LDA $F6".to_string());
        self.emit("  STA $400A".to_string());
        self.emit("  LDA $F7".to_string());
        self.output
            .push(format!("  CMP ${:04X}, X", SOUND_RAM_START + 0x12));
        self.emit("  BEQ SndCalcRet".to_string());
        self.emit("  STA $400B".to_string());
        self.output
            .push(format!("  STA ${:04X}, X", SOUND_RAM_START + 0x12));
        self.emit("  RTS".to_string());
    }
    fn generate_math_helpers(&mut self) {
        // ... (Keep existing helpers: Mul8, Div8, Mul16, Div16, Div16_Signed, ReadByte, ReadString)
        self.emit("; --- Math Helpers ---".to_string());

        self.emit("Math_Mul8:".to_string());
        self.emit("  STA $01".to_string());
        self.emit("  LDA #0".to_string());
        self.emit("  LDX #8".to_string());
        self.emit("Math_MulLoop:".to_string());
        self.emit("  ASL".to_string());
        self.emit("  ASL $01".to_string());
        self.emit("  BCC Math_MulNoAdd".to_string());
        self.emit("  CLC".to_string());
        self.emit("  ADC $00".to_string());
        self.emit("Math_MulNoAdd:".to_string());
        self.emit("  DEX".to_string());
        self.emit("  BNE Math_MulLoop".to_string());
        self.emit("  RTS".to_string());

        self.emit("Math_Div8:".to_string());
        self.emit("  STA $02".to_string());
        self.emit("  LDA #0".to_string());
        self.emit("  STA $01".to_string());
        self.emit("  LDX #8".to_string());
        self.emit("Math_DivLoop:".to_string());
        self.emit("  ASL $02".to_string());
        self.emit("  ROL".to_string());
        self.emit("  CMP $00".to_string());
        self.emit("  BCC Math_DivSkip".to_string());
        self.emit("  SBC $00".to_string());
        self.emit("  INC $02".to_string());
        self.emit("Math_DivSkip:".to_string());
        self.emit("  DEX".to_string());
        self.emit("  BNE Math_DivLoop".to_string());
        self.emit("  STA $01".to_string());
        self.emit("  LDA $02".to_string());
        self.emit("  RTS".to_string());

        self.emit("Math_Mul16:".to_string());
        self.emit("  STA $06".to_string());
        self.emit("  STX $07".to_string());
        self.emit("  LDA #0".to_string());
        self.emit("  STA $08".to_string());
        self.emit("  STA $09".to_string());
        self.emit("  LDY #16".to_string());
        self.emit("Math_Mul16Loop:".to_string());
        self.emit("  ASL $08".to_string());
        self.emit("  ROL $09".to_string());
        self.emit("  ASL $06".to_string());
        self.emit("  ROL $07".to_string());
        self.emit("  BCC Math_Mul16NoAdd".to_string());
        self.emit("  CLC".to_string());
        self.emit("  LDA $08".to_string());
        self.emit("  ADC $00".to_string());
        self.emit("  STA $08".to_string());
        self.emit("  LDA $09".to_string());
        self.emit("  ADC $01".to_string());
        self.emit("  STA $09".to_string());
        self.emit("Math_Mul16NoAdd:".to_string());
        self.emit("  DEY".to_string());
        self.emit("  BNE Math_Mul16Loop".to_string());
        self.emit("  LDA $08".to_string());
        self.emit("  LDX $09".to_string());
        self.emit("  RTS".to_string());

        self.emit("Math_Div16:".to_string());
        self.emit("  STA $06".to_string());
        self.emit("  STX $07".to_string());
        self.emit("  LDA #0".to_string());
        self.emit("  STA $08".to_string());
        self.emit("  STA $09".to_string());
        self.emit("  LDY #16".to_string());
        self.emit("Math_Div16Loop:".to_string());
        self.emit("  ASL $06".to_string());
        self.emit("  ROL $07".to_string());
        self.emit("  ROL $08".to_string());
        self.emit("  ROL $09".to_string());
        self.emit("  SEC".to_string());
        self.emit("  LDA $08".to_string());
        self.emit("  SBC $00".to_string());
        self.emit("  PHA".to_string());
        self.emit("  LDA $09".to_string());
        self.emit("  SBC $01".to_string());
        self.emit("  BCC Math_Div16Skip".to_string());
        self.emit("  STA $09".to_string());
        self.emit("  PLA".to_string());
        self.emit("  STA $08".to_string());
        self.emit("  INC $06".to_string());
        self.emit("  JMP Math_Div16Next".to_string());
        self.emit("Math_Div16Skip:".to_string());
        self.emit("  PLA".to_string());
        self.emit("Math_Div16Next:".to_string());
        self.emit("  DEY".to_string());
        self.emit("  BNE Math_Div16Loop".to_string());
        self.emit("  LDA $06".to_string());
        self.emit("  LDX $07".to_string());
        self.emit("  RTS".to_string());

        self.emit("Runtime_ReadByte:".to_string());
        self.emit("  LDY #0".to_string());
        self.emit("  LDA ($04), Y".to_string());
        self.emit("  INC $04".to_string());
        self.emit("  BNE Runtime_ReadByte_Done".to_string());
        self.emit("  INC $05".to_string());
        self.emit("Runtime_ReadByte_Done:".to_string());
        self.emit("  RTS".to_string());

        self.emit("Runtime_ReadString:".to_string());
        self.emit("  LDA $04".to_string());
        self.emit("  LDX $05".to_string());
        self.emit("  PHA".to_string());
        self.emit("  TXA".to_string());
        self.emit("  PHA".to_string());
        self.emit("  LDY #0".to_string());
        self.emit("Runtime_ReadString_Loop:".to_string());
        self.emit("  LDA ($04), Y".to_string());
        self.emit("  INC $04".to_string());
        self.output
            .push("  BNE Runtime_ReadString_NoWrap".to_string());
        self.emit("  INC $05".to_string());
        self.emit("Runtime_ReadString_NoWrap:".to_string());
        self.emit("  CMP #0".to_string());
        self.output
            .push("  BNE Runtime_ReadString_Loop".to_string());
        self.emit("  PLA".to_string());
        self.emit("  TAX".to_string());
        self.emit("  PLA".to_string());
        self.emit("  RTS".to_string());

        self.emit("Math_Div16_Signed:".to_string());
        self.emit("  STA $06".to_string());
        self.emit("  STX $07".to_string());
        self.emit("  STX $0B".to_string());
        self.emit("  LDA $01".to_string());
        self.emit("  STA $0C".to_string());
        self.emit("  EOR $0B".to_string());
        self.emit("  STA $0A".to_string());
        self.emit("  LDA $06".to_string());
        self.emit("  LDX $07".to_string());
        self.emit("  LDY $0B".to_string());
        self.output
            .push("  BPL Math_DivSigned_AbsDividend_Done".to_string());
        self.emit("  EOR #$FF".to_string());
        self.emit("  CLC".to_string());
        self.emit("  ADC #1".to_string());
        self.emit("  PHA".to_string());
        self.emit("  TXA".to_string());
        self.emit("  EOR #$FF".to_string());
        self.emit("  ADC #0".to_string());
        self.emit("  TAX".to_string());
        self.emit("  PLA".to_string());
        self.output
            .push("Math_DivSigned_AbsDividend_Done:".to_string());
        self.emit("  PHA".to_string());
        self.emit("  TXA".to_string());
        self.emit("  PHA".to_string());
        self.emit("  LDA $0C".to_string());
        self.emit("  BPL Math_DivSigned_Do".to_string());
        self.emit("  LDA $00".to_string());
        self.emit("  EOR #$FF".to_string());
        self.emit("  CLC".to_string());
        self.emit("  ADC #1".to_string());
        self.emit("  STA $00".to_string());
        self.emit("  LDA $01".to_string());
        self.emit("  EOR #$FF".to_string());
        self.emit("  ADC #0".to_string());
        self.emit("  STA $01".to_string());
        self.emit("Math_DivSigned_Do:".to_string());
        self.emit("  PLA".to_string());
        self.emit("  TAX".to_string());
        self.emit("  PLA".to_string());
        self.emit("  JSR Math_Div16".to_string());
        self.emit("  LDY $0A".to_string());
        self.output
            .push("  BPL Math_DivSigned_CheckRem".to_string());
        self.emit("  EOR #$FF".to_string());
        self.emit("  CLC".to_string());
        self.emit("  ADC #1".to_string());
        self.emit("  PHA".to_string());
        self.emit("  TXA".to_string());
        self.emit("  EOR #$FF".to_string());
        self.emit("  ADC #0".to_string());
        self.emit("  TAX".to_string());
        self.emit("  PLA".to_string());
        self.emit("Math_DivSigned_CheckRem:".to_string());
        self.emit("  LDY $0B".to_string());
        self.emit("  BPL Math_DivSigned_Done".to_string());
        self.emit("  PHA".to_string());
        self.emit("  TXA".to_string());
        self.emit("  PHA".to_string());
        self.emit("  LDA $08".to_string());
        self.emit("  EOR #$FF".to_string());
        self.emit("  CLC".to_string());
        self.emit("  ADC #1".to_string());
        self.emit("  STA $08".to_string());
        self.emit("  LDA $09".to_string());
        self.emit("  EOR #$FF".to_string());
        self.emit("  ADC #0".to_string());
        self.emit("  STA $09".to_string());
        self.emit("  PLA".to_string());
        self.emit("  TAX".to_string());
        self.emit("  PLA".to_string());
        self.emit("Math_DivSigned_Done:".to_string());
        self.emit("  RTS".to_string());

        self.emit("Math_Shl8:".to_string());
        self.emit("  LDY $00".to_string());
        self.emit("  BEQ Math_Shl8_Done".to_string());
        self.emit("Math_Shl8_Loop:".to_string());
        self.emit("  ASL".to_string());
        self.emit("  DEY".to_string());
        self.emit("  BNE Math_Shl8_Loop".to_string());
        self.emit("Math_Shl8_Done:".to_string());
        self.emit("  RTS".to_string());

        self.emit("Math_Shr8:".to_string());
        self.emit("  LDY $00".to_string());
        self.emit("  BEQ Math_Shr8_Done".to_string());
        self.emit("Math_Shr8_Loop:".to_string());
        self.emit("  LSR".to_string());
        self.emit("  DEY".to_string());
        self.emit("  BNE Math_Shr8_Loop".to_string());
        self.emit("Math_Shr8_Done:".to_string());
        self.emit("  RTS".to_string());

        self.emit("Math_Shl16:".to_string());
        self.emit("  LDY $00".to_string());
        self.emit("  BEQ Math_Shl16_Done".to_string());
        self.emit("Math_Shl16_Loop:".to_string());
        self.emit("  ASL".to_string());
        self.emit("  PHA".to_string());
        self.emit("  TXA".to_string());
        self.emit("  ROL".to_string());
        self.emit("  TAX".to_string());
        self.emit("  PLA".to_string());
        self.emit("  DEY".to_string());
        self.emit("  BNE Math_Shl16_Loop".to_string());
        self.emit("Math_Shl16_Done:".to_string());
        self.emit("  RTS".to_string());

        self.emit("Math_Shr16:".to_string());
        self.emit("  LDY $00".to_string());
        self.emit("  BEQ Math_Shr16_Done".to_string());
        self.emit("Math_Shr16_Loop:".to_string());
        self.emit("  PHA".to_string()); // Save Low
        self.emit("  TXA".to_string());
        self.emit("  LSR".to_string());
        self.emit("  TAX".to_string());
        self.emit("  PLA".to_string()); // Restore Low
        self.emit("  ROR".to_string());
        self.emit("  DEY".to_string());
        self.emit("  BNE Math_Shr16_Loop".to_string());
        self.emit("Math_Shr16_Done:".to_string());
        self.emit("  RTS".to_string());

        // Runtime_Random
        self.emit("Runtime_Random:".to_string());
        self.emit("  LDY #8".to_string());
        self.emit("Random_Loop:".to_string());
        self.emit("  LDA $E2".to_string());
        self.emit("  ASL".to_string());
        self.emit("  ROL $E3".to_string());
        self.emit("  BCC Random_NoEor".to_string());
        self.emit("  EOR #$39".to_string());
        self.emit("Random_NoEor:".to_string());
        self.emit("  STA $E2".to_string());
        self.emit("  DEY".to_string());
        self.emit("  BNE Random_Loop".to_string());
        self.emit("  LDA $E2".to_string());
        self.emit("  LDX $E3".to_string());
        self.emit("  RTS".to_string());

        // Runtime_Randomize
        self.emit("Runtime_Randomize:".to_string());
        self.emit("  STA $E2".to_string());
        self.emit("  STX $E3".to_string());
        self.emit("  ORA $E3".to_string());
        self.emit("  BNE Randomize_Ret".to_string());
        self.emit("  LDA #$FF".to_string());
        self.emit("  STA $E2".to_string());
        self.emit("  STA $E3".to_string());
        self.emit("Randomize_Ret:".to_string());
        self.emit("  RTS".to_string());
    }

    fn generate_animation_helpers(&mut self) {
        self.emit("; --- Animation Helpers ---".to_string());

        // Runtime_Anim_Update
        self.emit("Runtime_Anim_Update:".to_string());

        // 1. Check Finished (Offset 4)
        self.emit("  LDY #4".to_string());
        self.emit("  LDA ($02), Y".to_string());
        self.emit("  BNE Anim_Ret".to_string()); // If Finished != 0, return

        // 2. Dec Timer (Offset 3)
        self.emit("  DEY".to_string()); // Y=3
        self.emit("  LDA ($02), Y".to_string());
        self.emit("  SEC".to_string());
        self.emit("  SBC #1".to_string());
        self.emit("  STA ($02), Y".to_string());
        self.emit("  BNE Anim_Ret".to_string()); // If Timer > 0, return

        // 3. Timer Expired. Advance Frame.
        // Load Index (Offset 2)
        self.emit("  DEY".to_string()); // Y=2
        self.emit("  LDA ($02), Y".to_string());
        self.emit("  CLC".to_string());
        self.emit("  ADC #1".to_string());
        self.emit("  STA ($02), Y".to_string()); // Update Index (Tentative)
        self.emit("  TAX".to_string()); // X = New Index

        // Load Anim Ptr (Offset 0) to $04/$05
        self.emit("  LDY #0".to_string());
        self.emit("  LDA ($02), Y".to_string());
        self.emit("  STA $04".to_string());
        self.emit("  INY".to_string());
        self.emit("  LDA ($02), Y".to_string());
        self.emit("  STA $05".to_string());

        // Read Count (Anim+0)
        self.emit("  LDY #0".to_string());
        self.emit("  LDA ($04), Y".to_string());
        self.emit("  STA $06".to_string()); // Count in $06

        // Compare Index vs Count
        self.emit("  CPX $06".to_string()); // New Index vs Count
        self.emit("  BCC Anim_LoadFrame".to_string()); // If Index < Count, OK

        // End of Animation. Check Loop (Anim+1)
        self.emit("  INY".to_string()); // Y=1
        self.emit("  LDA ($04), Y".to_string());
        self.emit("  BNE Anim_Loop".to_string());

        // No Loop.
        self.emit("  DEC $06".to_string()); // Count - 1
        self.emit("  LDA $06".to_string()); // Last Index
        self.emit("  LDY #2".to_string());
        self.emit("  STA ($02), Y".to_string()); // Store Last Index

        self.emit("  LDA #$FF".to_string());
        self.emit("  LDY #4".to_string());
        self.emit("  STA ($02), Y".to_string()); // Finished = $FF (True)
        self.emit("  JMP Anim_Ret".to_string());

        self.emit("Anim_Loop:".to_string());
        self.emit("  LDA #0".to_string());
        self.emit("  LDY #2".to_string());
        self.emit("  STA ($02), Y".to_string()); // Index = 0
        self.emit("  TAX".to_string()); // X = 0

        self.emit("Anim_LoadFrame:".to_string());
        // Preserve the high byte when a three-byte frame crosses a page.
        self.emit("  TXA".to_string());
        self.emit("  JSR Runtime_Anim_FramePointer".to_string());
        self.emit("  LDY #4".to_string()); // Two header bytes plus two pointer bytes.

        // Read Duration
        self.emit("  LDA ($04), Y".to_string());
        self.emit("  LDY #3".to_string());
        self.emit("  STA ($02), Y".to_string()); // Store new timer

        self.emit("Anim_Ret:".to_string());
        self.emit("  RTS".to_string());

        // Runtime_Anim_Draw
        // Input: $14 (X), $15 (Y), $16/$17 (State Ptr)
        for (label, draw) in [
            ("Runtime_Anim_Draw", "Runtime_SpriteDraw"),
            ("Runtime_Anim_DrawVariant", "Runtime_SpriteDrawVariant"),
        ] {
            self.emit(format!("{label}:"));
            self.emit("  JSR Runtime_Anim_DrawFrame".into());
            self.emit(format!("  JMP {draw}"));
        }
        // Both draw paths resolve the same state/frame table. Share this code in the
        // fixed bank; tail-calling the renderer keeps the extra call cost small.
        self.emit("Runtime_Anim_DrawFrame:".into());

        // Copy State Ptr to $02/$03
        self.emit("  LDA $16".to_string());
        self.emit("  STA $02".to_string());
        self.emit("  LDA $17".to_string());
        self.emit("  STA $03".to_string());

        // Load Anim Ptr (Offset 0) to $04/$05
        self.emit("  LDY #0".to_string());
        self.emit("  LDA ($02), Y".to_string());
        self.emit("  STA $04".to_string());
        self.emit("  INY".to_string());
        self.emit("  LDA ($02), Y".to_string());
        self.emit("  STA $05".to_string());

        // Load Index (Offset 2)
        self.emit("  INY".to_string()); // Y=2
        self.emit("  LDA ($02), Y".to_string()); // A = Index

        self.emit("  JSR Runtime_Anim_FramePointer".to_string());
        self.emit("  LDY #2".to_string()); // Skip the animation header.

        // Read MetaPtr
        self.emit("  LDA ($04), Y".to_string());
        self.emit("  STA $16".to_string()); // Store in Argument 2 Low
        self.emit("  INY".to_string());
        self.emit("  LDA ($04), Y".to_string());
        self.emit("  STA $17".to_string()); // Store in Argument 2 High

        // Return the frame pointer to the selected renderer wrapper.
        self.emit("  RTS".to_string());

        // Add A*3 to the animation pointer using 16-bit arithmetic. Metadata stores
        // up to 255 frames; indexing it solely with Y truncated frame 84 onward.
        self.emit("Runtime_Anim_FramePointer:".to_string());
        self.emit("  STA $06".to_string());
        self.emit("  LDY #0".to_string());
        self.emit("  STY $07".to_string());
        self.emit("  ASL".to_string());
        self.emit("  ROL $07".to_string());
        self.emit("  CLC".to_string());
        self.emit("  ADC $06".to_string());
        self.emit("  STA $06".to_string());
        self.emit("  LDA $07".to_string());
        self.emit("  ADC #0".to_string());
        self.emit("  STA $07".to_string());
        self.emit("  LDA $04".to_string());
        self.emit("  CLC".to_string());
        self.emit("  ADC $06".to_string());
        self.emit("  STA $04".to_string());
        self.emit("  LDA $05".to_string());
        self.emit("  ADC $07".to_string());
        self.emit("  STA $05".to_string());
        self.emit("  RTS".to_string());

        self.emit("".to_string());
    }

    fn generate_collision_helpers(&mut self) {
        self.emit("; --- Collision Helpers ---".to_string());

        self.emit("Runtime_Collision_Rect:".to_string());
        self.emit("  TSX".to_string());

        // Empty rectangles never overlap, even when their origin lies inside another box.
        for (low, high) in [
            (0x010e, 0x010d),
            (0x010c, 0x010b),
            (0x0106, 0x0105),
            (0x0104, 0x0103),
        ] {
            self.emit(format!("  LDA ${low:04X},X"));
            self.emit(format!("  ORA ${high:04X},X"));
            self.emit("  BEQ Collision_Rect_Fail".to_string());
        }

        // Check 1: x1 < x2 + w2
        self.emit("  CLC".to_string());
        self.emit("  LDA $010A, X".to_string()); // X2_Low
        self.emit("  ADC $0106, X".to_string()); // W2_Low
        self.emit("  STA $00".to_string());
        self.emit("  LDA $0109, X".to_string()); // X2_High
        self.emit("  ADC $0105, X".to_string()); // W2_High
        self.emit("  STA $01".to_string());
        // A carry represents an endpoint beyond every unsigned 16-bit position.
        self.emit("  BCS Collision_Rect_Check2".to_string());

        self.emit("  LDA $0112, X".to_string()); // X1_Low
        self.emit("  CMP $00".to_string());
        self.emit("  LDA $0111, X".to_string()); // X1_High
        self.emit("  SBC $01".to_string());
        self.emit("  BCS Collision_Rect_Fail".to_string());

        // Check 2: x1 + w1 > x2
        self.emit("Collision_Rect_Check2:".to_string());
        self.emit("  CLC".to_string());
        self.emit("  LDA $0112, X".to_string()); // X1_Low
        self.emit("  ADC $010E, X".to_string()); // W1_Low
        self.emit("  STA $00".to_string());
        self.emit("  LDA $0111, X".to_string()); // X1_High
        self.emit("  ADC $010D, X".to_string()); // W1_High
        self.emit("  STA $01".to_string());
        self.emit("  BCS Collision_Rect_Check3".to_string());

        self.emit("  LDA $010A, X".to_string()); // X2_Low
        self.emit("  CMP $00".to_string());
        self.emit("  LDA $0109, X".to_string()); // X2_High
        self.emit("  SBC $01".to_string());
        self.emit("  BCS Collision_Rect_Fail".to_string());

        // Check 3: y1 < y2 + h2
        self.emit("Collision_Rect_Check3:".to_string());
        self.emit("  CLC".to_string());
        self.emit("  LDA $0108, X".to_string()); // Y2_Low
        self.emit("  ADC $0104, X".to_string()); // H2_Low
        self.emit("  STA $00".to_string());
        self.emit("  LDA $0107, X".to_string()); // Y2_High
        self.emit("  ADC $0103, X".to_string()); // H2_High
        self.emit("  STA $01".to_string());
        self.emit("  BCS Collision_Rect_Check4".to_string());

        self.emit("  LDA $0110, X".to_string()); // Y1_Low
        self.emit("  CMP $00".to_string());
        self.emit("  LDA $010F, X".to_string()); // Y1_High
        self.emit("  SBC $01".to_string());
        self.emit("  BCS Collision_Rect_Fail".to_string());

        // Check 4: y1 + h1 > y2
        self.emit("Collision_Rect_Check4:".to_string());
        self.emit("  CLC".to_string());
        self.emit("  LDA $0110, X".to_string()); // Y1_Low
        self.emit("  ADC $010C, X".to_string()); // H1_Low
        self.emit("  STA $00".to_string());
        self.emit("  LDA $010F, X".to_string()); // Y1_High
        self.emit("  ADC $010B, X".to_string()); // H1_High
        self.emit("  STA $01".to_string());
        self.emit("  BCS Collision_Rect_True".to_string());

        self.emit("  LDA $0108, X".to_string()); // Y2_Low
        self.emit("  CMP $00".to_string());
        self.emit("  LDA $0107, X".to_string()); // Y2_High
        self.emit("  SBC $01".to_string());
        self.emit("  BCS Collision_Rect_Fail".to_string());

        self.emit("Collision_Rect_True:".to_string());
        self.emit("  LDA #$FF".to_string());
        self.emit("  LDX #0".to_string());
        self.emit("  RTS".to_string());

        self.emit("Collision_Rect_Fail:".to_string());
        self.emit("  JMP Collision_False".to_string());

        self.emit("Runtime_Collision_Point:".to_string());
        self.emit("  TSX".to_string());

        // Check 1: px >= rx
        self.emit("  LDA $010E, X".to_string()); // PX_L
        self.emit("  CMP $010A, X".to_string()); // RX_L
        self.emit("  LDA $010D, X".to_string()); // PX_H
        self.emit("  SBC $0109, X".to_string()); // RX_H
        self.emit("  BCC Collision_Point_Fail".to_string());

        // Check 2: px < rx + rw
        self.emit("  CLC".to_string());
        self.emit("  LDA $010A, X".to_string()); // RX_L
        self.emit("  ADC $0106, X".to_string()); // RW_L
        self.emit("  STA $00".to_string());
        self.emit("  LDA $0109, X".to_string()); // RX_H
        self.emit("  ADC $0105, X".to_string()); // RW_H
        self.emit("  STA $01".to_string());
        self.emit("  BCS Collision_Point_Check3".to_string());

        self.emit("  LDA $010E, X".to_string()); // PX_L
        self.emit("  CMP $00".to_string());
        self.emit("  LDA $010D, X".to_string()); // PX_H
        self.emit("  SBC $01".to_string());
        self.emit("  BCS Collision_Point_Fail".to_string());

        // Check 3: py >= ry
        self.emit("Collision_Point_Check3:".to_string());
        self.emit("  LDA $010C, X".to_string()); // PY_L
        self.emit("  CMP $0108, X".to_string()); // RY_L
        self.emit("  LDA $010B, X".to_string()); // PY_H
        self.emit("  SBC $0107, X".to_string()); // RY_H
        self.emit("  BCC Collision_Point_Fail".to_string());

        // Check 4: py < ry + rh
        self.emit("  CLC".to_string());
        self.emit("  LDA $0108, X".to_string()); // RY_L
        self.emit("  ADC $0104, X".to_string()); // RH_L
        self.emit("  STA $00".to_string());
        self.emit("  LDA $0107, X".to_string()); // RY_H
        self.emit("  ADC $0103, X".to_string()); // RH_H
        self.emit("  STA $01".to_string());
        self.emit("  BCS Collision_Point_True".to_string());

        self.emit("  LDA $010C, X".to_string()); // PY_L
        self.emit("  CMP $00".to_string());
        self.emit("  LDA $010B, X".to_string()); // PY_H
        self.emit("  SBC $01".to_string());
        self.emit("  BCS Collision_Point_Fail".to_string());

        self.emit("Collision_Point_True:".to_string());
        self.emit("  LDA #$FF".to_string());
        self.emit("  LDX #0".to_string());
        self.emit("  RTS".to_string());

        self.emit("Collision_Point_Fail:".to_string());
        self.emit("  JMP Collision_False".to_string());

        self.emit("Runtime_Collision_Tile:".to_string());
        self.emit("  TSX".to_string());
        // Args: X (5,6), Y (3,4) (High, Low)
        // Collision.Tile uses local visible-screen pixels. Reject coordinates
        // outside 256x240 rather than aliasing the map or its palette attributes.
        self.emit("  LDA $0105,X".to_string());
        self.emit("  ORA $0103,X".to_string());
        self.emit("  BNE Collision_False".to_string());
        self.emit("  LDA $0104,X".to_string());
        self.emit("  CMP #240".to_string());
        self.emit("  BCS Collision_False".to_string());
        self.emit("  LDA $0106, X".to_string()); // X_L
        self.emit("  LSR".to_string());
        self.emit("  LSR".to_string());
        self.emit("  LSR".to_string());
        self.emit("  STA $00".to_string()); // TileX

        self.emit("  LDA $0104, X".to_string()); // Y_L
        self.emit("  LSR".to_string());
        self.emit("  LSR".to_string());
        self.emit("  LSR".to_string());
        self.emit("  STA $01".to_string()); // TileY

        // Offset = TileY * 32 + TileX
        self.emit("  LDA $01".to_string());
        self.emit("  STA $02".to_string()); // Low
        self.emit("  LDA #0".to_string());
        self.emit("  STA $03".to_string()); // High

        // x32
        for _ in 0..5 {
            self.emit("  ASL $02".to_string());
            self.emit("  ROL $03".to_string());
        }

        self.emit("  LDA $02".to_string());
        self.emit("  CLC".to_string());
        self.emit("  ADC $00".to_string()); // + TileX
        self.emit("  STA $02".to_string());
        self.emit("  LDA $03".to_string());
        self.emit("  ADC #0".to_string());
        self.emit("  STA $03".to_string());

        // World.Load selects the same map for graphics and collision. The helper
        // falls back to the first map until a world cell has been loaded.
        self.emit("  JSR Runtime_World_ReadTile".to_string());
        self.emit("  RTS".to_string());

        self.emit("Collision_False:".to_string());
        self.emit("  LDA #0".to_string());
        self.emit("  LDX #0".to_string());
        self.emit("  RTS".to_string());
        self.emit("".to_string());
    }

    fn generate_scroll_arguments(&mut self, args: &[Expression]) -> Result<(), String> {
        // Coordinate and optional attribute pointer must survive array address evaluation.
        let coord_type = self.generate_expression(&args[0])?;
        self.emit("  PHA".to_string());
        if coord_type == DataType::Word {
            self.emit("  TXA".to_string());
        } else {
            self.emit("  LDA #0".to_string());
        }
        self.emit("  PHA".to_string());
        if let Some(attributes) = args.get(2) {
            self.generate_address_expression(attributes)?;
            self.emit("  LDA $02".to_string());
            self.emit("  PHA".to_string());
            self.emit("  LDA $03".to_string());
            self.emit("  PHA".to_string());
        }
        self.generate_address_expression(&args[1])?;
        self.emit("  LDA $02".to_string());
        self.emit("  STA $00".to_string());
        self.emit("  LDA $03".to_string());
        self.emit("  STA $01".to_string());
        if args.len() == 3 {
            self.emit("  PLA".to_string());
            self.emit("  STA $09".to_string());
            self.emit("  PLA".to_string());
            self.emit("  STA $08".to_string());
        } else {
            self.emit("  LDA #0".to_string());
            self.emit("  STA $08".to_string());
            self.emit("  STA $09".to_string());
        }
        self.emit("  PLA".to_string());
        self.emit("  STA $03".to_string());
        self.emit("  PLA".to_string());
        self.emit("  STA $02".to_string());
        Ok(())
    }

    fn generate_scroll_helpers(&mut self) {
        self.emit("; --- Scroll Helpers ---".to_string());

        self.emit("Runtime_Scroll_LoadColumn:".to_string());

        // Arguments are restored before JSR: $00/$01 is data, $02/$03 is X.
        // A busy upload therefore returns without disturbing the caller's stack.
        // Check Buffer
        self.output
            .push(format!("  LDA ${:04X}", VBLANK_BUFFER_START));
        self.emit("  BNE Scroll_LoadColumn_Done".to_string());

        // Calculate Target High ($05)
        // Base $20 or $24 based on X High Bit 0
        self.emit("  LDA $03".to_string());
        self.emit("  AND #1".to_string());
        self.emit("  BEQ Scroll_Base2000".to_string());
        self.emit("  LDA #$24".to_string());
        self.emit("  JMP Scroll_BaseStore".to_string());
        self.emit("Scroll_Base2000:".to_string());
        self.emit("  LDA #$20".to_string());
        self.emit("Scroll_BaseStore:".to_string());
        self.emit("  STA $05".to_string());

        // Calculate Target Low ($04) = (X Low / 8)
        self.emit("  LDA $02".to_string());
        self.emit("  LSR".to_string());
        self.emit("  LSR".to_string());
        self.emit("  LSR".to_string());
        self.emit("  STA $04".to_string());

        // Write Header
        self.emit("  LDA #0".to_string());
        self.output
            .push(format!("  STA ${:04X}", VBLANK_BUFFER_START + 1)); // Type
        self.emit("  LDA $05".to_string());
        self.output
            .push(format!("  STA ${:04X}", VBLANK_BUFFER_START + 2)); // High
        self.emit("  LDA $04".to_string());
        self.output
            .push(format!("  STA ${:04X}", VBLANK_BUFFER_START + 3)); // Low

        // Copy Data
        self.emit("  LDY #0".to_string());
        self.emit("Scroll_CopyLoop:".to_string());
        self.emit("  LDA ($00), Y".to_string());
        self.output
            .push(format!("  STA ${:04X}, Y", VBLANK_BUFFER_START + 4));
        self.emit("  INY".to_string());
        self.emit("  CPY #30".to_string());
        self.emit("  BNE Scroll_CopyLoop".to_string());

        // Eight authored attribute bytes cover the entire seam. They are queued
        // separately so tile and palette uploads each fit inside a VBlank.
        self.emit("  LDA $05".to_string());
        self.emit("  ORA #3".to_string());
        self.emit(format!("  STA ${:04X}", VBLANK_BUFFER_START + 36));
        self.emit("  LDA $04".to_string());
        self.emit("  LSR".to_string());
        self.emit("  LSR".to_string());
        self.emit("  ORA #$C0".to_string());
        self.emit(format!("  STA ${:04X}", VBLANK_BUFFER_START + 37));
        self.emit("  JSR Runtime_Scroll_QueueAttributes".to_string());

        // Set Flag
        self.emit("  LDA #1".to_string());
        self.output
            .push(format!("  STA ${:04X}", VBLANK_BUFFER_START));

        self.emit("Scroll_LoadColumn_Done:".to_string());
        self.emit("  RTS".to_string());
        self.emit("".to_string());

        self.emit("Runtime_Scroll_LoadRow:".to_string());

        // Same register contract as LoadColumn, with Y in $02/$03.
        // Check Buffer
        self.output
            .push(format!("  LDA ${:04X}", VBLANK_BUFFER_START));
        self.emit("  BNE Scroll_LoadRow_Done".to_string());

        // World Y uses 240 visible pixels per screen, not a padded 256-pixel page.
        self.emit("  JSR Runtime_Scroll_NormalizeY".to_string());

        // Calculate Target High ($05)
        // (Y Low / 64) -> High Offset
        self.emit("  LDA $02".to_string());
        self.emit("  LSR".to_string());
        self.emit("  LSR".to_string());
        self.emit("  LSR".to_string());
        self.emit("  LSR".to_string());
        self.emit("  LSR".to_string());
        self.emit("  LSR".to_string());
        self.emit("  STA $06".to_string()); // Temp High Offset

        // Base $20 or $28 based on the normalized vertical screen parity.
        self.emit("  LDA $07".to_string());
        self.emit("  AND #1".to_string());
        self.emit("  BEQ Scroll_RowBase2000".to_string());
        self.emit("  LDA #$28".to_string());
        self.emit("  JMP Scroll_RowBaseStore".to_string());
        self.emit("Scroll_RowBase2000:".to_string());
        self.emit("  LDA #$20".to_string());
        self.emit("Scroll_RowBaseStore:".to_string());
        self.emit("  CLC".to_string());
        self.emit("  ADC $06".to_string());
        self.emit("  STA $05".to_string());

        // Calculate Target Low ($04) = (Y Low & F8) << 2
        self.emit("  LDA $02".to_string());
        self.emit("  AND #$F8".to_string());
        self.emit("  ASL".to_string());
        self.emit("  ASL".to_string());
        self.emit("  STA $04".to_string());

        // Write Header
        self.emit("  LDA #1".to_string()); // Type 1 (Row)
        self.output
            .push(format!("  STA ${:04X}", VBLANK_BUFFER_START + 1)); // Type
        self.emit("  LDA $05".to_string());
        self.output
            .push(format!("  STA ${:04X}", VBLANK_BUFFER_START + 2)); // High
        self.emit("  LDA $04".to_string());
        self.output
            .push(format!("  STA ${:04X}", VBLANK_BUFFER_START + 3)); // Low

        // Copy Data
        self.emit("  LDY #0".to_string());
        self.emit("Scroll_RowCopyLoop:".to_string());
        self.emit("  LDA ($00), Y".to_string());
        self.output
            .push(format!("  STA ${:04X}, Y", VBLANK_BUFFER_START + 4));
        self.emit("  INY".to_string());
        self.emit("  CPY #32".to_string()); // 32 tiles for row
        self.emit("  BNE Scroll_RowCopyLoop".to_string());

        self.emit("  LDA $05".to_string());
        self.emit("  AND #$FC".to_string());
        self.emit("  ORA #3".to_string());
        self.emit(format!("  STA ${:04X}", VBLANK_BUFFER_START + 36));
        self.emit("  LDA $02".to_string());
        self.emit("  LSR".to_string());
        self.emit("  LSR".to_string());
        self.emit("  AND #$38".to_string());
        self.emit("  ORA #$C0".to_string());
        self.emit(format!("  STA ${:04X}", VBLANK_BUFFER_START + 37));
        self.emit("  JSR Runtime_Scroll_QueueAttributes".to_string());

        // Set Flag
        self.emit("  LDA #1".to_string());
        self.output
            .push(format!("  STA ${:04X}", VBLANK_BUFFER_START));

        self.emit("Scroll_LoadRow_Done:".to_string());
        self.emit("  RTS".to_string());

        self.emit("Runtime_Scroll_QueueAttributes:".to_string());
        self.emit("  LDA $08".to_string());
        self.emit("  ORA $09".to_string());
        self.emit("  BEQ Scroll_QueueAttributes_Done".to_string());
        self.emit("  LDY #0".to_string());
        self.emit("Scroll_QueueAttributes_Loop:".to_string());
        self.emit("  LDA ($08),Y".to_string());
        self.emit(format!("  STA ${:04X},Y", VBLANK_BUFFER_START + 38));
        self.emit("  INY".to_string());
        self.emit("  CPY #8".to_string());
        self.emit("  BNE Scroll_QueueAttributes_Loop".to_string());
        self.emit(format!("  LDA ${:04X}", VBLANK_BUFFER_START + 1));
        self.emit("  ORA #$80".to_string());
        self.emit(format!("  STA ${:04X}", VBLANK_BUFFER_START + 1));
        self.emit("Scroll_QueueAttributes_Done:".to_string());
        self.emit("  RTS".to_string());

        // Reduce an unsigned WORD world coordinate into 0..239 and nametable parity.
        // Repeated subtraction handles every crossing, including direction changes
        // and coordinates beyond the first 256-byte page.
        self.emit("Runtime_Scroll_NormalizeY:".to_string());
        self.emit("  LDA #0".to_string());
        self.emit("  STA $07".to_string());
        self.emit("Scroll_NormalizeY_Loop:".to_string());
        self.emit("  LDA $03".to_string());
        self.emit("  BNE Scroll_NormalizeY_Subtract".to_string());
        self.emit("  LDA $02".to_string());
        self.emit("  CMP #$F0".to_string());
        self.emit("  BCC Scroll_Row_Gap_Skip".to_string());
        self.emit("Scroll_NormalizeY_Subtract:".to_string());
        self.emit("  LDA $02".to_string());
        self.emit("  SEC".to_string());
        self.emit("  SBC #$F0".to_string());
        self.emit("  STA $02".to_string());
        self.emit("  LDA $03".to_string());
        self.emit("  SBC #0".to_string());
        self.emit("  STA $03".to_string());
        self.emit("  LDA $07".to_string());
        self.emit("  EOR #1".to_string());
        self.emit("  STA $07".to_string());
        self.emit("  JMP Scroll_NormalizeY_Loop".to_string());
        self.emit("Scroll_Row_Gap_Skip:".to_string());
        self.emit("  RTS".to_string());
        self.emit("".to_string());
    }

    fn generate_pool_helpers(&mut self) {
        self.emit("; --- Pool Helpers ---".to_string());

        self.emit("Runtime_Pool_Spawn:".to_string());
        self.emit("  LDA #0".to_string());
        self.emit("  STA $08".to_string());
        self.emit("  STA $09".to_string());

        self.emit("Pool_Spawn_Loop:".to_string());
        self.emit("  LDA $08".to_string());
        self.emit("  CMP $04".to_string());
        self.emit("  LDA $09".to_string());
        self.emit("  SBC $05".to_string());
        self.emit("  BCS Pool_Full".to_string());

        self.emit("  LDY #0".to_string());
        self.emit("  LDA ($02), Y".to_string());
        self.emit("  BEQ Pool_Found".to_string());

        self.emit("  LDA $02".to_string());
        self.emit("  CLC".to_string());
        self.emit("  ADC $06".to_string());
        self.emit("  STA $02".to_string());
        self.emit("  LDA $03".to_string());
        self.emit("  ADC #0".to_string());
        self.emit("  STA $03".to_string());

        self.emit("  INC $08".to_string());
        self.emit("  BNE Pool_Inc_Skip".to_string());
        self.emit("  INC $09".to_string());
        self.emit("Pool_Inc_Skip:".to_string());
        self.emit("  JMP Pool_Spawn_Loop".to_string());

        self.emit("Pool_Found:".to_string());
        // Reused entities must start from the same zeroed state as fresh slots.
        // Otherwise stale position/velocity fields survive despawn and respawn.
        self.emit("  LDY #0".to_string());
        self.emit("  LDA #0".to_string());
        self.emit("Pool_InitializeSlot:".to_string());
        self.emit("  STA ($02),Y".to_string());
        self.emit("  INY".to_string());
        self.emit("  CPY $06".to_string());
        self.emit("  BNE Pool_InitializeSlot".to_string());
        self.emit("  LDY #0".to_string());
        self.emit("  LDA #1".to_string());
        self.emit("  STA ($02), Y".to_string());
        self.emit("  LDA $08".to_string());
        self.emit("  LDX $09".to_string());
        self.emit("  RTS".to_string());

        self.emit("Pool_Full:".to_string());
        self.emit("  LDA #$FF".to_string());
        self.emit("  LDX #$FF".to_string());
        self.emit("  RTS".to_string());

        self.emit("Runtime_Pool_Despawn:".to_string());
        self.emit("  PLA".to_string());
        self.emit("  STA $0A".to_string());
        self.emit("  PLA".to_string());
        self.emit("  STA $0B".to_string());
        self.emit("  PLA".to_string());
        self.emit("  STA $09".to_string());
        self.emit("  PLA".to_string());
        self.emit("  STA $08".to_string());
        self.emit("  LDA $0B".to_string());
        self.emit("  PHA".to_string());
        self.emit("  LDA $0A".to_string());
        self.emit("  PHA".to_string());

        self.emit("  LDA $08".to_string());
        self.emit("  LDX $09".to_string());

        self.emit("  PHA".to_string());
        self.emit("  LDA $06".to_string());
        self.emit("  STA $00".to_string());
        self.emit("  LDA #0".to_string());
        self.emit("  STA $01".to_string());
        self.emit("  PLA".to_string());

        self.emit("  JSR Math_Mul16".to_string());

        self.emit("  CLC".to_string());
        self.emit("  ADC $02".to_string());
        self.emit("  STA $02".to_string());
        self.emit("  TXA".to_string());
        self.emit("  ADC $03".to_string());
        self.emit("  STA $03".to_string());

        self.emit("  LDA #0".to_string());
        self.emit("  LDY #0".to_string());
        self.emit("  STA ($02), Y".to_string());

        self.emit("  RTS".to_string());
        self.emit("".to_string());
    }

    fn generate_string_helpers(&mut self) {
        self.emit("; --- String Helpers ---".to_string());

        // Runtime_GetHeapSlot
        // Returns ptr to next 16-byte slot in A (Low), X (High)
        // Uses $0E (Index), $02/$03 (Temp)
        self.emit("Runtime_GetHeapSlot:".to_string());
        self.emit("  INC $0E".to_string());
        self.emit("  LDA $0E".to_string());
        self.emit("  AND #$1F".to_string()); // 32 slots (0-31)
        self.emit("  STA $0E".to_string());

        // Calculate High Byte Offset: (Index & 0x10) >> 4
        self.emit("  AND #$10".to_string());
        self.emit("  LSR".to_string());
        self.emit("  LSR".to_string());
        self.emit("  LSR".to_string());
        self.emit("  LSR".to_string());
        self.emit("  STA $03".to_string()); // Temp High Offset

        // Calculate Low Byte: (Index & 0x0F) << 4
        self.emit("  LDA $0E".to_string());
        self.emit("  AND #$0F".to_string());
        self.emit("  ASL".to_string());
        self.emit("  ASL".to_string());
        self.emit("  ASL".to_string());
        self.emit("  ASL".to_string());

        // Add Base Low
        self.emit("  CLC".to_string());
        self.output
            .push(format!("  ADC #${:02X}", (STRING_HEAP_START & 0xFF) as u8));
        self.emit("  STA $02".to_string()); // Result Low

        // Add Base High + Carry
        self.emit("  LDA $03".to_string());
        self.emit(format!(
            "  ADC #${:02X}",
            ((STRING_HEAP_START >> 8) & 0xFF) as u8
        ));
        self.emit("  TAX".to_string()); // Result High in X

        self.emit("  LDA $02".to_string()); // Result Low in A
        self.emit("  RTS".to_string());

        // Runtime_Asc
        // Input: A/X = String Ptr
        // Output: A = First Char (Word returned as A/0)
        self.emit("Runtime_Asc:".to_string());
        self.emit("  STA $02".to_string());
        self.emit("  STX $03".to_string());
        self.emit("  LDY #0".to_string());
        self.emit("  LDA ($02),Y".to_string());
        self.emit("  LDX #0".to_string());
        self.emit("  RTS".to_string());

        // Runtime_Val
        // Input: A/X = String Ptr
        // Output: A/X = Value
        self.emit("Runtime_Val:".to_string());
        self.emit("  STA $02".to_string());
        self.emit("  STX $03".to_string());
        self.emit("  LDA #0".to_string());
        self.emit("  STA $0A".to_string()); // Result Low
        self.emit("  STA $0B".to_string()); // Result High
        self.emit("  STA $0C".to_string()); // Sign (0=Pos, 1=Neg)
        self.emit("  LDY #0".to_string());
        self.emit("  LDA ($02),Y".to_string());
        self.emit("  CMP #$2D".to_string()); // '-'
        self.emit("  BNE Val_Loop".to_string());
        self.emit("  INC $0C".to_string());
        self.emit("  INY".to_string());
        self.emit("Val_Loop:".to_string());
        self.emit("  LDA ($02),Y".to_string());
        self.emit("  CMP #0".to_string());
        self.emit("  BEQ Val_Done".to_string());
        self.emit("  SEC".to_string());
        self.emit("  SBC #$30".to_string()); // - '0'
        self.emit("  BMI Val_Done".to_string()); // Not digit
        self.emit("  CMP #10".to_string());
        self.emit("  BCS Val_Done".to_string()); // Not digit
        self.emit("  PHA".to_string()); // Save digit
                                        // The math helper changes Y; keep the parser cursor on
                                        // the stack so multi-digit conversions continue through the string.
        self.emit("  TYA".to_string());
        self.emit("  PHA".to_string());
        // Result * 10
        self.emit("  LDA $0A".to_string());
        self.emit("  LDX $0B".to_string());
        self.emit("  STA $00".to_string()); // Mul16 expects args in A/X and $00/$01... wait, Mul16 takes A/X * $00/$01
        self.emit("  STX $01".to_string());
        self.emit("  LDA #10".to_string());
        self.emit("  LDX #0".to_string());
        self.emit("  JSR Math_Mul16".to_string());
        self.emit("  STA $0A".to_string());
        self.emit("  STX $0B".to_string());
        self.emit("  PLA".to_string());
        self.emit("  TAY".to_string());
        // Add digit
        self.emit("  PLA".to_string());
        self.emit("  CLC".to_string());
        self.emit("  ADC $0A".to_string());
        self.emit("  STA $0A".to_string());
        self.emit("  LDA #0".to_string());
        self.emit("  ADC $0B".to_string());
        self.emit("  STA $0B".to_string());
        self.emit("  INY".to_string());
        self.emit("  JMP Val_Loop".to_string());
        self.emit("Val_Done:".to_string());
        self.emit("  LDA $0C".to_string());
        self.emit("  BEQ Val_Ret".to_string());
        // Negate
        self.emit("  LDA $0A".to_string());
        self.emit("  EOR #$FF".to_string());
        self.emit("  CLC".to_string());
        self.emit("  ADC #1".to_string());
        self.emit("  STA $0A".to_string());
        self.emit("  LDA $0B".to_string());
        self.emit("  EOR #$FF".to_string());
        self.emit("  ADC #0".to_string());
        self.emit("  STA $0B".to_string());
        self.emit("Val_Ret:".to_string());
        self.emit("  LDA $0A".to_string());
        self.emit("  LDX $0B".to_string());
        self.emit("  RTS".to_string());

        // Runtime_Chr
        // Input: A (Char)
        // Output: A/X (String Ptr)
        self.emit("Runtime_Chr:".to_string());
        self.emit("  PHA".to_string());
        self.emit("  JSR Runtime_GetHeapSlot".to_string());
        self.emit("  STA $02".to_string());
        self.emit("  STX $03".to_string());
        self.emit("  PLA".to_string());
        self.emit("  LDY #0".to_string());
        self.emit("  STA ($02),Y".to_string());
        self.emit("  INY".to_string());
        self.emit("  LDA #0".to_string());
        self.emit("  STA ($02),Y".to_string());
        self.emit("  LDA $02".to_string());
        self.emit("  LDX $03".to_string());
        self.emit("  RTS".to_string());

        // Runtime_Str
        // Input: A/X (Number)
        // Output: A/X (String Ptr)
        self.emit("Runtime_Str:".to_string());
        self.emit("  STA $0A".to_string()); // Value Low
        self.emit("  STX $0B".to_string()); // Value High
        self.emit("  STY $0F".to_string()); // Caller marks a signed expression.
        self.emit("  JSR Runtime_GetHeapSlot".to_string());
        self.emit("  STA $02".to_string()); // Ptr Low
        self.emit("  STX $03".to_string()); // Ptr High
        self.emit("  LDY #0".to_string());
        // WORD is unsigned even above 32767; only INT/negated expressions
        // should render a minus sign. Do not infer signedness from value bits.
        self.emit("  LDA $0F".to_string());
        self.emit("  BEQ Str_Pos".to_string());
        self.emit("  LDA $0B".to_string());
        self.emit("  BPL Str_Pos".to_string());
        self.emit("  LDA #$2D".to_string()); // '-'
        self.emit("  STA ($02),Y".to_string());
        self.emit("  INY".to_string());
        // Negate $0A/$0B
        self.emit("  LDA $0A".to_string());
        self.emit("  EOR #$FF".to_string());
        self.emit("  CLC".to_string());
        self.emit("  ADC #1".to_string());
        self.emit("  STA $0A".to_string());
        self.emit("  LDA $0B".to_string());
        self.emit("  EOR #$FF".to_string());
        self.emit("  ADC #0".to_string());
        self.emit("  STA $0B".to_string());
        self.emit("Str_Pos:".to_string());
        // Convert
        self.emit("  LDA #0".to_string());
        self.emit("  STA $0C".to_string()); // Digit Count
        self.emit("Str_Loop:".to_string());
        // Division changes Y. Preserve the output cursor
        // beneath this iteration's digit so zero and signed values also return
        // an intact heap pointer and correctly ordered characters.
        self.emit("  TYA".to_string());
        self.emit("  PHA".to_string());
        // Divide $0A/$0B by 10
        self.emit("  LDA #10".to_string());
        self.emit("  STA $00".to_string()); // Divisor
        self.emit("  LDA #0".to_string());
        self.emit("  STA $01".to_string());
        self.emit("  LDA $0A".to_string()); // Dividend
        self.emit("  LDX $0B".to_string());
        self.emit("  JSR Math_Div16".to_string());
        // Remainder in $08/$09 (Wait, Div16 puts remainder in $08/$09? No, Div16 returns quotient in A/X. Remainder?)
        // Let's check Math_Div16:
        // "Math_Div16 returns quotient in A/X. Remainder is NOT returned?"
        // Oops. Math_Div16 in codegen does NOT expose remainder in $08.
        // Math_Div16_Signed DOES expose remainder in $08 (Wait, does it?).
        // Math_Div16 implementation:
        // Div16 returns Quotient in A/X ($06/$07).
        // Where is remainder?
        // It uses $08/$09 as temporary dividend/remainder.
        // At end, $08/$09 holds remainder!
        // Yes: "ROL $08", "ROL $09".
        // And "LDA $06", "LDX $07", "RTS".
        // So $08/$09 IS remainder.
        self.emit("  STA $0A".to_string()); // Update Value
        self.emit("  STX $0B".to_string());
        self.emit("  PLA".to_string());
        self.emit("  TAY".to_string());
        self.emit("  LDA $08".to_string()); // Remainder Low (High is 0 for 10)
        self.emit("  PHA".to_string());
        self.emit("  INC $0C".to_string());
        self.emit("  LDA $0A".to_string());
        self.emit("  ORA $0B".to_string());
        self.emit("  BNE Str_Loop".to_string());
        // Pop
        self.emit("Str_Pop:".to_string());
        self.emit("  PLA".to_string());
        self.emit("  CLC".to_string());
        self.emit("  ADC #$30".to_string());
        self.emit("  STA ($02),Y".to_string());
        self.emit("  INY".to_string());
        self.emit("  DEC $0C".to_string());
        self.emit("  BNE Str_Pop".to_string());
        self.emit("  LDA #0".to_string());
        self.emit("  STA ($02),Y".to_string());
        self.emit("  LDA $02".to_string());
        self.emit("  LDX $03".to_string());
        self.emit("  RTS".to_string());

        // Runtime_Left
        // Input: Stack (Count), A/X (Str Ptr)
        // Output: A/X (New Str Ptr)
        self.emit("Runtime_Left:".to_string());
        self.emit("  STA $04".to_string()); // Src Ptr
        self.emit("  STX $05".to_string());
        self.emit("  PLA".to_string()); // Ret Low
        self.emit("  STA $0A".to_string());
        self.emit("  PLA".to_string()); // Ret High
        self.emit("  STA $0B".to_string());
        self.emit("  PLA".to_string()); // Count (Low) - assuming < 256 for string ops
        self.emit("  STA $0C".to_string());

        // Safety: Clamp Count to 15
        self.emit("  CMP #16".to_string());
        self.emit("  BCC Left_CountOk".to_string());
        self.emit("  LDA #15".to_string());
        self.emit("  STA $0C".to_string());
        self.emit("Left_CountOk:".to_string());

        // Restore Ret
        self.emit("  LDA $0B".to_string());
        self.emit("  PHA".to_string());
        self.emit("  LDA $0A".to_string());
        self.emit("  PHA".to_string());

        self.emit("  JSR Runtime_GetHeapSlot".to_string());
        self.emit("  STA $02".to_string()); // Dest
        self.emit("  STX $03".to_string());
        self.emit("  LDY #0".to_string());
        self.emit("Left_Loop:".to_string());
        self.emit("  CPY $0C".to_string());
        self.emit("  BEQ Left_Done".to_string());
        self.emit("  LDA ($04),Y".to_string());
        self.emit("  BEQ Left_Done".to_string()); // End of src
        self.emit("  STA ($02),Y".to_string());
        self.emit("  INY".to_string());
        self.emit("  JMP Left_Loop".to_string());
        self.emit("Left_Done:".to_string());
        self.emit("  LDA #0".to_string());
        self.emit("  STA ($02),Y".to_string());
        self.emit("  LDA $02".to_string());
        self.emit("  LDX $03".to_string());
        self.emit("  RTS".to_string());

        // Runtime_Right
        // Input: Stack (Count), A/X (Str Ptr)
        self.emit("Runtime_Right:".to_string());
        self.emit("  STA $04".to_string());
        self.emit("  STX $05".to_string());
        // Pop Return Addr
        self.emit("  PLA".to_string());
        self.emit("  STA $0A".to_string());
        self.emit("  PLA".to_string());
        self.emit("  STA $0B".to_string());
        // Pop Count
        self.emit("  PLA".to_string());
        self.emit("  STA $0C".to_string());

        // Safety: Clamp Count to 15
        self.emit("  CMP #16".to_string());
        self.emit("  BCC Right_CountOk".to_string());
        self.emit("  LDA #15".to_string());
        self.emit("  STA $0C".to_string());
        self.emit("Right_CountOk:".to_string());

        // Push Ret
        self.emit("  LDA $0B".to_string());
        self.emit("  PHA".to_string());
        self.emit("  LDA $0A".to_string());
        self.emit("  PHA".to_string());

        // Calc Len
        self.emit("  LDA $04".to_string());
        self.emit("  LDX $05".to_string());
        self.emit("  JSR Runtime_StringLen".to_string());
        // Returns Len in A/X. (A=Low)
        self.emit("  SEC".to_string());
        self.emit("  SBC $0C".to_string()); // Len - Count
        self.emit("  BCC Right_Full".to_string()); // Count > Len
        self.emit("  TAY".to_string()); // Index
        self.emit("  JMP Right_Copy".to_string());
        self.emit("Right_Full:".to_string());
        self.emit("  LDY #0".to_string());
        self.emit("Right_Copy:".to_string());
        self.emit("  STY $0D".to_string()); // Start Index
        self.emit("  JSR Runtime_GetHeapSlot".to_string());
        self.emit("  STA $02".to_string());
        self.emit("  STX $03".to_string());
        self.emit("  LDY #0".to_string());

        // Add Start to Src Ptr
        self.emit("  LDA $04".to_string());
        self.emit("  CLC".to_string());
        self.emit("  ADC $0D".to_string());
        self.emit("  STA $04".to_string());
        self.emit("  LDA $05".to_string());
        self.emit("  ADC #0".to_string());
        self.emit("  STA $05".to_string());

        self.emit("Right_Loop:".to_string());
        self.emit("  LDA ($04),Y".to_string());
        self.emit("  BEQ Right_Done".to_string());
        self.emit("  STA ($02),Y".to_string());
        self.emit("  INY".to_string());
        self.emit("  JMP Right_Loop".to_string());

        self.emit("Right_Done:".to_string());
        self.emit("  LDA #0".to_string());
        self.emit("  STA ($02),Y".to_string());
        self.emit("  LDA $02".to_string());
        self.emit("  LDX $03".to_string());
        self.emit("  RTS".to_string());

        // Runtime_Mid
        // Input: Stack(Len), Stack(Start), A/X(Str)
        self.emit("Runtime_Mid:".to_string());
        self.emit("  STA $04".to_string());
        self.emit("  STX $05".to_string());
        // Pop Ret
        self.emit("  PLA".to_string());
        self.emit("  STA $0A".to_string());
        self.emit("  PLA".to_string());
        self.emit("  STA $0B".to_string());
        // Pop Start
        self.emit("  PLA".to_string());
        self.emit("  STA $0C".to_string()); // Start
                                            // Pop Len
        self.emit("  PLA".to_string());
        self.emit("  STA $0D".to_string()); // Len

        // Safety: Clamp Len to 15
        self.emit("  LDA $0D".to_string());
        self.emit("  CMP #16".to_string());
        self.emit("  BCC Mid_LenOk".to_string());
        self.emit("  LDA #15".to_string());
        self.emit("  STA $0D".to_string());
        self.emit("Mid_LenOk:".to_string());

        // Push Ret
        self.emit("  LDA $0B".to_string());
        self.emit("  PHA".to_string());
        self.emit("  LDA $0A".to_string());
        self.emit("  PHA".to_string());

        // Adjust Start (1-based to 0-based)
        self.emit("  LDA $0C".to_string());
        self.emit("  BEQ Mid_BadStart".to_string());
        self.emit("  DEC $0C".to_string());
        self.emit("  JMP Mid_StartOk".to_string());
        self.emit("Mid_BadStart:".to_string());
        // Return Empty String
        self.emit("  JSR Runtime_GetHeapSlot".to_string());
        self.emit("  STA $02".to_string());
        self.emit("  STX $03".to_string());
        self.emit("  LDY #0".to_string());
        self.emit("  LDA #0".to_string());
        self.emit("  STA ($02),Y".to_string());
        self.emit("  LDA $02".to_string());
        self.emit("  LDX $03".to_string());
        self.emit("  RTS".to_string());
        self.emit("Mid_StartOk:".to_string());

        // Add Start to Src Ptr
        self.emit("  LDA $04".to_string());
        self.emit("  CLC".to_string());
        self.emit("  ADC $0C".to_string());
        self.emit("  STA $04".to_string());
        self.emit("  LDA $05".to_string());
        self.emit("  ADC #0".to_string());
        self.emit("  STA $05".to_string());

        self.emit("  JSR Runtime_GetHeapSlot".to_string());
        self.emit("  STA $02".to_string());
        self.emit("  STX $03".to_string());
        self.emit("  LDY #0".to_string());
        self.emit("Mid_Loop:".to_string());
        self.emit("  CPY $0D".to_string());
        self.emit("  BEQ Mid_Done".to_string());
        self.emit("  LDA ($04),Y".to_string());
        self.emit("  BEQ Mid_Done".to_string());
        self.emit("  STA ($02),Y".to_string());
        self.emit("  INY".to_string());
        self.emit("  JMP Mid_Loop".to_string());
        self.emit("Mid_Done:".to_string());
        self.emit("  LDA #0".to_string());
        self.emit("  STA ($02),Y".to_string());
        self.emit("  LDA $02".to_string());
        self.emit("  LDX $03".to_string());
        self.emit("  RTS".to_string());

        // Runtime_StringConcat
        // Input: Stack (LHS Ptr), A/X (RHS Ptr)
        // Output: A/X (New Ptr)
        self.emit("Runtime_StringConcat:".to_string());
        self.emit("  STA $06".to_string()); // RHS Low
        self.emit("  STX $07".to_string()); // RHS High
                                            // Pop Return Addr
        self.emit("  PLA".to_string());
        self.emit("  STA $0A".to_string());
        self.emit("  PLA".to_string());
        self.emit("  STA $0B".to_string());
        // Pop LHS
        self.emit("  PLA".to_string());
        self.emit("  STA $04".to_string()); // LHS Low
        self.emit("  PLA".to_string());
        self.emit("  STA $05".to_string()); // LHS High

        // Push Ret
        self.emit("  LDA $0B".to_string());
        self.emit("  PHA".to_string());
        self.emit("  LDA $0A".to_string());
        self.emit("  PHA".to_string());

        self.emit("  JSR Runtime_GetHeapSlot".to_string());
        self.emit("  STA $02".to_string()); // Dest Low
        self.emit("  STX $03".to_string()); // Dest High

        self.emit("  LDY #0".to_string()); // Dest Index

        // Copy LHS
        self.emit("Concat_LoopLHS:".to_string());
        self.emit("  LDA ($04),Y".to_string()); // Read from LHS
        self.emit("  BEQ Concat_StartRHS".to_string());
        self.emit("  STA ($02),Y".to_string());
        self.emit("  INY".to_string());
        self.emit("  CPY #15".to_string()); // Max 15 chars
        self.emit("  BCS Concat_Full".to_string());
        self.emit("  JMP Concat_LoopLHS".to_string());

        self.emit("Concat_StartRHS:".to_string());
        // Need to read RHS[0] but write to Dest[Y].
        // To read ($06),0 we need Y=0. But we need Y preserved for Dest.
        self.emit("Concat_LoopRHS:".to_string());
        self.emit("  TYA".to_string());
        self.emit("  PHA".to_string()); // Save Dest Index
        self.emit("  LDY #0".to_string());
        self.emit("  LDA ($06),Y".to_string()); // Read RHS char

        // Increment RHS pointer ($06/$07)
        self.emit("  INC $06".to_string());
        self.emit("  BNE Concat_IncSkip".to_string());
        self.emit("  INC $07".to_string());
        self.emit("Concat_IncSkip:".to_string());

        self.emit("  TAX".to_string()); // Save char to X
        self.emit("  PLA".to_string());
        self.emit("  TAY".to_string()); // Restore Dest Index
        self.emit("  TXA".to_string()); // Restore char

        self.emit("  BEQ Concat_Done".to_string());
        self.emit("  STA ($02),Y".to_string());
        self.emit("  INY".to_string());
        self.emit("  CPY #15".to_string()); // Max 15 chars
        self.emit("  BCS Concat_Full".to_string());
        self.emit("  JMP Concat_LoopRHS".to_string());

        self.emit("Concat_Full:".to_string());
        self.emit("Concat_Done:".to_string());
        self.emit("  LDA #0".to_string());
        self.emit("  STA ($02),Y".to_string());
        self.emit("  LDA $02".to_string());
        self.emit("  LDX $03".to_string());
        self.emit("  RTS".to_string());

        // Runtime_StringCompare
        // Input: Stack (LHS Ptr), A/X (RHS Ptr)
        // Output: A ($FF=Equal, $00=NotEqual), X=0
        self.emit("Runtime_StringCompare:".to_string());
        self.emit("  STA $06".to_string()); // RHS Low
        self.emit("  STX $07".to_string()); // RHS High
                                            // Pop Return Addr
        self.emit("  PLA".to_string());
        self.emit("  STA $0A".to_string());
        self.emit("  PLA".to_string());
        self.emit("  STA $0B".to_string());
        // Pop LHS
        self.emit("  PLA".to_string());
        self.emit("  STA $04".to_string()); // LHS Low
        self.emit("  PLA".to_string());
        self.emit("  STA $05".to_string()); // LHS High

        // Push Ret
        self.emit("  LDA $0B".to_string());
        self.emit("  PHA".to_string());
        self.emit("  LDA $0A".to_string());
        self.emit("  PHA".to_string());

        self.emit("  LDY #0".to_string());
        self.emit("StrCmp_Loop:".to_string());
        self.emit("  LDA ($04),Y".to_string());
        self.emit("  CMP ($06),Y".to_string());
        self.emit("  BNE StrCmp_NotEq".to_string());
        self.emit("  CMP #0".to_string());
        self.emit("  BEQ StrCmp_Eq".to_string());
        self.emit("  INY".to_string());
        self.emit("  JMP StrCmp_Loop".to_string());

        self.emit("StrCmp_NotEq:".to_string());
        self.emit("  LDA #0".to_string());
        self.emit("  LDX #0".to_string());
        self.emit("  RTS".to_string());

        self.emit("StrCmp_Eq:".to_string());
        self.emit("  LDA #$FF".to_string());
        self.emit("  LDX #0".to_string());
        self.emit("  RTS".to_string());

        self.emit("".to_string());
    }

    fn generate_string_data(&mut self) {
        self.emit("".to_string());
        self.emit("; --- String Data ---".to_string());
        let literals: Vec<(String, String)> = self
            .string_literals
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        for (content, label) in literals {
            self.emit(format!("{}:", label));
            let bytes: Vec<String> = content.bytes().map(|b| format!("${:02X}", b)).collect();
            self.emit(format!("  db {}, $00", bytes.join(", ")));
        }
        self.emit("".to_string());
    }

    fn generate_user_data(&mut self, program: &Program) -> Result<(), String> {
        let mut meta_groups: HashMap<Vec<u8>, Vec<String>> = HashMap::new();
        let mut meta_payloads = HashMap::new();
        let mut canonical_meta = HashMap::new();
        self.asset_reuse = Default::default();
        for decl in &program.declarations {
            if let TopLevelKind::Metasprite(name, tiles) = &decl.kind {
                let payload = self.metasprite_payload(tiles)?;
                self.asset_reuse.metasprites += 1;
                self.asset_reuse.metadata_before_dedup += payload.len();
                let group = meta_groups.entry(payload.clone()).or_default();
                if group.is_empty() {
                    self.asset_reuse.metadata_after_dedup += payload.len();
                }
                group.push(name.clone());
                canonical_meta.insert(name.clone(), group[0].clone());
                meta_payloads.insert(name.clone(), payload);
            }
        }
        let mut animation_groups: HashMap<AnimationDataKey, Vec<String>> = HashMap::new();
        let mut animation_keys = HashMap::new();
        for decl in &program.declarations {
            if let TopLevelKind::Animation(name, frames, loops) = &decl.kind {
                let key = (
                    *loops,
                    frames
                        .iter()
                        .map(|frame| {
                            Ok((
                                canonical_meta
                                    .get(&frame.metasprite)
                                    .ok_or_else(|| {
                                        format!("Missing metasprite '{}'", frame.metasprite)
                                    })?
                                    .clone(),
                                frame.duration,
                            ))
                        })
                        .collect::<Result<Vec<_>, String>>()?,
                );
                self.asset_reuse.animations += 1;
                self.asset_reuse.metadata_before_dedup += 2 + frames.len() * 3;
                let group = animation_groups.entry(key.clone()).or_default();
                if group.is_empty() {
                    self.asset_reuse.metadata_after_dedup += 2 + frames.len() * 3;
                }
                group.push(name.clone());
                animation_keys.insert(name.clone(), key);
            }
        }
        self.asset_reuse.unique_metasprites = meta_groups.len();
        self.asset_reuse.unique_animations = animation_groups.len();
        self.asset_reuse.metadata_bytes_saved =
            self.asset_reuse.metadata_before_dedup - self.asset_reuse.metadata_after_dedup;
        let mut emitted = std::collections::HashSet::new();
        self.emit("".to_string());
        self.emit("; --- User Data ---".to_string());
        self.emit("USER_DATA_START:".to_string());
        for decl in &program.declarations {
            if let TopLevelKind::Data(label, exprs) = &decl.kind {
                if let Some(l) = label {
                    self.emit(format!("{}:", l));
                }
                for expr in exprs {
                    match expr {
                        Expression::Integer(val) => {
                            if (-128..=255).contains(val) {
                                self.output
                                    .push(format!("  db ${:02X}", (val & 0xFF) as u8));
                            } else {
                                let low = (val & 0xFF) as u8;
                                let high = ((val >> 8) & 0xFF) as u8;
                                self.output
                                    .push(format!("  db ${:02X}, ${:02X}", low, high));
                            }
                        }
                        Expression::StringLiteral(s) => {
                            let bytes: Vec<String> =
                                s.bytes().map(|b| format!("${:02X}", b)).collect();
                            self.emit(format!("  db {}, $00", bytes.join(", ")));
                        }
                        Expression::UnaryOp(UnaryOperator::Negate, operand) => {
                            if let Expression::Integer(val) = &**operand {
                                let neg_val = -val;
                                if (-128..=255).contains(&neg_val) {
                                    self.output
                                        .push(format!("  db ${:02X}", (neg_val & 0xFF) as u8));
                                } else {
                                    let low = (neg_val & 0xFF) as u8;
                                    let high = ((neg_val >> 8) & 0xFF) as u8;
                                    self.output
                                        .push(format!("  db ${:02X}, ${:02X}", low, high));
                                }
                            } else {
                                return Err("DATA error".to_string());
                            }
                        }
                        _ => return Err("DATA statement only supports literals".to_string()),
                    }
                }
            } else if let TopLevelKind::Metasprite(name, tiles) = &decl.kind {
                if tiles.len() > 255 {
                    return Err(format!("Metasprite '{name}' supports at most 255 tiles"));
                }
                if emitted.contains(name) {
                    continue;
                }
                let payload = &meta_payloads[name];
                for alias in &meta_groups[payload] {
                    self.emit(format!("{alias}:"));
                    emitted.insert(alias.clone());
                }
                self.emit(format!("  db ${:02X}", payload[0]));
                for record in payload[1..payload.len() - 2].chunks(4) {
                    self.emit(format!(
                        "  db {}",
                        record
                            .iter()
                            .map(|byte| format!("${byte:02X}"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
                self.emit(format!(
                    "  db ${:02X}, ${:02X}",
                    payload[payload.len() - 2],
                    payload[payload.len() - 1]
                ));
            } else if let TopLevelKind::Animation(name, frames, loops) = &decl.kind {
                // Retain validation for direct codegen callers that bypass semantic analysis.
                if frames.is_empty()
                    || frames.len() > 255
                    || frames.iter().any(|frame| frame.duration == 0)
                {
                    return Err(format!(
                        "Animation '{name}' requires 1..255 frames with durations 1..255"
                    ));
                }
                if emitted.contains(name) {
                    continue;
                }
                let key = &animation_keys[name];
                let aliases = &animation_groups[key];
                for alias in aliases {
                    self.emit(format!("{alias}:"));
                    emitted.insert(alias.clone());
                }
                self.output
                    .push(format!("  db ${:02X}", frames.len() as u8));
                self.output
                    .push(format!("  db ${:02X}", if *loops { 1 } else { 0 }));
                for (i, frame) in frames.iter().enumerate() {
                    for alias in aliases {
                        self.emit(format!("{alias}_f{i}:"));
                    }
                    self.emit(format!("  WORD {}", canonical_meta[&frame.metasprite]));
                    self.emit(format!("  db ${:02X}", frame.duration));
                }
            } else if let TopLevelKind::Metatile(name, tiles, attr) = &decl.kind {
                self.emit(format!("{}:", name));
                self.emit(format!(
                    "  db ${:02X}, ${:02X}, ${:02X}, ${:02X}, ${:02X}",
                    tiles[0], tiles[1], tiles[2], tiles[3], attr
                ));
            } else if let TopLevelKind::World(width, height, data) = &decl.kind {
                self.emit("World_Map:".to_string());
                self.emit(format!(
                    "  db ${:02X}, ${:02X}",
                    width & 0xFF,
                    (width >> 8) & 0xFF
                ));
                self.emit(format!(
                    "  db ${:02X}, ${:02X}",
                    height & 0xFF,
                    (height >> 8) & 0xFF
                ));
                let mut line = String::from("  db ");
                for (i, val) in data.iter().enumerate() {
                    if i > 0 && i % 16 == 0 {
                        self.emit(line);
                        line = String::from("  db ");
                    } else if i > 0 {
                        line.push_str(", ");
                    }
                    line.push_str(&format!("${:02X}", *val as u8));
                }
                self.emit(line);
            }
        }
        self.emit("".to_string());
        Ok(())
    }

    fn metasprite_payload(
        &self,
        tiles: &[crate::compiler::ast::MetaspriteTile],
    ) -> Result<Vec<u8>, String> {
        if tiles.len() > 255 {
            return Err("Metasprite supports at most 255 tiles".into());
        }
        let mut payload = vec![tiles.len() as u8];
        let mut xs = Vec::new();
        let mut ys = Vec::new();
        for tile in tiles {
            let x = self.resolve_constant(&tile.x)? as i8;
            let y = self.resolve_constant(&tile.y)? as i8;
            xs.push(i16::from(x));
            ys.push(i16::from(y));
            payload.extend([
                x as u8,
                y as u8,
                self.resolve_constant(&tile.tile)? as u8,
                self.resolve_constant(&tile.attr)? as u8,
            ]);
        }
        // Footer preserves the legacy count/tile layout and supplies signed-frame mirror pivots.
        payload.push((xs.iter().min().unwrap_or(&0) + xs.iter().max().unwrap_or(&0)) as u8);
        payload.push((ys.iter().min().unwrap_or(&0) + ys.iter().max().unwrap_or(&0)) as u8);
        Ok(payload)
    }

    fn resolve_constant(&self, expr: &Expression) -> Result<i32, String> {
        match expr {
            Expression::Integer(val) => Ok(*val),
            Expression::UnaryOp(UnaryOperator::Negate, operand) => {
                let val = self.resolve_constant(operand)?;
                Ok(-val)
            }
            Expression::Identifier(name) => {
                if let Some(sym) = self.symbol_table.resolve(name) {
                    if let Some(val) = sym.value {
                        Ok(val)
                    } else {
                        Err(format!("Symbol '{}' is not a constant value", name))
                    }
                } else {
                    Err(format!("Undefined symbol '{}'", name))
                }
            }
            Expression::BinaryOp(l, op, r) => {
                let lv = self.resolve_constant(l)?;
                let rv = self.resolve_constant(r)?;
                match op {
                    BinaryOperator::Add => Ok(lv + rv),
                    BinaryOperator::Subtract => Ok(lv - rv),
                    BinaryOperator::Multiply => Ok(lv * rv),
                    BinaryOperator::Divide => Ok(lv / rv),
                    _ => Err("Unsupported constant operator".to_string()),
                }
            }
            _ => Err("Not a constant expression".to_string()),
        }
    }

    fn generate_data_tables(&mut self, program: &Program) -> Result<(), String> {
        self.emit("".to_string());
        self.emit("; --- Data Tables ---".to_string());
        self.emit(".ORG $FF00".to_string());
        let mut current_addr = 0xFF00;
        self.output
            .push("InitDefaultRTI: WORD DefaultRTI".to_string());
        current_addr += 2;
        self.output
            .push("InitUserData: WORD USER_DATA_START".to_string());
        current_addr += 2;

        for decl in &program.declarations {
            if let TopLevelKind::Interrupt(name, _) = &decl.kind {
                let upper = name.to_uppercase();
                if upper == "NMI" {
                    self.emit(format!("InitNMI_{}: WORD {}", name, name));
                    current_addr += 2;
                } else if upper == "IRQ" {
                    self.emit(format!("InitIRQ_{}: WORD {}", name, name));
                    current_addr += 2;
                }
                self.emit(format!("Ptr_{}: WORD {}", name, name));
                self.data_table_offsets.insert(name.clone(), current_addr);
                current_addr += 2;
            }
            if let TopLevelKind::Sub(name, _, _) = &decl.kind {
                self.emit(format!("Ptr_{}: WORD {}", name, name));
                self.data_table_offsets.insert(name.clone(), current_addr);
                current_addr += 2;
            }
            if let TopLevelKind::Data(Some(label), _) = &decl.kind {
                self.emit(format!("Ptr_{}: WORD {}", label, label));
                self.data_table_offsets
                    .insert(label.to_string(), current_addr);
                current_addr += 2;
            }
            if let TopLevelKind::Metatile(name, _, _)
            | TopLevelKind::Metasprite(name, _)
            | TopLevelKind::Animation(name, _, _) = &decl.kind
            {
                self.emit(format!("Ptr_{}: WORD {}", name, name));
                self.data_table_offsets.insert(name.clone(), current_addr);
                current_addr += 2;
            }
            if let TopLevelKind::World(_, _, _) = &decl.kind {
                let label = "World_Map";
                self.emit(format!("Ptr_{}: WORD {}", label, label));
                self.data_table_offsets
                    .insert(label.to_string(), current_addr);
                current_addr += 2;
            }
        }

        let mut sorted_strings: Vec<(String, String)> = self
            .string_literals
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        sorted_strings.sort_by_key(|(k, _)| k.clone());
        for (_, label) in sorted_strings {
            self.emit(format!("Ptr_{}: WORD {}", label, label));
        }
        self.emit("".to_string());
        Ok(())
    }

    fn generate_vectors(&mut self, _program: &Program) -> Result<(), String> {
        self.emit("".to_string());
        self.emit(".ORG $FFFA".to_string());
        self.emit("VecNMI: WORD TrampolineNMI".to_string());
        self.emit("VecReset: WORD Startup".to_string());
        self.emit("VecIRQ: WORD TrampolineIRQ".to_string());
        self.emit("".to_string());
        Ok(())
    }

    fn generate_controller_helpers(&mut self) {
        self.emit("; --- Controller Helpers ---".to_string());
        self.emit("Runtime_Controller_Read:".to_string());
        self.emit("  LDA #$01".to_string());
        self.emit("  STA $4016".to_string());
        self.emit("  LDA #$00".to_string());
        self.emit("  STA $4016".to_string());
        self.emit("  LDA $10".to_string()); // Load old held
        self.emit("  STA $11".to_string()); // Save to last
        self.emit("  LDA #0".to_string());
        self.emit("  STA $10".to_string()); // Clear held
        self.emit("  LDX #8".to_string());
        self.emit("Controller_ReadLoop:".to_string());
        self.emit("  LDA $4016".to_string());
        self.emit("  LSR".to_string()); // Bit0 -> Carry
        self.emit("  ROL $10".to_string()); // Carry -> Bit0 of Held
        self.emit("  DEX".to_string());
        self.emit("  BNE Controller_ReadLoop".to_string());
        // Pressed = New & !Last
        self.emit("  LDA $11".to_string());
        self.emit("  EOR #$FF".to_string());
        self.emit("  AND $10".to_string());
        self.emit("  STA $12".to_string());
        // Released = !New & Last
        self.emit("  LDA $10".to_string());
        self.emit("  EOR #$FF".to_string());
        self.emit("  AND $11".to_string());
        self.emit("  STA $13".to_string());
        self.emit("  RTS".to_string());
        self.emit("".to_string());
    }

    fn generate_sprite_helpers(&mut self) {
        self.emit("; --- Sprite Helpers ---".to_string());

        // Runtime_SpriteClear
        // Resets $19 (OAM Ptr) and fills OAM with $FF (Offscreen)
        // Implements OAM Cycling (Flicker) if $1C is set.
        // Uses $1A (OAM Base Offset), $1B (Sprites Drawn Count), $1C (Flicker Enable)
        self.emit("Runtime_SpriteClear:".to_string());
        self.emit("  LDA $1C".to_string());
        self.emit("  BEQ SpriteClear_NoShuffle".to_string());
        self.emit("  LDA $1A".to_string());
        self.emit("  CLC".to_string());
        self.emit("  ADC #44".to_string()); // 11 slots
        self.emit("  STA $1A".to_string());
        self.emit("  JMP SpriteClear_SetPtr".to_string());
        self.emit("SpriteClear_NoShuffle:".to_string());
        self.emit("  LDA #0".to_string());
        self.emit("  STA $1A".to_string());
        self.emit("SpriteClear_SetPtr:".to_string());
        self.emit("  LDA $1A".to_string());
        self.emit("  STA $19".to_string()); // Set Ptr to Base
        self.emit("  LDA #0".to_string());
        self.emit("  STA $1B".to_string()); // Reset Count

        self.emit("  LDA #$FF".to_string());
        self.emit("  LDX #0".to_string());
        self.emit("SpriteClear_Loop:".to_string());
        self.emit("  STA $0200, X".to_string());
        self.emit("  INX".to_string());
        self.emit("  BNE SpriteClear_Loop".to_string());
        self.emit("  RTS".to_string());

        // Runtime_SpriteDraw
        // Args: X ($14), Y ($15), MetaPtr ($16/$17)
        // Uses: $02/$03 (Ptr), $04 (Count), $05 (Temp X), $06 (Temp Y), $19 (OAM Offset), $1B (Total Count)
        self.emit("Runtime_SpriteDraw:".to_string());
        self.emit("  LDA $16".to_string());
        self.emit("  STA $02".to_string());
        self.emit("  LDA $17".to_string());
        self.emit("  STA $03".to_string());

        // Read Count
        self.emit("  LDY #0".to_string());
        self.emit("  LDA ($02), Y".to_string());
        self.emit("  STA $04".to_string()); // Count
                                            // Advance the pointer past the count; each tile is addressed at offsets 0..3.
                                            // A 64-tile metasprite is 257 bytes, so a single 8-bit Y cursor would wrap its final attribute.
        self.emit("  INC $02".to_string());
        self.emit("  BNE SpriteDraw_DataStart".to_string());
        self.emit("  INC $03".to_string());
        self.emit("SpriteDraw_DataStart:".to_string());

        self.emit("  LDX $19".to_string()); // Load current OAM ptr

        self.emit("SpriteDraw_Loop:".to_string());
        self.emit("  LDA $04".to_string());
        self.emit("  BEQ SpriteDraw_Done".to_string());
        self.emit("  DEC $04".to_string());

        // Check Limit (64 sprites)
        self.emit("  LDA $1B".to_string());
        self.emit("  CMP #64".to_string());
        self.emit("  BCS SpriteDraw_Full".to_string());
        self.emit("  LDY #0".to_string());

        // Read Rel X
        self.emit("  LDA ($02), Y".to_string());
        self.emit("  CLC".to_string());
        self.emit("  ADC $14".to_string()); // + Screen X
        self.emit("  STA $05".to_string()); // Store Screen X
        self.emit("  INY".to_string());

        // Read Rel Y
        self.emit("  LDA ($02), Y".to_string());
        self.emit("  CLC".to_string());
        self.emit("  ADC $15".to_string()); // + Screen Y
        self.emit("  STA $06".to_string()); // Store Screen Y
        self.emit("  INY".to_string());

        // Read Tile
        self.emit("  LDA ($02), Y".to_string());
        self.emit("  PHA".to_string()); // Push Tile
        self.emit("  INY".to_string());

        // Read Attr
        self.emit("  LDA ($02), Y".to_string());
        self.emit("  PHA".to_string()); // Push Attr
        self.emit("  INY".to_string());

        // Write to OAM
        // X points to current slot (starting with Y coord at Base)
        self.emit("  LDA $06".to_string()); // Y
        self.emit("  STA $0200, X".to_string());
        self.emit("  INX".to_string());

        self.emit("  PLA".to_string()); // Attr (Popped first)
        self.emit("  STA $0201, X".to_string()); // Writes to Base+2 (Since X is Base+1)

        self.emit("  PLA".to_string()); // Tile
        self.emit("  STA $0200, X".to_string()); // Writes to Base+1 (Since X is Base+1)

        self.emit("  INX".to_string()); // X -> Base+2
        self.emit("  INX".to_string()); // X -> Base+3

        self.emit("  LDA $05".to_string()); // Screen X
        self.emit("  STA $0200, X".to_string()); // Writes to Base+3

        self.emit("  INX".to_string()); // X -> Base+4 (Next Sprite Base)
        self.emit("  STX $19".to_string()); // Save OAM Ptr

        self.emit("  INC $1B".to_string()); // Increment Global Sprite Count

        self.emit("  LDA $02".to_string());
        self.emit("  CLC".to_string());
        self.emit("  ADC #4".to_string());
        self.emit("  STA $02".to_string());
        self.emit("  BCC SpriteDraw_Next".to_string());
        self.emit("  INC $03".to_string());
        self.emit("SpriteDraw_Next:".to_string());

        self.emit("  JMP SpriteDraw_Loop".to_string());

        self.emit("SpriteDraw_Full:".to_string());
        // Once OAM is full there is no remaining output to produce.

        self.emit("SpriteDraw_Done:".to_string());
        self.emit("  RTS".to_string());
        self.emit("".to_string());
        self.generate_sprite_variant_helpers();
    }

    fn generate_sprite_variant_helpers(&mut self) {
        // Only styled callers pay for palette/whole-frame mirroring; legacy Draw stays fast.
        let setup="Runtime_SpriteDrawVariant:\n  LDA $09\n  AND #$C0\n  STA $09\n  LDA $08\n  CMP #$FF\n  BEQ SpriteVariant_PaletteReady\n  AND #3\n  STA $08\nSpriteVariant_PaletteReady:\n  LDA $16\n  STA $02\n  LDA $17\n  STA $03\n  LDY #0\n  LDA ($02), Y\n  STA $04\n  LDA $09\n  BEQ SpriteVariant_Data\n  LDA #0\n  STA $0B\n  LDA $04\n  ASL\n  ROL $0B\n  ASL\n  ROL $0B\n  CLC\n  ADC #1\n  STA $0A\n  LDA $0B\n  ADC #0\n  STA $0B\n  LDA $02\n  CLC\n  ADC $0A\n  STA $00\n  LDA $03\n  ADC $0B\n  STA $01\n  LDY #0\n  LDA ($00), Y\n  STA $0A\n  INY\n  LDA ($00), Y\n  STA $0B\nSpriteVariant_Data:\n  INC $02\n  BNE SpriteVariant_DataReady\n  INC $03\nSpriteVariant_DataReady:\n  LDX $19\nSpriteVariant_Loop:\n  LDA $04\n  BEQ SpriteVariant_Done\n  DEC $04\n  LDA $1B\n  CMP #64\n  BCS SpriteVariant_Done\n  LDY #0\n  LDA $09\n  AND #$40\n  BEQ SpriteVariant_NormalX\n  LDA $0A\n  SEC\n  SBC ($02), Y\n  JMP SpriteVariant_AddX\nSpriteVariant_NormalX:\n  LDA ($02), Y\nSpriteVariant_AddX:\n  CLC\n  ADC $14\n  STA $05\n  INY\n  LDA $09\n  AND #$80\n  BEQ SpriteVariant_NormalY\n  LDA $0B\n  SEC\n  SBC ($02), Y\n  JMP SpriteVariant_AddY\nSpriteVariant_NormalY:\n  LDA ($02), Y\nSpriteVariant_AddY:\n  CLC\n  ADC $15\n  STA $06\n  INY\n  LDA ($02), Y\n  PHA\n  INY\n  LDA ($02), Y\n  EOR $09\n  STA $0C\n  LDA $08\n  CMP #$FF\n  BEQ SpriteVariant_KeepPalette\n  LDA $0C\n  AND #$FC\n  ORA $08\n  JMP SpriteVariant_Attribute\nSpriteVariant_KeepPalette:\n  LDA $0C\nSpriteVariant_Attribute:\n  PHA\n  LDA $06\n  STA $0200, X\n  INX\n  PLA\n  STA $0201, X\n  PLA\n  STA $0200, X\n  INX\n  INX\n  LDA $05\n  STA $0200, X\n  INX\n  STX $19\n  INC $1B\n  LDA $02\n  CLC\n  ADC #4\n  STA $02\n  BCC SpriteVariant_Next\n  INC $03\nSpriteVariant_Next:\n  JMP SpriteVariant_Loop\nSpriteVariant_Done:\n  RTS";
        for line in setup.lines() {
            self.emit(line.into());
        }
    }

    fn generate_variant_draw(
        &mut self,
        args: &[Expression],
        animation: bool,
    ) -> Result<(), String> {
        for arg in &args[..2] {
            self.generate_expression(arg)?;
            self.emit("  PHA".into());
        }
        if animation {
            let address = self.get_static_address(&args[2])?;
            self.emit(format!("  LDA #${:02X}", address as u8));
            self.emit("  PHA".into());
            self.emit(format!("  LDA #${:02X}", (address >> 8) as u8));
            self.emit("  PHA".into());
        } else {
            self.generate_expression(&args[2])?;
            self.emit("  PHA".into());
            self.emit("  TXA".into());
            self.emit("  PHA".into());
        }
        for arg in &args[3..] {
            self.generate_expression(arg)?;
            self.emit("  PHA".into());
        }
        // Stage arguments on the CPU stack so later expressions cannot clobber earlier ones.
        for address in [0x09, 0x08, 0x17, 0x16, 0x15, 0x14] {
            self.emit("  PLA".into());
            self.emit(format!("  STA ${address:02X}"));
        }
        self.emit(format!(
            "  JSR {}",
            if animation {
                "Runtime_Anim_DrawVariant"
            } else {
                "Runtime_SpriteDrawVariant"
            }
        ));
        Ok(())
    }

    fn generate_text_helpers(&mut self) {
        self.emit("; --- Text Helpers ---".to_string());
        // Runtime_Text_Print
        // Inputs: $14 (X), $15 (Y), $16/$17 (Str Ptr), $18 (Offset)
        // Writes to PPU $2006/$2007
        // Addr = $2000 + Y*32 + X
        // Y*32 = (Y << 5)
        self.emit("Runtime_Text_Print:".to_string());
        // Clip to one visible tile row so strings cannot enter attribute memory.
        // Printing remains a direct PPU operation; callers must wait for VBlank or disable rendering.
        self.emit("  LDA $14".to_string());
        self.emit("  CMP #32".to_string());
        self.emit("  BCS Text_Return".to_string());
        self.emit("  LDA $15".to_string());
        self.emit("  CMP #30".to_string());
        self.emit("  BCS Text_Return".to_string());
        self.emit("  LDA #32".to_string());
        self.emit("  SEC".to_string());
        self.emit("  SBC $14".to_string());
        self.emit("  STA $04".to_string());
        // A caller may have enabled vertical PPUDATA increment; text always advances horizontally.
        self.emit("  LDA $F8".to_string());
        self.emit("  AND #$FB".to_string());
        self.emit("  STA $2000".to_string());
        self.emit("  LDA $2002".to_string()); // Reset latch

        // Calc Addr
        self.emit("  LDA #0".to_string());
        self.emit("  STA $01".to_string()); // High Byte of offset
        self.emit("  LDA $15".to_string()); // Y
        self.emit("  ASL".to_string());
        self.emit("  ROL $01".to_string());
        self.emit("  ASL".to_string());
        self.emit("  ROL $01".to_string());
        self.emit("  ASL".to_string());
        self.emit("  ROL $01".to_string());
        self.emit("  ASL".to_string());
        self.emit("  ROL $01".to_string());
        self.emit("  ASL".to_string());
        self.emit("  ROL $01".to_string());
        // A now has Low Byte of Y*32, $01 has High Byte

        self.emit("  CLC".to_string());
        self.emit("  ADC $14".to_string()); // + X
        self.emit("  STA $00".to_string()); // Low Addr
        self.emit("  LDA $01".to_string());
        self.emit("  ADC #$20".to_string()); // + Base $2000 (High Byte $20)
        self.emit("  STA $01".to_string()); // High Addr

        // Set PPU Addr
        self.emit("  LDA $01".to_string());
        self.emit("  STA $2006".to_string());
        self.emit("  LDA $00".to_string());
        self.emit("  STA $2006".to_string());

        // Loop Chars
        self.emit("  LDY #0".to_string());
        self.emit("Text_Loop:".to_string());
        self.emit("  LDA ($16),Y".to_string());
        self.emit("  BEQ Text_Done".to_string());
        self.emit("  CLC".to_string());
        self.emit("  ADC $18".to_string()); // Add Offset
        self.emit("  STA $2007".to_string());
        self.emit("  DEC $04".to_string());
        self.emit("  BEQ Text_Done".to_string());
        self.emit("  INY".to_string());
        self.emit("  BNE Text_Loop".to_string());
        self.emit("Text_Done:".to_string());
        self.emit("  LDA $F8".to_string());
        self.emit("  STA $2000".to_string());
        self.emit("Text_Return:".to_string());
        self.emit("  RTS".to_string());
        self.emit("".to_string());
    }

    fn collect_all_strings(&mut self, program: &Program) {
        for decl in &program.declarations {
            match &decl.kind {
                TopLevelKind::Const(_, expr) => self.collect_strings_expr(expr),
                TopLevelKind::Dim(_, _, Some(expr)) => self.collect_strings_expr(expr),
                TopLevelKind::Sub(_, _, body) => self.collect_strings_block(body),
                TopLevelKind::Interrupt(_, body) => self.collect_strings_block(body),
                TopLevelKind::Data(_, exprs) => {
                    for e in exprs {
                        self.collect_strings_expr(e);
                    }
                }
                _ => {}
            }
        }
    }

    fn collect_strings_block(&mut self, statements: &[Statement]) {
        for stmt in statements {
            self.collect_strings_stmt(stmt);
        }
    }

    fn collect_strings_stmt(&mut self, stmt: &Statement) {
        self.current_line = stmt.line;
        match &stmt.kind {
            StatementKind::Let(target, expr) => {
                self.collect_strings_expr(target);
                self.collect_strings_expr(expr);
            }
            StatementKind::If(cond, then_b, else_b) => {
                self.collect_strings_expr(cond);
                self.collect_strings_block(then_b);
                if let Some(b) = else_b {
                    self.collect_strings_block(b);
                }
            }
            StatementKind::While(cond, body) => {
                self.collect_strings_expr(cond);
                self.collect_strings_block(body);
            }
            StatementKind::DoWhile(body, cond) => {
                self.collect_strings_block(body);
                self.collect_strings_expr(cond);
            }
            StatementKind::For(_, start, end, step, body) => {
                self.collect_strings_expr(start);
                self.collect_strings_expr(end);
                if let Some(s) = step {
                    self.collect_strings_expr(s);
                }
                self.collect_strings_block(body);
            }
            StatementKind::Return(Some(expr)) => self.collect_strings_expr(expr),
            StatementKind::Call(target, args) => {
                self.collect_strings_expr(target);
                for arg in args {
                    self.collect_strings_expr(arg);
                }
            }
            StatementKind::Poke(addr, val) => {
                self.collect_strings_expr(addr);
                self.collect_strings_expr(val);
            }
            StatementKind::PlaySfx(expr) => self.collect_strings_expr(expr),
            StatementKind::Select(expr, cases, else_b) => {
                self.collect_strings_expr(expr);
                for (val, block) in cases {
                    self.collect_strings_expr(val);
                    self.collect_strings_block(block);
                }
                if let Some(b) = else_b {
                    self.collect_strings_block(b);
                }
            }
            _ => {}
        }
    }

    fn collect_strings_expr(&mut self, expr: &Expression) {
        match expr {
            Expression::StringLiteral(s) => {
                if !self.string_literals.contains_key(s) {
                    let label = self.new_label();
                    self.string_literals.insert(s.clone(), label);
                }
            }
            Expression::BinaryOp(l, _, r) => {
                self.collect_strings_expr(l);
                self.collect_strings_expr(r);
            }
            Expression::UnaryOp(_, e) => self.collect_strings_expr(e),
            Expression::Call(target, args) => {
                self.collect_strings_expr(target);
                for arg in args {
                    self.collect_strings_expr(arg);
                }
            }
            Expression::Peek(e) => self.collect_strings_expr(e),
            Expression::MemberAccess(e, _) => self.collect_strings_expr(e),
            _ => {}
        }
    }

    fn allocate_memory(&mut self, program: &Program) -> Result<(), String> {
        self.collect_all_strings(program);
        self.current_line = 0; // Reset line number to avoid polluting source map with pre-pass lines
        self.ram_pointer = VAR_START_RAM;
        let mut data_table_addr = 0xFF00;
        data_table_addr += 2; // InitDefaultRTI
        self.data_table_offsets
            .insert("USER_DATA_START".to_string(), data_table_addr);
        data_table_addr += 2;

        for decl in &program.declarations {
            match &decl.kind {
                TopLevelKind::Dim(name, dtype, _) => {
                    self.symbol_table.assign_address(name, self.ram_pointer)?;
                    self.output
                        .push(format!("; {} @ ${:04X}", name, self.ram_pointer));
                    let size = self.get_type_size(dtype)?;
                    self.ram_pointer = self
                        .ram_pointer
                        .checked_add(size)
                        .ok_or("RAM allocation overflow")?;
                    if self.ram_pointer > 0x07F0 {
                        return Err(format!(
                            "RAM overflow: Variable '{}' allocation exceeded safe memory limit ($07EF)",
                            name
                        ));
                    }
                }
                TopLevelKind::Sub(sub_name, params, _) => {
                    self.symbol_table.enter_scope();
                    let mut sig_params = Vec::new();
                    for (param_name, param_type) in params {
                        self.symbol_table.define(
                            param_name.clone(),
                            param_type.clone(),
                            SymbolKind::Param,
                        )?;
                        self.symbol_table
                            .assign_address(param_name, self.ram_pointer)?;
                        self.emit(format!(
                            "; {}.{} @ ${:04X}",
                            sub_name, param_name, self.ram_pointer
                        ));
                        sig_params.push((self.ram_pointer, param_type.clone()));
                        self.ram_pointer = self
                            .ram_pointer
                            .checked_add(self.get_type_size(param_type)?)
                            .ok_or("RAM allocation overflow")?;
                        if self.ram_pointer > 0x07F0 {
                            return Err(format!(
                                "RAM overflow: Parameter '{}' in sub '{}' exceeded safe memory limit ($07EF)",
                                param_name, sub_name
                            ));
                        }
                    }
                    self.sub_signatures.insert(
                        sub_name.clone(),
                        (
                            self.symbol_table
                                .resolve(sub_name)
                                .and_then(|symbol| symbol.bank)
                                .unwrap_or(0),
                            sig_params,
                        ),
                    );
                    self.data_table_offsets
                        .insert(sub_name.clone(), data_table_addr);
                    data_table_addr += 2;
                    self.symbol_table.exit_scope();
                }
                TopLevelKind::Interrupt(name, _) => {
                    if name.to_uppercase() == "NMI" || name.to_uppercase() == "IRQ" {
                        data_table_addr += 2;
                    }
                    self.data_table_offsets
                        .insert(name.clone(), data_table_addr);
                    data_table_addr += 2;
                }
                TopLevelKind::Data(Some(label), _) => {
                    self.data_table_offsets
                        .insert(label.clone(), data_table_addr);
                    data_table_addr += 2;
                }
                TopLevelKind::Metasprite(name, _) => {
                    self.data_table_offsets
                        .insert(name.clone(), data_table_addr);
                    data_table_addr += 2; // Pointer, not the asset payload.
                }
                TopLevelKind::Animation(name, _, _) => {
                    self.data_table_offsets
                        .insert(name.clone(), data_table_addr);
                    data_table_addr += 2;
                }
                TopLevelKind::Metatile(name, _, _) => {
                    self.data_table_offsets
                        .insert(name.clone(), data_table_addr);
                    data_table_addr += 2;
                }
                TopLevelKind::World(_, _, _) => {
                    self.data_table_offsets
                        .insert("World_Map".to_string(), data_table_addr);
                    data_table_addr += 2;
                }
                _ => {}
            }
        }
        let mut sorted_strings: Vec<_> = self.string_literals.iter().collect();
        sorted_strings.sort_by_key(|(k, _)| *k);
        for (_, label) in sorted_strings {
            if !self.data_table_offsets.contains_key(label) {
                self.data_table_offsets
                    .insert(label.clone(), data_table_addr);
                data_table_addr += 2;
            }
        }
        Ok(())
    }

    fn generate_top_level(&mut self, decl: &TopLevel) -> Result<(), String> {
        self.output.push(format!(
            ";@source {}",
            serde_json::json!({"file": decl.source_file, "line": decl.line})
        ));
        self.select_stack_depth = 0;
        match &decl.kind {
            TopLevelKind::Sub(name, _, body) => {
                self.emit(format!("{}:", name));
                self.symbol_table.enter_scope();
                self.generate_block(body)?;
                self.symbol_table.exit_scope();
                self.emit("  RTS".to_string());
                self.emit("".to_string());
            }
            TopLevelKind::Bank(_) => {}
            TopLevelKind::Interrupt(name, body) => {
                self.emit(format!("{}:", name));
                self.symbol_table.enter_scope();
                self.generate_block(body)?;
                self.symbol_table.exit_scope();
                self.emit("  RTS".to_string());
                self.emit("".to_string());
            }
            TopLevelKind::Asm(lines) => {
                for line in lines {
                    self.emit(format!("  {}", line));
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn generate_block(&mut self, statements: &[Statement]) -> Result<(), String> {
        for stmt in statements {
            self.generate_statement(stmt)?;
        }
        Ok(())
    }

    fn generate_statement(&mut self, stmt: &Statement) -> Result<(), String> {
        self.output.push(format!(
            ";@source {}",
            serde_json::json!({"file": stmt.source_file, "line": stmt.line})
        ));
        self.current_line = stmt.line;
        match &stmt.kind {
            StatementKind::Let(target, expr) => {
                // Optimization: Try static address first
                if let Ok(addr) = self.get_static_address(target) {
                    let target_type = self.resolve_type(target).ok_or("Unknown target type")?;
                    match target_type {
                        DataType::Word | DataType::String => match expr {
                            Expression::Integer(val) => {
                                let low = (val & 0xFF) as u8;
                                let high = ((val >> 8) & 0xFF) as u8;
                                self.emit(format!("  LDA #${:02X}", low));
                                self.emit(format!("  STA ${:04X}", addr));
                                self.emit(format!("  LDA #${:02X}", high));
                                self.emit(format!("  STA ${:04X}", addr + 1));
                            }
                            _ => {
                                let rtype = self.generate_expression(expr)?;
                                self.emit(format!("  STA ${:04X}", addr));
                                if matches!(
                                    rtype,
                                    DataType::Word | DataType::Int | DataType::String
                                ) {
                                    self.emit(format!("  STX ${:04X}", addr + 1));
                                } else {
                                    self.emit("  LDA #0".to_string());
                                    self.emit(format!("  STA ${:04X}", addr + 1));
                                }
                            }
                        },
                        _ => {
                            self.generate_expression(expr)?;
                            self.emit(format!("  STA ${:04X}", addr));
                        }
                    }
                } else {
                    // Dynamic L-Value (Array or Dynamic Member Access)
                    let target_type = self.resolve_type(target).ok_or("Unknown target type")?;
                    let stores_word = matches!(target_type, DataType::Word | DataType::String);
                    // 1. Calculate Value
                    let rtype = self.generate_expression(expr)?;
                    // The destination owns storage width. A WORD expression
                    // assigned to BYTE must not overwrite the next element;
                    // a BYTE assigned to WORD must initialize its high byte.
                    if !stores_word {
                        self.emit("  PHA".to_string());
                    } else {
                        if matches!(rtype, DataType::Byte | DataType::Bool) {
                            self.emit("  LDX #0".to_string());
                        }
                        self.emit("  PHA".to_string()); // Low
                        self.emit("  TXA".to_string());
                        self.emit("  PHA".to_string()); // High
                    }

                    // 2. Calculate Address -> $02/$03
                    self.generate_address_expression(target)?;

                    // 3. Restore Value and Store
                    if !stores_word {
                        self.emit("  PLA".to_string());
                        self.emit("  LDY #0".to_string());
                        self.emit("  STA ($02),Y".to_string());
                    } else {
                        // Word/String
                        self.emit("  PLA".to_string()); // High
                        self.emit("  TAX".to_string());
                        self.emit("  PLA".to_string()); // Low
                        self.emit("  LDY #0".to_string());
                        self.emit("  STA ($02),Y".to_string());
                        self.emit("  TXA".to_string());
                        self.emit("  INY".to_string());
                        self.emit("  STA ($02),Y".to_string());
                    }
                }
            }
            StatementKind::Call(target, args) => {
                if let Expression::MemberAccess(base, member) = target {
                    if matches!(&**base, Expression::Identifier(name) if name.eq_ignore_ascii_case("World"))
                    {
                        self.generate_expression(&Expression::Call(
                            Box::new(target.clone()),
                            args.clone(),
                        ))?;
                        return Ok(());
                    }
                    if let Expression::Identifier(base_name) = &**base {
                        if base_name.eq_ignore_ascii_case("Controller")
                            && member.eq_ignore_ascii_case("Read")
                        {
                            self.output
                                .push("  JSR Runtime_Controller_Read".to_string());
                            return Ok(());
                        } else if base_name.eq_ignore_ascii_case("Text") {
                            if member.eq_ignore_ascii_case("Print") {
                                // Arg 0: X -> $14
                                self.generate_expression(&args[0])?;
                                self.emit("  STA $14".to_string());
                                // Arg 1: Y -> $15
                                self.generate_expression(&args[1])?;
                                self.emit("  STA $15".to_string());
                                // Arg 2: Str -> $16/$17
                                let dtype = self.generate_expression(&args[2])?;
                                if dtype == DataType::String {
                                    self.emit("  STA $16".to_string());
                                    self.emit("  STX $17".to_string());
                                }
                                self.emit("  JSR Runtime_Text_Print".to_string());
                                return Ok(());
                            } else if member.eq_ignore_ascii_case("SetOffset") {
                                // Arg 0: Offset -> $18
                                self.generate_expression(&args[0])?;
                                self.emit("  STA $18".to_string());
                                return Ok(());
                            }
                        } else if base_name.eq_ignore_ascii_case("Sprite") {
                            if member.eq_ignore_ascii_case("DrawVariant") {
                                return self.generate_variant_draw(args, false);
                            }
                            if member.eq_ignore_ascii_case("Draw") {
                                // Arg 0: X -> $14
                                self.generate_expression(&args[0])?;
                                self.emit("  STA $14".to_string());
                                // Arg 1: Y -> $15
                                self.generate_expression(&args[1])?;
                                self.emit("  STA $15".to_string());
                                // Arg 2: Metasprite Ptr -> $16/$17
                                self.generate_expression(&args[2])?;
                                self.emit("  STA $16".to_string());
                                self.emit("  STX $17".to_string());
                                self.emit("  JSR Runtime_SpriteDraw".to_string());
                                return Ok(());
                            } else if member.eq_ignore_ascii_case("Clear") {
                                self.emit("  JSR Runtime_SpriteClear".to_string());
                                return Ok(());
                            } else if member.eq_ignore_ascii_case("SetFlicker") {
                                self.generate_expression(&args[0])?;
                                self.emit("  STA $1C".to_string());
                                return Ok(());
                            }
                        } else if base_name.eq_ignore_ascii_case("Pool")
                            && member.eq_ignore_ascii_case("Despawn")
                        {
                            // 1. Generate Array Base Address ($02/$03)
                            self.generate_address_expression(&args[0])?;

                            // 2. Save Base to Stack to protect from Index Expression Eval
                            self.emit("  LDA $03".to_string());
                            self.emit("  PHA".to_string());
                            self.emit("  LDA $02".to_string());
                            self.emit("  PHA".to_string());

                            // 3. Generate Index Expression
                            let idx_type = self.generate_expression(&args[1])?;

                            // 4. Save Index Result temporarily to $04/$05 (Safe ZP temps)
                            self.emit("  STA $04".to_string());
                            if idx_type == DataType::Word || idx_type == DataType::Int {
                                self.emit("  STX $05".to_string());
                            } else {
                                self.emit("  LDA #0".to_string());
                                self.emit("  STA $05".to_string());
                            }

                            // 5. Restore Base to $02/$03
                            self.emit("  TSX".to_string());
                            self.emit("  LDA $0101, X".to_string()); // Base Low
                            self.emit("  STA $02".to_string());
                            self.emit("  LDA $0102, X".to_string()); // Base High
                            self.emit("  STA $03".to_string());
                            self.emit("  PLA".to_string()); // Pop Low
                            self.emit("  PLA".to_string()); // Pop High

                            // 6. Setup Stride ($06)
                            // Re-calculate stride (safe now)
                            let dtype = self
                                .resolve_type(&args[0])
                                .ok_or("Cannot resolve array type in Pool.Despawn")?;
                            let (stride, count) = if let DataType::Array(inner, count) = dtype {
                                (self.get_type_size(&inner)?, count)
                            } else {
                                return Err("Pool.Despawn expects an array".into());
                            };
                            if !(1..=255).contains(&stride) {
                                return Err("Pool entity size must be 1..255 bytes".into());
                            }
                            self.emit(format!("  LDA #{}", stride));
                            self.emit("  STA $06".to_string());

                            // Invalid and negative indices are no-ops; compare the
                            // full unsigned index before writing any neighboring RAM.
                            let done = self.new_label();
                            self.emit("  LDA $04".to_string());
                            self.emit(format!("  CMP #${:02X}", count & 0xff));
                            self.emit("  LDA $05".to_string());
                            self.emit(format!("  SBC #${:02X}", (count >> 8) & 0xff));
                            self.emit(format!("  BCS {done}"));

                            // 7. Push Index to Stack (Runtime expects it there)
                            self.emit("  LDA $04".to_string()); // Low
                            self.emit("  PHA".to_string());
                            self.emit("  LDA $05".to_string()); // High
                            self.emit("  PHA".to_string());

                            self.emit("  JSR Runtime_Pool_Despawn".to_string());
                            self.emit(format!("{done}:"));
                            return Ok(());
                        } else if base_name.eq_ignore_ascii_case("Animation") {
                            if member.eq_ignore_ascii_case("DrawVariant") {
                                return self.generate_variant_draw(args, true);
                            }
                            if member.eq_ignore_ascii_case("Play") {
                                // Arg 0: State (Address)
                                let state_addr = self.get_static_address(&args[0])?;

                                // Arg 1: Anim (Address) -> A/X
                                self.generate_expression(&args[1])?;
                                // Save Ptr to State (Offset 0)
                                self.emit(format!("  STA ${:04X}", state_addr));
                                self.emit(format!("  STX ${:04X}", state_addr + 1));

                                // Init Frame Index (Offset 2) = 0
                                self.emit("  LDA #0".to_string());
                                self.emit(format!("  STA ${:04X}", state_addr + 2));

                                // Init Timer (Offset 3) = Duration of Frame 0
                                // Use $02/$03 as temp for Anim Ptr (since A/X had it)
                                self.emit(format!("  LDA ${:04X}", state_addr));
                                self.emit("  STA $02".to_string());
                                self.emit(format!("  LDA ${:04X}", state_addr + 1));
                                self.emit("  STA $03".to_string());

                                // Offset to Duration of Frame 0 = 2 + 0*3 + 2 = 4
                                self.emit("  LDY #4".to_string());
                                self.emit("  LDA ($02), Y".to_string());
                                self.emit(format!("  STA ${:04X}", state_addr + 3));

                                // Init Finished (Offset 4) = 0
                                self.emit("  LDA #0".to_string());
                                self.emit(format!("  STA ${:04X}", state_addr + 4));
                                return Ok(());
                            } else if member.eq_ignore_ascii_case("Update") {
                                // Arg 0: State
                                let state_addr = self.get_static_address(&args[0])?;
                                self.output
                                    .push(format!("  LDA #${:02X}", (state_addr & 0xFF) as u8));
                                self.emit("  STA $02".to_string());
                                self.emit(format!(
                                    "  LDA #${:02X}",
                                    ((state_addr >> 8) & 0xFF) as u8
                                ));
                                self.emit("  STA $03".to_string());
                                self.emit("  JSR Runtime_Anim_Update".to_string());
                                return Ok(());
                            } else if member.eq_ignore_ascii_case("Draw") {
                                // Arg 0: X -> $14
                                self.generate_expression(&args[0])?;
                                self.emit("  STA $14".to_string());
                                // Arg 1: Y -> $15
                                self.generate_expression(&args[1])?;
                                self.emit("  STA $15".to_string());
                                // Arg 2: State -> $16/$17
                                let state_addr = self.get_static_address(&args[2])?;
                                self.output
                                    .push(format!("  LDA #${:02X}", (state_addr & 0xFF) as u8));
                                self.emit("  STA $16".to_string());
                                self.emit(format!(
                                    "  LDA #${:02X}",
                                    ((state_addr >> 8) & 0xFF) as u8
                                ));
                                self.emit("  STA $17".to_string());
                                self.emit("  JSR Runtime_Anim_Draw".to_string());
                                return Ok(());
                            }
                        } else if base_name.eq_ignore_ascii_case("Scroll") {
                            if member.eq_ignore_ascii_case("Set") {
                                // Store fine X and preserve horizontal screen parity while Y evaluates.
                                let x_type = self.generate_expression(&args[0])?;
                                self.emit("  STA $E0".to_string());
                                if x_type == DataType::Word {
                                    self.emit("  TXA".to_string());
                                    self.emit("  AND #1".to_string());
                                } else {
                                    self.emit("  LDA #0".to_string());
                                }
                                self.emit("  PHA".to_string());
                                let y_type = self.generate_expression(&args[1])?;
                                self.emit("  STA $02".to_string());
                                if y_type == DataType::Word {
                                    self.emit("  STX $03".to_string());
                                } else {
                                    self.emit("  LDA #0".to_string());
                                    self.emit("  STA $03".to_string());
                                }
                                self.emit("  JSR Runtime_Scroll_NormalizeY".to_string());
                                self.emit("  LDA $02".to_string());
                                self.emit("  STA $E1".to_string());
                                self.emit("  LDA $07".to_string());
                                self.emit("  ASL".to_string());
                                self.emit("  STA $07".to_string());
                                self.emit("  PLA".to_string());
                                self.emit("  ORA $07".to_string());
                                self.emit("  STA $07".to_string());
                                self.emit("  LDA $F8".to_string());
                                self.emit("  AND #$FC".to_string());
                                self.emit("  ORA $07".to_string());
                                self.emit("  STA $F8".to_string());
                                self.emit("  STA $2000".to_string());
                                return Ok(());
                            } else if member.eq_ignore_ascii_case("LoadColumn") {
                                self.generate_scroll_arguments(args)?;
                                self.output
                                    .push("  JSR Runtime_Scroll_LoadColumn".to_string());
                                return Ok(());
                            } else if member.eq_ignore_ascii_case("LoadRow") {
                                self.generate_scroll_arguments(args)?;
                                self.emit("  JSR Runtime_Scroll_LoadRow".to_string());
                                return Ok(());
                            }
                        } else if base_name.eq_ignore_ascii_case("PPU") {
                            if member.eq_ignore_ascii_case("Ctrl") {
                                self.generate_expression(&args[0])?;
                                self.emit("  STA $F8".to_string()); // Shadow
                                self.emit("  STA $2000".to_string());
                                return Ok(());
                            } else if member.eq_ignore_ascii_case("Mask") {
                                self.generate_expression(&args[0])?;
                                self.emit("  STA $F9".to_string()); // Mask shadow for full map transitions.
                                self.emit("  STA $2001".to_string());
                                return Ok(());
                            } else if member.eq_ignore_ascii_case("SetScroll") {
                                // Arg 0: X -> $E0
                                self.generate_expression(&args[0])?;
                                self.emit("  STA $E0".to_string());
                                // Arg 1: Y -> $E1
                                self.generate_expression(&args[1])?;
                                self.emit("  STA $E1".to_string());
                                return Ok(());
                            }
                        } else if base_name.eq_ignore_ascii_case("Memory") {
                            if member.eq_ignore_ascii_case("Fill") {
                                // Memory.Fill(address, length, value)
                                // 1. Value -> A
                                self.generate_expression(&args[2])?;
                                self.emit("  PHA".to_string()); // Save Value

                                // 2. Length -> A (Byte or Low Byte of Word). Keep it
                                // on the stack because address generation uses $00/$01.
                                self.generate_expression(&args[1])?;
                                self.emit("  PHA".to_string()); // Save Length

                                // 3. Address -> $02/$03
                                self.generate_address_expression(&args[0])?;

                                // Loop: Fill memory from $02/$03 using Y index
                                let fill_loop = self.new_label();
                                let fill_end = self.new_label();
                                self.emit("  PLA".to_string()); // Restore Length
                                self.emit("  STA $04".to_string());
                                self.emit("  PLA".to_string()); // Restore Value in A
                                self.emit("  LDY #0".to_string());
                                self.emit(format!("{}:", fill_loop));
                                self.emit("  CPY $04".to_string());
                                self.emit(format!("  BEQ {}", fill_end));
                                self.emit("  STA ($02),Y".to_string());
                                self.emit("  INY".to_string());
                                self.emit(format!("  JMP {}", fill_loop));
                                self.emit(format!("{}:", fill_end));
                                return Ok(());
                            } else if member.eq_ignore_ascii_case("Copy") {
                                // Memory.Copy(src_address, dst_address, length)
                                // Arg 2: length -> preserve on stack
                                self.generate_expression(&args[2])?;
                                self.emit("  PHA".to_string()); // Length on stack

                                // Arg 1: dst_address -> $02/$03
                                self.generate_address_expression(&args[1])?;
                                self.emit("  LDA $02".to_string());
                                self.emit("  PHA".to_string()); // Dst Low on stack
                                self.emit("  LDA $03".to_string());
                                self.emit("  PHA".to_string()); // Dst High on stack

                                // Arg 0: src_address -> $00/$01
                                self.generate_address_expression(&args[0])?;
                                self.emit("  LDA $02".to_string());
                                self.emit("  STA $00".to_string());
                                self.emit("  LDA $03".to_string());
                                self.emit("  STA $01".to_string());

                                // Restore Dst to $02/$03
                                self.emit("  PLA".to_string());
                                self.emit("  STA $03".to_string());
                                self.emit("  PLA".to_string());
                                self.emit("  STA $02".to_string());

                                // Restore Length
                                self.emit("  PLA".to_string());
                                self.emit("  STA $04".to_string());

                                let copy_loop = self.new_label();
                                let copy_end = self.new_label();
                                self.emit("  LDY #0".to_string());
                                self.emit(format!("{}:", copy_loop));
                                self.emit("  CPY $04".to_string());
                                self.emit(format!("  BEQ {}", copy_end));
                                self.emit("  LDA ($00),Y".to_string());
                                self.emit("  STA ($02),Y".to_string());
                                self.emit("  INY".to_string());
                                self.emit(format!("  JMP {}", copy_loop));
                                self.emit(format!("{}:", copy_end));
                                return Ok(());
                            }
                        } else if base_name.eq_ignore_ascii_case("Sound") {
                            if member.eq_ignore_ascii_case("Stop") {
                                self.emit("  JSR Sound_Init".to_string());
                            } else if member.eq_ignore_ascii_case("Play") {
                                let id_type = self.generate_expression(&args[0])?;
                                let done = self.new_label();
                                if id_type == DataType::Word {
                                    self.emit("  CPX #0".to_string());
                                    self.emit(format!("  BNE {done}"));
                                }
                                self.emit("  JSR Sound_Play".to_string());
                                self.emit(format!("{done}:"));
                            }
                            return Ok(());
                        }
                    }
                }

                if let Expression::Identifier(name) = target {
                    let (target_bank, params) = if let Some((b, p)) = self.sub_signatures.get(name)
                    {
                        (*b, p.clone())
                    } else {
                        return Err(format!("Undefined sub '{}'", name));
                    };
                    if args.len() != params.len() {
                        return Err(format!("Arg mismatch for {}", name));
                    }
                    self.generate_args(&params, args)?;
                    if target_bank != self.current_bank && target_bank != 7 {
                        self.emit(format!("  JSR Trampoline_{}", name));
                    } else {
                        self.emit(format!("  JSR {}", name));
                    }
                } else {
                    return Err("Only direct sub calls are supported in Statement".to_string());
                }
            }
            StatementKind::If(condition, then_block, else_block) => {
                let else_label = self.new_label();
                let end_label = self.new_label();
                self.generate_expression(condition)?;
                self.emit("  CMP #0".to_string());
                if else_block.is_some() {
                    self.emit(format!("  BEQ {}", else_label));
                } else {
                    self.emit(format!("  BEQ {}", end_label));
                }
                self.generate_block(then_block)?;
                self.emit(format!("  JMP {}", end_label));
                if let Some(else_stmts) = else_block {
                    self.emit(format!("{}:", else_label));
                    self.generate_block(else_stmts)?;
                }
                self.emit(format!("{}:", end_label));
            }
            StatementKind::While(condition, body) => {
                let start_label = self.new_label();
                let end_label = self.new_label();
                self.emit(format!("{}:", start_label));
                self.generate_expression(condition)?;
                self.emit("  CMP #0".to_string());
                self.emit(format!("  BEQ {}", end_label));
                self.generate_block(body)?;
                self.emit(format!("  JMP {}", start_label));
                self.emit(format!("{}:", end_label));
            }
            StatementKind::DoWhile(body, condition) => {
                let start_label = self.new_label();
                self.emit(format!("{}:", start_label));
                self.generate_block(body)?;
                self.generate_expression(condition)?;
                self.emit("  CMP #0".to_string());
                self.emit(format!("  BNE {}", start_label));
            }
            StatementKind::For(var_name, start_expr, end_expr, step_expr, body) => {
                let loop_label = self.new_label();
                let exit_label = self.new_label();

                // Determine step direction and value to use
                let (is_negative_step, step_val_expr) = match step_expr {
                    Some(expr) => {
                        let is_neg = match expr {
                            Expression::Integer(val) => *val < 0,
                            Expression::UnaryOp(UnaryOperator::Negate, _) => true,
                            _ => false,
                        };
                        (is_neg, expr.clone())
                    }
                    None => (false, Expression::Integer(1)),
                };

                // Init: var = start
                let init_stmt = Statement {
                    source_file: stmt.source_file.clone(),
                    kind: StatementKind::Let(
                        Expression::Identifier(var_name.clone()),
                        start_expr.clone(),
                    ),
                    line: self.current_line,
                };
                self.generate_statement(&init_stmt)?;

                self.emit(format!("{}:", loop_label));

                // Condition:
                // If step < 0: Check var >= end
                // If step >= 0: Check var <= end
                let operator = if is_negative_step {
                    BinaryOperator::GreaterThanOrEqual
                } else {
                    BinaryOperator::LessThanOrEqual
                };

                // Synthesize condition expression
                let condition = Expression::BinaryOp(
                    Box::new(Expression::Identifier(var_name.clone())),
                    operator,
                    Box::new(end_expr.clone()),
                );

                self.generate_expression(&condition)?;

                // Result is Boolean ($FF or $00) in A
                self.emit("  CMP #0".to_string());
                self.emit(format!("  BEQ {}", exit_label)); // Exit if False

                // Body
                self.generate_block(body)?;

                // Increment: var = var + step
                let increment = Statement {
                    source_file: stmt.source_file.clone(),
                    kind: StatementKind::Let(
                        Expression::Identifier(var_name.clone()),
                        Expression::BinaryOp(
                            Box::new(Expression::Identifier(var_name.clone())),
                            BinaryOperator::Add,
                            Box::new(step_val_expr),
                        ),
                    ),
                    line: self.current_line,
                };
                self.generate_statement(&increment)?;

                self.emit(format!("  JMP {}", loop_label));
                self.emit(format!("{}:", exit_label));
            }
            StatementKind::Return(expr) => {
                if let Some(e) = expr {
                    self.generate_expression(e)?;
                }
                for _ in 0..self.select_stack_depth {
                    self.emit("  PLA".to_string());
                }
                self.emit("  RTS".to_string());
            }
            StatementKind::Asm(lines) => {
                for line in lines {
                    self.emit(format!("  {}", line));
                }
            }
            StatementKind::Poke(addr_expr, val_expr) => {
                self.generate_expression(val_expr)?;
                self.emit("  PHA".to_string());

                // Optimization: Check for constant address
                let const_addr = match addr_expr {
                    Expression::Integer(val) => Some(*val as u16),
                    Expression::Identifier(name) => self
                        .symbol_table
                        .resolve(name)
                        .and_then(|sym| sym.value.map(|val| val as u16)),
                    _ => None,
                };

                if let Some(addr) = const_addr {
                    self.emit("  PLA".to_string());
                    self.emit(format!("  STA ${:04X}", addr));
                } else {
                    self.generate_address_expression(addr_expr)?;
                    self.emit("  LDY #$00".to_string());
                    self.emit("  PLA".to_string());
                    self.emit("  STA ($02),y".to_string());
                }
            }
            StatementKind::PlaySfx(id_expr) => {
                let id_type = self.generate_expression(id_expr)?;
                let done = self.new_label();
                if id_type == DataType::Word {
                    self.emit("  CPX #0".to_string());
                    self.emit(format!("  BNE {done}"));
                }
                self.emit("  JSR SFX_Play".to_string());
                self.emit(format!("{done}:"));
            }
            StatementKind::Read(vars) => {
                for name in vars {
                    if let Some(sym) = self.symbol_table.resolve(name) {
                        if let Some(addr) = sym.address {
                            match sym.data_type {
                                DataType::Byte | DataType::Bool | DataType::Int => {
                                    self.emit("  JSR Runtime_ReadByte".to_string());
                                    self.emit(format!("  STA ${:04X}", addr));
                                }
                                DataType::Word => {
                                    self.emit("  JSR Runtime_ReadByte".to_string());
                                    self.emit(format!("  STA ${:04X}", addr));
                                    self.emit("  JSR Runtime_ReadByte".to_string());
                                    self.emit(format!("  STA ${:04X}", addr + 1));
                                }
                                DataType::String => {
                                    self.emit("  JSR Runtime_ReadString".to_string());
                                    self.emit(format!("  STA ${:04X}", addr));
                                    self.emit(format!("  STX ${:04X}", addr + 1));
                                }
                                _ => {
                                    // Structs/Arrays not supported in READ
                                }
                            }
                        }
                    }
                }
            }
            StatementKind::Restore(label) => {
                let target = label.as_deref().unwrap_or("USER_DATA_START");
                let addr = self.data_table_offsets.get(target).cloned();
                if let Some(addr) = addr {
                    self.emit(format!("  LDA ${:04X}", addr));
                    self.emit("  STA $04".to_string());
                    self.emit(format!("  LDA ${:04X}", addr + 1));
                    self.emit("  STA $05".to_string());
                }
            }
            StatementKind::On(event, handler) => {
                let selector = match event.to_ascii_uppercase().as_str() {
                    "NMI" => 0x07f8,
                    "IRQ" => 0x07f9,
                    _ => return Err(format!("Unsupported interrupt event {event}")),
                };
                let index = self
                    .interrupt_targets
                    .iter()
                    .position(|(name, _)| name == handler)
                    .ok_or_else(|| {
                        format!("Interrupt handler {handler} must be a zero-argument routine")
                    })?
                    + 1;
                self.emit(format!("  LDA #{index}"));
                self.emit(format!("  STA ${selector:04X}"));
            }
            StatementKind::WaitVBlank => {
                let lbl = self.new_label();
                let hardware = self.new_label();
                let done = self.new_label();
                self.emit("  LDA $F8".to_string());
                self.emit(format!("  BPL {hardware}"));
                self.emit("  LDA $FD".to_string());
                self.emit(format!("{}:", lbl));
                self.emit("  CMP $FD".to_string());
                self.emit(format!("  BEQ {lbl}"));
                self.emit(format!("  JMP {done}"));
                self.emit(format!("{hardware}:"));
                self.emit("  BIT $2002".to_string());
                self.emit(format!("  BPL {hardware}"));
                self.emit(format!("{done}:"));
            }
            StatementKind::Randomize(expr) => {
                self.generate_expression(expr)?;
                self.emit("  JSR Runtime_Randomize".to_string());
            }
            StatementKind::Select(expr, cases, case_else) => {
                let end_select_label = self.new_label();

                // 1. Evaluate Expression
                let expr_type = self.generate_expression(expr)?;

                // 2. Push to Stack
                let pushed_bytes = match expr_type {
                    DataType::Byte | DataType::Bool | DataType::Int | DataType::Enum(_) => {
                        self.emit("  PHA".to_string());
                        1
                    }
                    DataType::Word | DataType::String => {
                        self.emit("  PHA".to_string()); // Low
                        self.emit("  TXA".to_string());
                        self.emit("  PHA".to_string()); // High
                        2
                    }
                    DataType::Struct(_) | DataType::Array(_, _) => {
                        return Err("Cannot SELECT on struct/array".to_string());
                    }
                };
                self.select_stack_depth += pushed_bytes;

                // 3. Generate Checks
                for (case_val, case_body) in cases {
                    let next_case = self.new_label();

                    // Eval Case Value -> A/X
                    self.generate_expression(case_val)?;

                    // Compare (Stack vs A/X)
                    self.emit("  TSX".to_string());

                    match expr_type {
                        DataType::Byte | DataType::Bool | DataType::Int | DataType::Enum(_) => {
                            // A has Case Val. Stack has Select Val at $0101,X
                            self.emit("  CMP $0101, X".to_string());
                            self.emit(format!("  BNE {}", next_case));
                        }
                        _ => {
                            // A=Low, X=High of Case Val.
                            // Stack: High at 101, Low at 102.
                            // We must save X (High Byte) because TSX clobbers it.
                            self.emit("  STX $01".to_string()); // Save High
                            self.emit("  TSX".to_string());

                            self.emit("  CMP $0102, X".to_string()); // Compare Low
                            self.emit(format!("  BNE {}", next_case));

                            self.emit("  LDA $01".to_string()); // Restore High
                            self.emit("  CMP $0101, X".to_string()); // Compare High
                            self.emit(format!("  BNE {}", next_case));
                        }
                    }

                    // If we are here, it matched.
                    self.generate_block(case_body)?;
                    self.emit(format!("  JMP {}", end_select_label));

                    self.emit(format!("{}:", next_case));
                }

                // Else
                if let Some(else_body) = case_else {
                    self.generate_block(else_body)?;
                }

                self.emit(format!("{}:", end_select_label));

                // 4. Pop Stack
                for _ in 0..pushed_bytes {
                    self.emit("  PLA".to_string());
                }
                self.select_stack_depth -= pushed_bytes;
            }
            _ => {}
        }
        Ok(())
    }

    fn generate_args(
        &mut self,
        params: &[(u16, DataType)],
        args: &[Expression],
    ) -> Result<(), String> {
        for (i, expr) in args.iter().enumerate() {
            let (addr, dtype) = params[i].clone();
            match dtype {
                DataType::Word | DataType::String => {
                    let rtype = self.generate_expression(expr)?;
                    self.emit(format!("  STA ${:04X}", addr));
                    if rtype == DataType::Word {
                        self.emit(format!("  STX ${:04X}", addr + 1));
                    } else {
                        self.emit("  LDA #0".to_string());
                        self.emit(format!("  STA ${:04X}", addr + 1));
                    }
                }
                _ => {
                    self.generate_expression(expr)?;
                    self.emit(format!("  STA ${:04X}", addr));
                }
            }
        }
        Ok(())
    }

    fn get_static_address(&self, expr: &Expression) -> Result<u16, String> {
        match expr {
            Expression::Identifier(name) => self
                .symbol_table
                .resolve(name)
                .and_then(|s| s.address)
                .ok_or_else(|| format!("Symbol '{}' has no address", name)),
            Expression::MemberAccess(base, member) => {
                let base_type = self.resolve_type(base).ok_or("Unknown base type")?;
                match base_type {
                    DataType::Struct(s_name) => {
                        let base_addr = self.get_static_address(base)?;
                        if let Some(sym) = self.symbol_table.resolve(&s_name) {
                            if let Some(members) = &sym.members {
                                for (m_name, _, offset) in members {
                                    if m_name == member {
                                        return Ok(base_addr + offset);
                                    }
                                }
                            }
                        }
                        Err("Struct Member error".to_string())
                    }
                    DataType::Enum(_) => Err("Cannot take address of enum member".to_string()),
                    _ => Err("Member access on non-struct/enum".to_string()),
                }
            }
            _ => Err("Not static".to_string()),
        }
    }

    fn resolve_type(&self, expr: &Expression) -> Option<DataType> {
        match expr {
            Expression::Identifier(name) => {
                self.symbol_table.resolve(name).map(|s| s.data_type.clone())
            }
            Expression::MemberAccess(base, member) => {
                let base_type = self.resolve_type(base)?;
                match base_type {
                    DataType::Struct(struct_name) => {
                        if let Some(sym) = self.symbol_table.resolve(&struct_name) {
                            if let Some(members) = &sym.members {
                                for (m, t, _) in members {
                                    if m == member {
                                        return Some(t.clone());
                                    }
                                }
                            }
                        }
                    }
                    DataType::Enum(enum_name) => {
                        if let Some(sym) = self.symbol_table.resolve(&enum_name) {
                            if let Some(variants) = &sym.variants {
                                for (v_name, _) in variants {
                                    if v_name == member {
                                        return Some(DataType::Int);
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
                None
            }
            Expression::Call(callee, _) => {
                // If array, return inner type
                if let Some(DataType::Array(inner, _)) = self.resolve_type(callee) {
                    return Some(*inner);
                }
                Some(DataType::Word) // Default for Function call
            }
            Expression::Integer(_) => Some(DataType::Word),
            Expression::BinaryOp(_, _, _) => Some(DataType::Word),
            Expression::UnaryOp(_, _) => Some(DataType::Int),
            _ => Some(DataType::Byte),
        }
    }

    // Generates address in $02/$03
    fn generate_array_address(
        &mut self,
        callee: &Expression,
        index_expr: &Expression,
    ) -> Result<(), String> {
        // 1. Get Base Address
        let base_addr = self.get_static_address(callee)?;

        // 2. Evaluate Index -> A (Byte) or A/X (Word)
        // We will promote Byte to Word ($00/$01)
        let idx_type = self.generate_expression(index_expr)?;

        self.emit("  STA $00".to_string());
        if idx_type == DataType::Word || idx_type == DataType::Int {
            self.emit("  STX $01".to_string());
        } else {
            self.emit("  LDA #0".to_string());
            self.emit("  STA $01".to_string());
        }

        // 3. Get Element Size
        let elem_size = if let Some(DataType::Array(inner, _)) = self.resolve_type(callee) {
            self.get_type_size(&inner)?
        } else {
            1
        };

        // 4. Multiply Index * Size
        if elem_size > 1 {
            self.emit("  LDA $00".to_string()); // Low
            self.emit("  LDX $01".to_string()); // High
            self.emit(format!("  LDY #{}", elem_size)); // Size (Assuming small size, < 256)
                                                        // Math_Mul16 uses A/X * $00/$01.
                                                        // We need X * Y?
                                                        // We don't have Mul16(A, Y).
                                                        // Helper Mul16 expects $00/$01.
                                                        // Save size to $00/$01? No, Index is there.
                                                        // Move Index to A/X.
                                                        // Move Size to $00/$01.
            self.emit("  PHA".to_string()); // Index Low
            self.emit("  TXA".to_string());
            self.emit("  PHA".to_string()); // Index High

            self.emit(format!("  LDA #{}", elem_size));
            self.emit("  STA $00".to_string());
            self.emit("  LDA #0".to_string());
            self.emit("  STA $01".to_string());

            self.emit("  PLA".to_string()); // High
            self.emit("  TAX".to_string());
            self.emit("  PLA".to_string()); // Low

            self.emit("  JSR Math_Mul16".to_string());
            // Result in A/X
            self.emit("  STA $00".to_string());
            self.emit("  STX $01".to_string());
        }

        // 5. Add Base Address
        self.emit("  LDA $00".to_string());
        self.emit("  CLC".to_string());
        self.output
            .push(format!("  ADC #${:02X}", (base_addr & 0xFF) as u8));
        self.emit("  STA $02".to_string());

        self.emit("  LDA $01".to_string());
        self.output
            .push(format!("  ADC #${:02X}", ((base_addr >> 8) & 0xFF) as u8));
        self.emit("  STA $03".to_string());

        Ok(())
    }

    fn generate_address_expression(&mut self, expr: &Expression) -> Result<(), String> {
        match expr {
            Expression::Call(callee, args) => {
                self.generate_array_address(callee, &args[0])?;
            }
            Expression::MemberAccess(base, member) => {
                if let Ok(addr) = self.get_static_address(expr) {
                    let low = (addr & 0xFF) as u8;
                    let high = ((addr >> 8) & 0xFF) as u8;
                    self.emit(format!("  LDA #${:02X}", low));
                    self.emit("  STA $02".to_string());
                    self.emit(format!("  LDA #${:02X}", high));
                    self.emit("  STA $03".to_string());
                } else {
                    self.generate_address_expression(base)?;
                    let base_type = self.resolve_type(base).ok_or("Unknown base type")?;
                    if let DataType::Struct(s_name) = base_type {
                        if let Some(sym) = self.symbol_table.resolve(&s_name) {
                            if let Some(members) = &sym.members {
                                for (m_name, _, offset) in members {
                                    if m_name == member {
                                        self.emit(format!("  LDA #${:02X}", offset));
                                        self.emit("  CLC".to_string());
                                        self.emit("  ADC $02".to_string());
                                        self.emit("  STA $02".to_string());
                                        self.emit("  LDA #0".to_string());
                                        self.emit("  ADC $03".to_string());
                                        self.emit("  STA $03".to_string());
                                        return Ok(());
                                    }
                                }
                            }
                        }
                    }
                    return Err("Member access on non-struct".to_string());
                }
            }
            Expression::Integer(val) => {
                let low = (val & 0xFF) as u8;
                let high = ((val >> 8) & 0xFF) as u8;
                self.emit(format!("  LDA #${:02X}", low));
                self.emit("  STA $02".to_string());
                self.emit(format!("  LDA #${:02X}", high));
                self.emit("  STA $03".to_string());
            }
            Expression::Identifier(name) => {
                if let Some(sym) = self.symbol_table.resolve(name) {
                    if let Some(addr) = sym.address {
                        match sym.data_type {
                            DataType::Array(_, _) | DataType::Struct(_) => {
                                // Aggregate arguments denote their storage; loading the first
                                // byte as a pointer made upload/copy routines read unrelated RAM.
                                self.emit(format!("  LDA #${:02X}", addr & 0xff));
                                self.emit("  STA $02".to_string());
                                self.emit(format!("  LDA #${:02X}", addr >> 8));
                                self.emit("  STA $03".to_string());
                            }
                            DataType::Word | DataType::String => {
                                self.emit(format!("  LDA ${:04X}", addr));
                                self.emit("  STA $02".to_string());
                                self.emit(format!("  LDA ${:04X}", addr + 1));
                                self.emit("  STA $03".to_string());
                            }
                            _ => {
                                self.emit(format!("  LDA ${:04X}", addr));
                                self.emit("  STA $02".to_string());
                                self.emit("  LDA #0".to_string());
                                self.emit("  STA $03".to_string());
                            }
                        }
                    } else if sym.kind == SymbolKind::Constant {
                        if let Some(val) = sym.value {
                            let low = (val & 0xFF) as u8;
                            let high = ((val >> 8) & 0xFF) as u8;
                            self.emit(format!("  LDA #${:02X}", low));
                            self.emit("  STA $02".to_string());
                            self.emit(format!("  LDA #${:02X}", high));
                            self.emit("  STA $03".to_string());
                        } else {
                            return Err(format!("Constant '{}' has no value", name));
                        }
                    } else {
                        return Err(format!("Symbol '{}' has no address", name));
                    }
                } else {
                    return Err(format!("Undefined symbol '{}'", name));
                }
            }
            _ => {
                if let Ok(addr) = self.get_static_address(expr) {
                    let low = (addr & 0xFF) as u8;
                    let high = ((addr >> 8) & 0xFF) as u8;
                    self.emit(format!("  LDA #${:02X}", low));
                    self.emit("  STA $02".to_string());
                    self.emit(format!("  LDA #${:02X}", high));
                    self.emit("  STA $03".to_string());
                } else {
                    // Dynamic address
                    let dtype = self.generate_expression(expr)?;
                    self.emit("  STA $02".to_string());
                    if dtype == DataType::Word || dtype == DataType::Int {
                        self.emit("  STX $03".to_string());
                    } else {
                        self.emit("  LDA #0".to_string());
                        self.emit("  STA $03".to_string());
                    }
                }
            }
        }
        Ok(())
    }

    fn generate_expression(&mut self, expr: &Expression) -> Result<DataType, String> {
        match expr {
            Expression::Call(callee, args) => {
                // Controller Logic
                if let Expression::MemberAccess(base, member) = &**callee {
                    if matches!(&**base, Expression::Identifier(name) if name.eq_ignore_ascii_case("World"))
                    {
                        if args.len() != 2
                            || (!member.eq_ignore_ascii_case("Load")
                                && !member.eq_ignore_ascii_case("Tile"))
                        {
                            return Err(format!("World.{member} requires 2 numeric arguments"));
                        }
                        // Evaluating y may reuse all expression temporaries; stack x.
                        let x_type = self.generate_expression(&args[0])?;
                        self.emit("  PHA".into());
                        if matches!(x_type, DataType::Word | DataType::Int) {
                            self.emit("  TXA".into());
                        } else {
                            self.emit("  LDA #0".into());
                        }
                        self.emit("  PHA".into());
                        let y_type = self.generate_expression(&args[1])?;
                        self.emit("  STA $02".into());
                        if matches!(y_type, DataType::Word | DataType::Int) {
                            self.emit("  STX $03".into());
                        } else {
                            self.emit("  LDA #0".into());
                            self.emit("  STA $03".into());
                        }
                        self.emit("  PLA".into());
                        self.emit("  STA $01".into());
                        self.emit("  PLA".into());
                        self.emit("  STA $00".into());
                        let load = member.eq_ignore_ascii_case("Load");
                        self.emit(format!(
                            "  JSR Runtime_World_{}",
                            if load { "Load" } else { "Tile" }
                        ));
                        return Ok(if load { DataType::Bool } else { DataType::Byte });
                    }
                }
                if let Expression::MemberAccess(base, member) = &**callee {
                    if let Expression::Identifier(base_name) = &**base {
                        if base_name.eq_ignore_ascii_case("Pool")
                            && member.eq_ignore_ascii_case("Spawn")
                        {
                            self.generate_address_expression(&args[0])?;

                            let dtype = self
                                .resolve_type(&args[0])
                                .ok_or("Cannot resolve array type in Pool.Spawn")?;
                            let (count, stride) = if let DataType::Array(inner, size) = dtype {
                                (size, self.get_type_size(&inner)?)
                            } else {
                                return Err("Pool.Spawn expects an array".into());
                            };
                            if !(1..=255).contains(&stride) {
                                return Err("Pool entity size must be 1..255 bytes".into());
                            }

                            self.output
                                .push(format!("  LDA #${:02X}", (count & 0xFF) as u8));
                            self.emit("  STA $04".to_string());
                            self.output
                                .push(format!("  LDA #${:02X}", ((count >> 8) & 0xFF) as u8));
                            self.emit("  STA $05".to_string());

                            self.emit(format!("  LDA #{}", stride));
                            self.emit("  STA $06".to_string());

                            self.emit("  JSR Runtime_Pool_Spawn".to_string());
                            return Ok(DataType::Int);
                        }
                        if base_name.eq_ignore_ascii_case("Collision") {
                            if member.eq_ignore_ascii_case("Rect") {
                                for arg in args {
                                    let dtype = self.generate_expression(arg)?;
                                    if dtype == DataType::Word
                                        || dtype == DataType::Int
                                        || dtype == DataType::String
                                    {
                                        self.emit("  PHA".to_string()); // Low
                                        self.emit("  TXA".to_string());
                                        self.emit("  PHA".to_string()); // High
                                    } else {
                                        self.emit("  PHA".to_string()); // Low
                                        self.emit("  LDA #0".to_string());
                                        self.emit("  PHA".to_string()); // High
                                    }
                                }
                                self.emit("  JSR Runtime_Collision_Rect".to_string());
                                self.emit("  TAY".to_string()); // Preserve result during stack cleanup
                                                                // Clean up stack (16 bytes)
                                self.emit("  TSX".to_string());
                                self.emit("  TXA".to_string());
                                self.emit("  CLC".to_string());
                                self.emit("  ADC #16".to_string());
                                self.emit("  TAX".to_string());
                                self.emit("  TXS".to_string());
                                self.emit("  TYA".to_string());
                                self.emit("  LDX #0".to_string());
                                return Ok(DataType::Bool);
                            } else if member.eq_ignore_ascii_case("Point") {
                                for arg in args {
                                    let dtype = self.generate_expression(arg)?;
                                    if dtype == DataType::Word
                                        || dtype == DataType::Int
                                        || dtype == DataType::String
                                    {
                                        self.emit("  PHA".to_string()); // Low
                                        self.emit("  TXA".to_string());
                                        self.emit("  PHA".to_string()); // High
                                    } else {
                                        self.emit("  PHA".to_string()); // Low
                                        self.emit("  LDA #0".to_string());
                                        self.emit("  PHA".to_string()); // High
                                    }
                                }
                                self.output
                                    .push("  JSR Runtime_Collision_Point".to_string());
                                self.emit("  TAY".to_string()); // Preserve result during stack cleanup
                                                                // Clean up stack (6 args * 2 bytes = 12)
                                self.emit("  TSX".to_string());
                                self.emit("  TXA".to_string());
                                self.emit("  CLC".to_string());
                                self.emit("  ADC #12".to_string());
                                self.emit("  TAX".to_string());
                                self.emit("  TXS".to_string());
                                self.emit("  TYA".to_string());
                                self.emit("  LDX #0".to_string());
                                return Ok(DataType::Bool);
                            } else if member.eq_ignore_ascii_case("Tile") {
                                for arg in args {
                                    let dtype = self.generate_expression(arg)?;
                                    if dtype == DataType::Word
                                        || dtype == DataType::Int
                                        || dtype == DataType::String
                                    {
                                        self.emit("  PHA".to_string()); // Low
                                        self.emit("  TXA".to_string());
                                        self.emit("  PHA".to_string()); // High
                                    } else {
                                        self.emit("  PHA".to_string()); // Low
                                        self.emit("  LDA #0".to_string());
                                        self.emit("  PHA".to_string()); // High
                                    }
                                }
                                self.emit("  JSR Runtime_Collision_Tile".to_string());
                                self.emit("  TAY".to_string()); // Preserve result during stack cleanup
                                                                // Clean up stack (2 args * 2 bytes = 4)
                                self.emit("  TSX".to_string());
                                self.emit("  TXA".to_string());
                                self.emit("  CLC".to_string());
                                self.emit("  ADC #4".to_string());
                                self.emit("  TAX".to_string());
                                self.emit("  TXS".to_string());
                                self.emit("  TYA".to_string());
                                self.emit("  LDX #0".to_string());
                                return Ok(DataType::Byte);
                            }
                        }
                        if base_name.eq_ignore_ascii_case("Math")
                            && member.eq_ignore_ascii_case("Abs")
                        {
                            let dtype = self.generate_expression(&args[0])?;
                            if dtype == DataType::Word || dtype == DataType::Int {
                                self.emit("  CPX #$80".to_string());
                                let pos_lbl = self.new_label();
                                self.emit(format!("  BCC {}", pos_lbl));
                                self.emit("  EOR #$FF".to_string());
                                self.emit("  CLC".to_string());
                                self.emit("  ADC #1".to_string());
                                self.emit("  PHA".to_string());
                                self.emit("  TXA".to_string());
                                self.emit("  EOR #$FF".to_string());
                                self.emit("  ADC #0".to_string());
                                self.emit("  TAX".to_string());
                                self.emit("  PLA".to_string());
                                self.emit(format!("{}:", pos_lbl));
                                return Ok(dtype);
                            } else {
                                self.emit("  CMP #$80".to_string());
                                let pos_lbl = self.new_label();
                                self.emit(format!("  BCC {}", pos_lbl));
                                self.emit("  EOR #$FF".to_string());
                                self.emit("  CLC".to_string());
                                self.emit("  ADC #1".to_string());
                                self.emit(format!("{}:", pos_lbl));
                                return Ok(dtype);
                            }
                        }
                        if base_name.eq_ignore_ascii_case("Math") {
                            if member.eq_ignore_ascii_case("Min")
                                || member.eq_ignore_ascii_case("Max")
                            {
                                let is_max = member.eq_ignore_ascii_case("Max");
                                let t1 = self.generate_expression(&args[0])?;
                                let is_16_1 = t1 == DataType::Word || t1 == DataType::Int;
                                if is_16_1 {
                                    self.emit("  PHA".to_string());
                                    self.emit("  TXA".to_string());
                                    self.emit("  PHA".to_string());
                                } else {
                                    self.emit("  PHA".to_string());
                                }
                                let t2 = self.generate_expression(&args[1])?;
                                let is_16_2 = t2 == DataType::Word || t2 == DataType::Int;
                                let is_16 = is_16_1 || is_16_2;

                                if is_16 {
                                    if !is_16_2 {
                                        self.emit("  STA $00".to_string());
                                        self.emit("  LDA #0".to_string());
                                        self.emit("  STA $01".to_string());
                                    } else {
                                        self.emit("  STA $00".to_string());
                                        self.emit("  STX $01".to_string());
                                    }

                                    if !is_16_1 {
                                        self.emit("  PLA".to_string());
                                        self.emit("  STA $02".to_string());
                                        self.emit("  LDA #0".to_string());
                                        self.emit("  STA $03".to_string());
                                    } else {
                                        self.emit("  PLA".to_string());
                                        self.emit("  STA $03".to_string());
                                        self.emit("  PLA".to_string());
                                        self.emit("  STA $02".to_string());
                                    }

                                    // Arg1 in $02/$03, Arg2 in $00/$01
                                    self.emit("  SEC".to_string());
                                    self.emit("  LDA $02".to_string());
                                    self.emit("  SBC $00".to_string());
                                    self.emit("  LDA $03".to_string());
                                    self.emit("  SBC $01".to_string());

                                    let take_arg2_lbl = self.new_label();
                                    let done_lbl = self.new_label();
                                    if is_max {
                                        self.emit(format!("  BCC {}", take_arg2_lbl));
                                    // Arg1 < Arg2 -> take Arg2
                                    } else {
                                        self.emit(format!("  BCS {}", take_arg2_lbl));
                                        // Arg1 >= Arg2 -> take Arg2
                                    }
                                    // Ret Arg1
                                    self.emit("  LDA $02".to_string());
                                    self.emit("  LDX $03".to_string());
                                    self.emit(format!("  JMP {}", done_lbl));

                                    self.emit(format!("{}:", take_arg2_lbl));
                                    self.emit("  LDA $00".to_string());
                                    self.emit("  LDX $01".to_string());

                                    self.emit(format!("{}:", done_lbl));
                                    return Ok(if is_16_1 { t1 } else { t2 });
                                } else {
                                    // 8-bit
                                    self.emit("  STA $00".to_string()); // Arg2
                                    self.emit("  PLA".to_string()); // Arg1
                                    self.emit("  CMP $00".to_string());

                                    let take_arg2_lbl = self.new_label();
                                    let done_lbl = self.new_label();
                                    if is_max {
                                        self.emit(format!("  BCC {}", take_arg2_lbl));
                                    // Arg1 < Arg2 -> take Arg2
                                    } else {
                                        self.emit(format!("  BCS {}", take_arg2_lbl));
                                        // Arg1 >= Arg2 -> take Arg2
                                    }
                                    // Ret Arg1 in A (already in A? No, CMP preserved A!)
                                    self.emit(format!("  JMP {}", done_lbl));

                                    self.emit(format!("{}:", take_arg2_lbl));
                                    self.emit("  LDA $00".to_string());

                                    self.emit(format!("{}:", done_lbl));
                                    self.emit("  LDX #0".to_string());
                                    return Ok(DataType::Byte);
                                }
                            } else if member.eq_ignore_ascii_case("Sign") {
                                let dtype = self.generate_expression(&args[0])?;
                                let pos_lbl = self.new_label();
                                let done_lbl = self.new_label();
                                let neg_lbl = self.new_label();

                                if dtype == DataType::Word || dtype == DataType::Int {
                                    self.emit("  CPX #$80".to_string());
                                    self.emit(format!("  BCS {}", neg_lbl));
                                    self.emit("  CPX #0".to_string());
                                    self.emit(format!("  BNE {}", pos_lbl));
                                    self.emit("  CMP #0".to_string());
                                    self.emit(format!("  BNE {}", pos_lbl));
                                    self.emit("  LDA #0".to_string());
                                    self.emit("  LDX #0".to_string());
                                    self.emit(format!("  JMP {}", done_lbl));

                                    self.emit(format!("{}:", neg_lbl));
                                    self.emit("  LDA #$FF".to_string());
                                    self.emit("  LDX #$FF".to_string());
                                    self.emit(format!("  JMP {}", done_lbl));

                                    self.emit(format!("{}:", pos_lbl));
                                    self.emit("  LDA #1".to_string());
                                    self.emit("  LDX #0".to_string());

                                    self.emit(format!("{}:", done_lbl));
                                    return Ok(DataType::Int);
                                } else {
                                    self.emit("  CMP #$80".to_string());
                                    self.emit(format!("  BCS {}", neg_lbl));
                                    self.emit("  CMP #0".to_string());
                                    self.emit(format!("  BNE {}", pos_lbl));
                                    self.emit("  LDA #0".to_string());
                                    self.emit(format!("  JMP {}", done_lbl));

                                    self.emit(format!("{}:", neg_lbl));
                                    self.emit("  LDA #$FF".to_string());
                                    self.emit(format!("  JMP {}", done_lbl));

                                    self.emit(format!("{}:", pos_lbl));
                                    self.emit("  LDA #1".to_string());

                                    self.emit(format!("{}:", done_lbl));
                                    self.emit("  CMP #$80".to_string());
                                    let sign_ext_lbl = self.new_label();
                                    let sign_ext_done = self.new_label();
                                    self.emit(format!("  BCS {}", sign_ext_lbl));
                                    self.emit("  LDX #0".to_string());
                                    self.emit(format!("  JMP {}", sign_ext_done));
                                    self.emit(format!("{}:", sign_ext_lbl));
                                    self.emit("  LDX #$FF".to_string());
                                    self.emit(format!("{}:", sign_ext_done));

                                    return Ok(DataType::Int);
                                }
                            } else if member.eq_ignore_ascii_case("Clamp") {
                                // Math.Clamp(val, min, max) -> Max(min, Min(val, max))
                                // Evaluate max -> stack, min -> stack, val -> A/X
                                let t_val = self.generate_expression(&args[0])?;
                                let is_16_val = t_val == DataType::Word || t_val == DataType::Int;
                                if is_16_val {
                                    self.emit("  PHA".to_string());
                                    self.emit("  TXA".to_string());
                                    self.emit("  PHA".to_string());
                                } else {
                                    self.emit("  PHA".to_string());
                                }
                                let t_min = self.generate_expression(&args[1])?;
                                let is_16_min = t_min == DataType::Word || t_min == DataType::Int;
                                if is_16_min {
                                    self.emit("  PHA".to_string());
                                    self.emit("  TXA".to_string());
                                    self.emit("  PHA".to_string());
                                } else {
                                    self.emit("  PHA".to_string());
                                }
                                let t_max = self.generate_expression(&args[2])?;
                                let is_16_max = t_max == DataType::Word || t_max == DataType::Int;
                                let is_16 = is_16_val || is_16_min || is_16_max;
                                let is_signed = t_val == DataType::Int
                                    || t_min == DataType::Int
                                    || t_max == DataType::Int;

                                if is_16 {
                                    // Pop Max -> $04/$05
                                    if !is_16_max {
                                        self.emit("  STA $04".to_string());
                                        self.emit("  LDA #0".to_string());
                                        self.emit("  STA $05".to_string());
                                    } else {
                                        self.emit("  STA $04".to_string());
                                        self.emit("  STX $05".to_string());
                                    }
                                    // Pop Min -> $06/$07
                                    if !is_16_min {
                                        self.emit("  PLA".to_string());
                                        self.emit("  STA $06".to_string());
                                        self.emit("  LDA #0".to_string());
                                        self.emit("  STA $07".to_string());
                                    } else {
                                        self.emit("  PLA".to_string());
                                        self.emit("  STA $07".to_string());
                                        self.emit("  PLA".to_string());
                                        self.emit("  STA $06".to_string());
                                    }
                                    // Pop Val -> $02/$03
                                    if !is_16_val {
                                        self.emit("  PLA".to_string());
                                        self.emit("  STA $02".to_string());
                                        self.emit("  LDA #0".to_string());
                                        self.emit("  STA $03".to_string());
                                    } else {
                                        self.emit("  PLA".to_string());
                                        self.emit("  STA $03".to_string());
                                        self.emit("  PLA".to_string());
                                        self.emit("  STA $02".to_string());
                                    }

                                    // First: if Val > Max => Val = Max
                                    self.emit("  SEC".to_string());
                                    self.emit("  LDA $02".to_string());
                                    self.emit("  SBC $04".to_string());
                                    self.emit("  LDA $03".to_string());
                                    self.emit("  SBC $05".to_string());
                                    let no_max_clamp = self.new_label();
                                    if is_signed {
                                        // Signed order uses N xor V; carry alone would
                                        // treat a sign-extended negative INT as a large WORD.
                                        let corrected = self.new_label();
                                        self.emit(format!("  BVC {}", corrected));
                                        self.emit("  EOR #$80".to_string());
                                        self.emit(format!("{}:", corrected));
                                        self.emit(format!("  BMI {}", no_max_clamp));
                                    } else {
                                        self.emit(format!("  BCC {}", no_max_clamp));
                                    }
                                    self.emit("  LDA $04".to_string());
                                    self.emit("  STA $02".to_string());
                                    self.emit("  LDA $05".to_string());
                                    self.emit("  STA $03".to_string());
                                    self.emit(format!("{}:", no_max_clamp));

                                    // Second: if Val < Min => Val = Min
                                    self.emit("  SEC".to_string());
                                    self.emit("  LDA $02".to_string());
                                    self.emit("  SBC $06".to_string());
                                    self.emit("  LDA $03".to_string());
                                    self.emit("  SBC $07".to_string());
                                    let no_min_clamp = self.new_label();
                                    if is_signed {
                                        let corrected = self.new_label();
                                        self.emit(format!("  BVC {}", corrected));
                                        self.emit("  EOR #$80".to_string());
                                        self.emit(format!("{}:", corrected));
                                        self.emit(format!("  BPL {}", no_min_clamp));
                                    } else {
                                        self.emit(format!("  BCS {}", no_min_clamp));
                                    }
                                    self.emit("  LDA $06".to_string());
                                    self.emit("  STA $02".to_string());
                                    self.emit("  LDA $07".to_string());
                                    self.emit("  STA $03".to_string());
                                    self.emit(format!("{}:", no_min_clamp));

                                    self.emit("  LDA $02".to_string());
                                    self.emit("  LDX $03".to_string());
                                    return Ok(if is_16_val { t_val } else { DataType::Word });
                                } else {
                                    // 8-bit
                                    self.emit("  STA $01".to_string()); // Max
                                    self.emit("  PLA".to_string());
                                    self.emit("  STA $00".to_string()); // Min
                                    self.emit("  PLA".to_string()); // Val

                                    // If Val > Max => Val = Max
                                    self.emit("  CMP $01".to_string());
                                    let no_max_clamp = self.new_label();
                                    self.emit(format!("  BCC {}", no_max_clamp));
                                    self.emit(format!("  BEQ {}", no_max_clamp));
                                    self.emit("  LDA $01".to_string());
                                    self.emit(format!("{}:", no_max_clamp));

                                    // If Val < Min => Val = Min
                                    self.emit("  CMP $00".to_string());
                                    let no_min_clamp = self.new_label();
                                    self.emit(format!("  BCS {}", no_min_clamp));
                                    self.emit("  LDA $00".to_string());
                                    self.emit(format!("{}:", no_min_clamp));

                                    self.emit("  LDX #0".to_string());
                                    return Ok(DataType::Byte);
                                }
                            } else if member.eq_ignore_ascii_case("Wrap") {
                                let t_val = self.generate_expression(&args[0])?;
                                let is_16_val = t_val == DataType::Word || t_val == DataType::Int;
                                if is_16_val {
                                    self.emit("  PHA".to_string());
                                    self.emit("  TXA".to_string());
                                    self.emit("  PHA".to_string());
                                } else {
                                    self.emit("  PHA".to_string());
                                }
                                let t_min = self.generate_expression(&args[1])?;
                                let is_16_min = t_min == DataType::Word || t_min == DataType::Int;
                                if is_16_min {
                                    self.emit("  PHA".to_string());
                                    self.emit("  TXA".to_string());
                                    self.emit("  PHA".to_string());
                                } else {
                                    self.emit("  PHA".to_string());
                                }
                                let t_max = self.generate_expression(&args[2])?;
                                let is_16_max = t_max == DataType::Word || t_max == DataType::Int;
                                let is_16 = is_16_val || is_16_min || is_16_max;

                                let wrap_done = self.new_label();
                                let wrap_check_max = self.new_label();

                                if is_16 {
                                    if !is_16_max {
                                        self.emit("  STA $04".to_string());
                                        self.emit("  LDA #0".to_string());
                                        self.emit("  STA $05".to_string());
                                    } else {
                                        self.emit("  STA $04".to_string());
                                        self.emit("  STX $05".to_string());
                                    }
                                    if !is_16_min {
                                        self.emit("  PLA".to_string());
                                        self.emit("  STA $06".to_string());
                                        self.emit("  LDA #0".to_string());
                                        self.emit("  STA $07".to_string());
                                    } else {
                                        self.emit("  PLA".to_string());
                                        self.emit("  STA $07".to_string());
                                        self.emit("  PLA".to_string());
                                        self.emit("  STA $06".to_string());
                                    }
                                    if !is_16_val {
                                        self.emit("  PLA".to_string());
                                        self.emit("  STA $02".to_string());
                                        self.emit("  LDA #0".to_string());
                                        self.emit("  STA $03".to_string());
                                    } else {
                                        self.emit("  PLA".to_string());
                                        self.emit("  STA $03".to_string());
                                        self.emit("  PLA".to_string());
                                        self.emit("  STA $02".to_string());
                                    }

                                    self.emit("  SEC".to_string());
                                    self.emit("  LDA $04".to_string());
                                    self.emit("  SBC $06".to_string());
                                    self.emit("  STA $08".to_string());
                                    self.emit("  LDA $05".to_string());
                                    self.emit("  SBC $07".to_string());
                                    self.emit("  STA $09".to_string());

                                    self.emit("  LDA $08".to_string());
                                    self.emit("  ORA $09".to_string());
                                    self.emit(format!("  BEQ {}", wrap_done));

                                    self.emit("  SEC".to_string());
                                    self.emit("  LDA $02".to_string());
                                    self.emit("  SBC $06".to_string());
                                    self.emit("  LDA $03".to_string());
                                    self.emit("  SBC $07".to_string());
                                    self.emit(format!("  BCS {}", wrap_check_max));

                                    self.emit("  CLC".to_string());
                                    self.emit("  LDA $02".to_string());
                                    self.emit("  ADC $08".to_string());
                                    self.emit("  STA $02".to_string());
                                    self.emit("  LDA $03".to_string());
                                    self.emit("  ADC $09".to_string());
                                    self.emit("  STA $03".to_string());
                                    self.emit(format!("  JMP {}", wrap_done));

                                    self.emit(format!("{}:", wrap_check_max));
                                    self.emit("  SEC".to_string());
                                    self.emit("  LDA $02".to_string());
                                    self.emit("  SBC $04".to_string());
                                    self.emit("  LDA $03".to_string());
                                    self.emit("  SBC $05".to_string());
                                    self.emit(format!("  BCC {}", wrap_done));
                                    self.emit(format!("  BEQ {}", wrap_done));

                                    self.emit("  SEC".to_string());
                                    self.emit("  LDA $02".to_string());
                                    self.emit("  SBC $08".to_string());
                                    self.emit("  STA $02".to_string());
                                    self.emit("  LDA $03".to_string());
                                    self.emit("  SBC $09".to_string());
                                    self.emit("  STA $03".to_string());

                                    self.emit(format!("{}:", wrap_done));
                                    self.emit("  LDA $02".to_string());
                                    self.emit("  LDX $03".to_string());
                                    return Ok(if is_16_val { t_val } else { DataType::Word });
                                } else {
                                    self.emit("  STA $01".to_string()); // Max
                                    self.emit("  PLA".to_string());
                                    self.emit("  STA $00".to_string()); // Min
                                    self.emit("  PLA".to_string()); // Val ($02)
                                    self.emit("  STA $02".to_string());

                                    self.emit("  SEC".to_string());
                                    self.emit("  LDA $01".to_string());
                                    self.emit("  SBC $00".to_string());
                                    self.emit("  STA $03".to_string());
                                    self.emit(format!("  BEQ {}", wrap_done));

                                    self.emit("  LDA $02".to_string());
                                    self.emit("  CMP $00".to_string());
                                    self.emit(format!("  BCS {}", wrap_check_max));
                                    self.emit("  CLC".to_string());
                                    self.emit("  ADC $03".to_string());
                                    self.emit("  STA $02".to_string());
                                    self.emit(format!("  JMP {}", wrap_done));

                                    self.emit(format!("{}:", wrap_check_max));
                                    self.emit("  CMP $01".to_string());
                                    self.emit(format!("  BCC {}", wrap_done));
                                    self.emit(format!("  BEQ {}", wrap_done));
                                    self.emit("  SEC".to_string());
                                    self.emit("  SBC $03".to_string());
                                    self.emit("  STA $02".to_string());

                                    self.emit(format!("{}:", wrap_done));
                                    self.emit("  LDA $02".to_string());
                                    self.emit("  LDX #0".to_string());
                                    return Ok(DataType::Byte);
                                }
                            } else if member.eq_ignore_ascii_case("Lerp") {
                                let t_a = self.generate_expression(&args[0])?;
                                let is_16_a = t_a == DataType::Word || t_a == DataType::Int;
                                if is_16_a {
                                    self.emit("  PHA".to_string());
                                    self.emit("  TXA".to_string());
                                    self.emit("  PHA".to_string());
                                } else {
                                    self.emit("  PHA".to_string());
                                }
                                let t_b = self.generate_expression(&args[1])?;
                                let is_16_b = t_b == DataType::Word || t_b == DataType::Int;
                                if is_16_b {
                                    self.emit("  PHA".to_string());
                                    self.emit("  TXA".to_string());
                                    self.emit("  PHA".to_string());
                                } else {
                                    self.emit("  PHA".to_string());
                                }
                                let _ = self.generate_expression(&args[2])?;

                                self.emit("  STA $00".to_string());
                                self.emit("  LDA #0".to_string());
                                self.emit("  STA $01".to_string());

                                if !is_16_b {
                                    self.emit("  PLA".to_string());
                                    self.emit("  STA $04".to_string());
                                    self.emit("  LDA #0".to_string());
                                    self.emit("  STA $05".to_string());
                                } else {
                                    self.emit("  PLA".to_string());
                                    self.emit("  STA $05".to_string());
                                    self.emit("  PLA".to_string());
                                    self.emit("  STA $04".to_string());
                                }

                                if !is_16_a {
                                    self.emit("  PLA".to_string());
                                    self.emit("  STA $02".to_string());
                                    self.emit("  LDA #0".to_string());
                                    self.emit("  STA $03".to_string());
                                } else {
                                    self.emit("  PLA".to_string());
                                    self.emit("  STA $03".to_string());
                                    self.emit("  PLA".to_string());
                                    self.emit("  STA $02".to_string());
                                }

                                self.emit("  SEC".to_string());
                                self.emit("  LDA $04".to_string());
                                self.emit("  SBC $02".to_string());
                                self.emit("  PHA".to_string());
                                self.emit("  LDA $05".to_string());
                                self.emit("  SBC $03".to_string());
                                self.emit("  TAX".to_string());
                                self.emit("  PLA".to_string());

                                self.emit("  JSR Math_Mul16".to_string());

                                // Add the base value to the 16-bit product.
                                // Preserve each product byte while adding low first
                                // so the carry reaches the high-byte addition.
                                self.emit("  PHA".to_string()); // Product low
                                self.emit("  TXA".to_string()); // Product high
                                self.emit("  STA $09".to_string());
                                self.emit("  PLA".to_string());
                                self.emit("  CLC".to_string());
                                self.emit("  ADC $02".to_string());
                                self.emit("  STA $08".to_string());
                                self.emit("  LDA $09".to_string());
                                self.emit("  ADC $03".to_string());
                                self.emit("  TAX".to_string());
                                self.emit("  LDA $08".to_string());

                                return Ok(if is_16_a || is_16_b {
                                    t_a
                                } else {
                                    DataType::Byte
                                });
                            }
                        }
                        if base_name.eq_ignore_ascii_case("Controller") {
                            if member.eq_ignore_ascii_case("AnyPressed") {
                                self.emit("  LDA $12".to_string()); // Newly pressed
                                self.emit("  CMP #0".to_string());
                                let true_lbl = self.new_label();
                                let end_lbl = self.new_label();
                                self.emit(format!("  BNE {}", true_lbl));
                                self.emit("  LDA #0".to_string());
                                self.emit(format!("  JMP {}", end_lbl));
                                self.emit(format!("{}:", true_lbl));
                                self.emit("  LDA #$FF".to_string());
                                self.emit(format!("{}:", end_lbl));
                                self.emit("  LDX #0".to_string());
                                return Ok(DataType::Bool);
                            } else {
                                self.generate_expression(&args[0])?;
                                self.emit("  STA $00".to_string());
                                if member.eq_ignore_ascii_case("IsPressed") {
                                    self.emit("  LDA $12".to_string());
                                } else if member.eq_ignore_ascii_case("IsHeld") {
                                    self.emit("  LDA $10".to_string());
                                } else if member.eq_ignore_ascii_case("IsReleased") {
                                    self.emit("  LDA $13".to_string());
                                }
                                self.emit("  AND $00".to_string());
                                self.emit("  CMP #0".to_string());
                                let true_lbl = self.new_label();
                                let end_lbl = self.new_label();
                                self.emit(format!("  BNE {}", true_lbl));
                                self.emit("  LDA #0".to_string());
                                self.emit(format!("  JMP {}", end_lbl));
                                self.emit(format!("{}:", true_lbl));
                                self.emit("  LDA #$FF".to_string());
                                self.emit(format!("{}:", end_lbl));
                                self.emit("  LDX #0".to_string());
                                return Ok(DataType::Bool);
                            }
                        }
                    }
                }

                // Built-in Functions
                if let Expression::Identifier(name) = &**callee {
                    if name.eq_ignore_ascii_case("LEN") {
                        // Generate Arg (String) -> A/X (Address)
                        self.generate_expression(&args[0])?;
                        self.emit("  JSR Runtime_StringLen".to_string());
                        return Ok(DataType::Word);
                    } else if name.eq_ignore_ascii_case("ABS") {
                        let dtype = self.generate_expression(&args[0])?;
                        // ABS of A (Byte) or A/X (Word/Int)
                        if dtype == DataType::Word || dtype == DataType::Int {
                            // Check High Byte (X) for sign
                            self.emit("  CPX #$80".to_string());
                            let pos_lbl = self.new_label();
                            self.emit(format!("  BCC {}", pos_lbl));
                            // Negate
                            self.emit("  EOR #$FF".to_string()); // Low
                            self.emit("  CLC".to_string());
                            self.emit("  ADC #1".to_string());
                            self.emit("  PHA".to_string());
                            self.emit("  TXA".to_string());
                            self.emit("  EOR #$FF".to_string()); // High
                            self.emit("  ADC #0".to_string());
                            self.emit("  TAX".to_string());
                            self.emit("  PLA".to_string());
                            self.emit(format!("{}:", pos_lbl));
                            return Ok(dtype); // Returns same type (unsigned value)
                        } else {
                            // Byte. Treat as signed? "INT" is signed 8-bit.
                            // If arg is Byte (unsigned 0-255), ABS is itself.
                            // If arg is INT (-128 to 127), ABS needs negate.
                            // Assuming INT logic if 8-bit.
                            // Check bit 7
                            self.emit("  CMP #$80".to_string());
                            let pos_lbl = self.new_label();
                            self.emit(format!("  BCC {}", pos_lbl));
                            // Negate
                            self.emit("  EOR #$FF".to_string());
                            self.emit("  CLC".to_string());
                            self.emit("  ADC #1".to_string());
                            self.emit(format!("{}:", pos_lbl));
                            return Ok(dtype);
                        }
                    } else if name.eq_ignore_ascii_case("SGN") {
                        let dtype = self.generate_expression(&args[0])?;
                        let pos_lbl = self.new_label();
                        let done_lbl = self.new_label();
                        let neg_lbl = self.new_label();

                        if dtype == DataType::Word || dtype == DataType::Int {
                            // Check High (X)
                            self.emit("  CPX #$80".to_string());
                            self.emit(format!("  BCS {}", neg_lbl)); // >= $8000 (Negative)
                                                                     // Positive or Zero. Check if Zero.
                            self.emit("  CPX #0".to_string());
                            self.emit(format!("  BNE {}", pos_lbl));
                            self.emit("  CMP #0".to_string());
                            self.emit(format!("  BNE {}", pos_lbl));
                            // Zero
                            self.emit("  LDA #0".to_string());
                            self.emit("  LDX #0".to_string());
                            self.emit(format!("  JMP {}", done_lbl));

                            self.emit(format!("{}:", neg_lbl));
                            self.emit("  LDA #$FF".to_string()); // -1
                            self.emit("  LDX #$FF".to_string());
                            self.emit(format!("  JMP {}", done_lbl));

                            self.emit(format!("{}:", pos_lbl));
                            self.emit("  LDA #1".to_string());
                            self.emit("  LDX #0".to_string());

                            self.emit(format!("{}:", done_lbl));
                            return Ok(DataType::Int);
                        } else {
                            // Byte/Int
                            self.emit("  CMP #$80".to_string());
                            self.emit(format!("  BCS {}", neg_lbl));
                            self.emit("  CMP #0".to_string());
                            self.emit(format!("  BNE {}", pos_lbl));
                            // Zero
                            self.emit("  LDA #0".to_string());
                            self.emit(format!("  JMP {}", done_lbl));

                            self.emit(format!("{}:", neg_lbl));
                            self.emit("  LDA #$FF".to_string()); // -1
                            self.emit(format!("  JMP {}", done_lbl));

                            self.emit(format!("{}:", pos_lbl));
                            self.emit("  LDA #1".to_string());

                            self.emit(format!("{}:", done_lbl));
                            // Sign extend result to 16-bit
                            self.emit("  CMP #$80".to_string()); // Check sign of result
                            let sign_ext_lbl = self.new_label();
                            let sign_ext_done = self.new_label();
                            self.emit(format!("  BCS {}", sign_ext_lbl));
                            self.emit("  LDX #0".to_string());
                            self.emit(format!("  JMP {}", sign_ext_done));
                            self.emit(format!("{}:", sign_ext_lbl));
                            self.emit("  LDX #$FF".to_string());
                            self.emit(format!("{}:", sign_ext_done));

                            return Ok(DataType::Int);
                        }
                    } else if name.eq_ignore_ascii_case("ASC") {
                        self.generate_expression(&args[0])?;
                        self.emit("  JSR Runtime_Asc".to_string());
                        return Ok(DataType::Word);
                    } else if name.eq_ignore_ascii_case("VAL") {
                        self.generate_expression(&args[0])?;
                        self.emit("  JSR Runtime_Val".to_string());
                        return Ok(DataType::Word);
                    } else if name.eq_ignore_ascii_case("CHR") {
                        self.generate_expression(&args[0])?;
                        self.emit("  JSR Runtime_Chr".to_string());
                        return Ok(DataType::String);
                    } else if name.eq_ignore_ascii_case("STR") {
                        let dtype = self.generate_expression(&args[0])?;
                        self.emit(format!("  LDY #{}", u8::from(dtype == DataType::Int)));
                        self.emit("  JSR Runtime_Str".to_string());
                        return Ok(DataType::String);
                    } else if name.eq_ignore_ascii_case("LEFT") {
                        // Arg 1: Count (Stack)
                        self.generate_expression(&args[1])?;
                        self.emit("  PHA".to_string());
                        // Arg 0: String (A/X)
                        self.generate_expression(&args[0])?;
                        self.emit("  JSR Runtime_Left".to_string());
                        return Ok(DataType::String);
                    } else if name.eq_ignore_ascii_case("RIGHT") {
                        // Arg 1: Count (Stack)
                        self.generate_expression(&args[1])?;
                        self.emit("  PHA".to_string());
                        // Arg 0: String (A/X)
                        self.generate_expression(&args[0])?;
                        self.emit("  JSR Runtime_Right".to_string());
                        return Ok(DataType::String);
                    } else if name.eq_ignore_ascii_case("MID") {
                        // Arg 2: Len (Stack)
                        self.generate_expression(&args[2])?;
                        self.emit("  PHA".to_string());
                        // Arg 1: Start (Stack)
                        self.generate_expression(&args[1])?;
                        self.emit("  PHA".to_string());
                        // Arg 0: String (A/X)
                        self.generate_expression(&args[0])?;
                        self.emit("  JSR Runtime_Mid".to_string());
                        return Ok(DataType::String);
                    } else if name.eq_ignore_ascii_case("RND") {
                        // Arg 0: Max (Stack or Temp)
                        self.generate_expression(&args[0])?;
                        // Save Max to Stack
                        self.emit("  PHA".to_string()); // Low
                        self.emit("  TXA".to_string());
                        self.emit("  PHA".to_string()); // High

                        // Get Random
                        self.emit("  JSR Runtime_Random".to_string());

                        // Setup Div16: Random (A/X) / Max ($00/$01)
                        // Pop Max to $00/$01
                        self.emit("  STX $07".to_string()); // Save Random High
                        self.emit("  STA $06".to_string()); // Save Random Low

                        self.emit("  PLA".to_string()); // Max High
                        self.emit("  STA $01".to_string());
                        self.emit("  PLA".to_string()); // Max Low
                        self.emit("  STA $00".to_string());

                        self.emit("  LDA $06".to_string()); // Restore Random
                        self.emit("  LDX $07".to_string());

                        self.emit("  JSR Math_Div16".to_string());
                        // Remainder is in $08/$09
                        self.emit("  LDA $08".to_string());
                        self.emit("  LDX $09".to_string());
                        return Ok(DataType::Word);
                    } else if name.eq_ignore_ascii_case("BITAND")
                        || name.eq_ignore_ascii_case("BITOR")
                        || name.eq_ignore_ascii_case("BITXOR")
                    {
                        let t1 = self.generate_expression(&args[0])?;
                        let is_16_1 = t1 == DataType::Word || t1 == DataType::Int;
                        if is_16_1 {
                            self.emit("  PHA".to_string());
                            self.emit("  TXA".to_string());
                            self.emit("  PHA".to_string());
                        } else {
                            self.emit("  PHA".to_string());
                        }
                        let t2 = self.generate_expression(&args[1])?;
                        let is_16_2 = t2 == DataType::Word || t2 == DataType::Int;
                        let is_16 = is_16_1 || is_16_2;

                        if is_16 {
                            if !is_16_2 {
                                self.emit("  STA $00".to_string());
                                self.emit("  LDA #0".to_string());
                                self.emit("  STA $01".to_string());
                            } else {
                                self.emit("  STA $00".to_string());
                                self.emit("  STX $01".to_string());
                            }

                            if !is_16_1 {
                                self.emit("  PLA".to_string());
                                self.emit("  LDX #0".to_string());
                            } else {
                                self.emit("  PLA".to_string());
                                self.emit("  TAX".to_string());
                                self.emit("  PLA".to_string());
                            }

                            if name.eq_ignore_ascii_case("BITAND") {
                                self.emit("  AND $00".to_string());
                                self.emit("  PHA".to_string());
                                self.emit("  TXA".to_string());
                                self.emit("  AND $01".to_string());
                                self.emit("  TAX".to_string());
                                self.emit("  PLA".to_string());
                            } else if name.eq_ignore_ascii_case("BITOR") {
                                self.emit("  ORA $00".to_string());
                                self.emit("  PHA".to_string());
                                self.emit("  TXA".to_string());
                                self.emit("  ORA $01".to_string());
                                self.emit("  TAX".to_string());
                                self.emit("  PLA".to_string());
                            } else {
                                self.emit("  EOR $00".to_string());
                                self.emit("  PHA".to_string());
                                self.emit("  TXA".to_string());
                                self.emit("  EOR $01".to_string());
                                self.emit("  TAX".to_string());
                                self.emit("  PLA".to_string());
                            }
                            return Ok(DataType::Word);
                        } else {
                            self.emit("  STA $00".to_string());
                            self.emit("  PLA".to_string());
                            if name.eq_ignore_ascii_case("BITAND") {
                                self.emit("  AND $00".to_string());
                            } else if name.eq_ignore_ascii_case("BITOR") {
                                self.emit("  ORA $00".to_string());
                            } else {
                                self.emit("  EOR $00".to_string());
                            }
                            self.emit("  LDX #0".to_string());
                            return Ok(DataType::Byte);
                        }
                    } else if name.eq_ignore_ascii_case("BITNOT") {
                        let t = self.generate_expression(&args[0])?;
                        if t == DataType::Word || t == DataType::Int {
                            self.emit("  EOR #$FF".to_string());
                            self.emit("  PHA".to_string());
                            self.emit("  TXA".to_string());
                            self.emit("  EOR #$FF".to_string());
                            self.emit("  TAX".to_string());
                            self.emit("  PLA".to_string());
                            return Ok(DataType::Word);
                        } else {
                            self.emit("  EOR #$FF".to_string());
                            self.emit("  LDX #0".to_string());
                            return Ok(DataType::Byte);
                        }
                    } else if name.eq_ignore_ascii_case("BITSHL")
                        || name.eq_ignore_ascii_case("BITSHR")
                    {
                        let is_shl = name.eq_ignore_ascii_case("BITSHL");
                        let t1 = self.generate_expression(&args[0])?;
                        let is_16 = t1 == DataType::Word || t1 == DataType::Int;
                        if is_16 {
                            self.emit("  PHA".to_string());
                            self.emit("  TXA".to_string());
                            self.emit("  PHA".to_string());
                        } else {
                            self.emit("  PHA".to_string());
                        }
                        let _ = self.generate_expression(&args[1])?;
                        // Keep runtime shifts consistent with constant folding:
                        // counts wrap at 32 instead of executing an unnecessary
                        // full byte-count loop when the low five bits are zero.
                        self.emit("  AND #$1F".to_string());
                        self.emit("  STA $00".to_string());
                        if is_16 {
                            self.emit("  PLA".to_string());
                            self.emit("  TAX".to_string());
                            self.emit("  PLA".to_string());
                            if is_shl {
                                self.emit("  JSR Math_Shl16".to_string());
                            } else {
                                self.emit("  JSR Math_Shr16".to_string());
                            }
                            return Ok(DataType::Word);
                        } else {
                            self.emit("  PLA".to_string());
                            if is_shl {
                                self.emit("  JSR Math_Shl8".to_string());
                            } else {
                                self.emit("  JSR Math_Shr8".to_string());
                            }
                            self.emit("  LDX #0".to_string());
                            return Ok(DataType::Byte);
                        }
                    }
                }

                // Determine if Array or Sub
                if let Some(DataType::Array(_, _)) = self.resolve_type(callee) {
                    // Array Access
                    // 1. Calc Address in $02/$03
                    self.generate_array_address(callee, &args[0])?;

                    let rtype = self
                        .resolve_type(expr)
                        .ok_or("Cannot resolve array element type")?;
                    self.emit("  LDY #0".to_string());
                    match rtype {
                        DataType::Byte | DataType::Bool | DataType::Enum(_) => {
                            self.emit("  LDA ($02),Y".to_string());
                            self.emit("  LDX #0".to_string());
                        }
                        DataType::Int => {
                            self.emit("  LDA ($02),Y".to_string());
                            self.emit("  LDX #0".to_string());
                            self.emit("  CMP #$80".to_string());
                            let positive = self.new_label();
                            self.emit(format!("  BCC {}", positive));
                            self.emit("  LDX #$FF".to_string());
                            self.emit(format!("{}:", positive));
                        }
                        DataType::Word | DataType::String => {
                            self.emit("  LDA ($02),Y".to_string());
                            self.emit("  PHA".to_string()); // Low
                            self.emit("  INY".to_string());
                            self.emit("  LDA ($02),Y".to_string());
                            self.emit("  TAX".to_string());
                            self.emit("  PLA".to_string());
                        }
                        _ => {}
                    }
                    Ok(rtype)
                } else {
                    // Function Call
                    if let Expression::Identifier(name) = &**callee {
                        let (target_bank, params) =
                            if let Some((b, p)) = self.sub_signatures.get(name) {
                                (*b, p.clone())
                            } else {
                                return Err(format!("Undefined function '{}'", name));
                            };
                        if args.len() != params.len() {
                            return Err("Arg mismatch".to_string());
                        }
                        self.generate_args(&params, args)?;
                        if target_bank != self.current_bank && target_bank != 7 {
                            self.emit(format!("  JSR Trampoline_{}", name));
                        } else {
                            self.emit(format!("  JSR {}", name));
                        }
                        Ok(DataType::Word)
                    } else {
                        Err("Indirect function call not supported".to_string())
                    }
                }
            }
            Expression::Integer(val) => {
                let low = (val & 0xFF) as u8;
                let high = ((val >> 8) & 0xFF) as u8;
                self.emit(format!("  LDA #${:02X}", low));
                self.emit(format!("  LDX #${:02X}", high));
                Ok(DataType::Word)
            }
            Expression::StringLiteral(s) => {
                let label = self.string_literals.get(s).cloned();
                if let Some(label) = label {
                    let addr = self.data_table_offsets.get(&label).cloned();
                    if let Some(addr) = addr {
                        self.emit(format!("  LDA ${:04X}", addr));
                        self.emit(format!("  LDX ${:04X}", addr + 1));
                        Ok(DataType::String)
                    } else {
                        Err("No table entry".to_string())
                    }
                } else {
                    Err("String not alloc".to_string())
                }
            }
            Expression::Identifier(name) => {
                let (sym_addr, sym_type, sym_kind, sym_val) =
                    if let Some(sym) = self.symbol_table.resolve(name) {
                        (
                            sym.address,
                            sym.data_type.clone(),
                            sym.kind.clone(),
                            sym.value,
                        )
                    } else {
                        return Err(format!("Undefined symbol '{}'", name));
                    };

                if let Some(addr) = sym_addr {
                    match sym_type {
                        DataType::Word | DataType::String => {
                            self.emit(format!("  LDA ${:04X}", addr));
                            self.emit(format!("  LDX ${:04X}", addr + 1));
                            Ok(sym_type)
                        }
                        DataType::Int => {
                            self.emit(format!("  LDA ${:04X}", addr));
                            self.emit("  CMP #$80".to_string());
                            let pos_lbl = self.new_label();
                            let done_lbl = self.new_label();
                            self.emit(format!("  BCC {}", pos_lbl));
                            self.emit("  LDX #$FF".to_string());
                            self.emit(format!("  JMP {}", done_lbl));
                            self.emit(format!("{}:", pos_lbl));
                            self.emit("  LDX #0".to_string());
                            self.emit(format!("{}:", done_lbl));
                            Ok(DataType::Int)
                        }
                        _ => {
                            self.emit(format!("  LDA ${:04X}", addr));
                            self.emit("  LDX #0".to_string());
                            Ok(DataType::Byte)
                        }
                    }
                } else if sym_kind == SymbolKind::Constant {
                    if let Some(val) = sym_val {
                        let low = (val & 0xFF) as u8;
                        let high = ((val >> 8) & 0xFF) as u8;
                        self.emit(format!("  LDA #${:02X}", low));
                        self.emit(format!("  LDX #${:02X}", high));
                        Ok(DataType::Word)
                    } else {
                        Err("Const no val".to_string())
                    }
                } else if sym_kind == SymbolKind::Metasprite {
                    let addr = self.data_table_offsets.get(name).cloned();
                    if let Some(addr) = addr {
                        self.emit(format!("  LDA ${:04X}", addr));
                        self.emit(format!("  LDX ${:04X}", addr + 1));
                        Ok(DataType::Word)
                    } else {
                        Err("Metasprite table entry missing".to_string())
                    }
                } else if sym_kind == SymbolKind::Animation {
                    let addr = self.data_table_offsets.get(name).cloned();
                    if let Some(addr) = addr {
                        self.emit(format!("  LDA ${:04X}", addr));
                        self.emit(format!("  LDX ${:04X}", addr + 1));
                        Ok(DataType::Word)
                    } else {
                        Err("Animation table entry missing".to_string())
                    }
                } else {
                    Err("No addr".to_string())
                }
            }
            Expression::BinaryOp(l, op, r) => {
                let tl = self.generate_expression(l)?;
                if tl == DataType::Byte || tl == DataType::Bool {
                    self.emit("  PHA".to_string());
                } else {
                    self.emit("  PHA".to_string());
                    self.emit("  TXA".to_string());
                    self.emit("  PHA".to_string());
                }
                let tr = self.generate_expression(r)?;

                let is_16 = tl == DataType::Word
                    || tr == DataType::Word
                    || tl == DataType::Int
                    || tr == DataType::Int;
                let is_signed = tl == DataType::Int || tr == DataType::Int;

                if tl == DataType::String {
                    match op {
                        BinaryOperator::Add => {
                            // LHS on stack, RHS in A/X
                            self.emit("  JSR Runtime_StringConcat".to_string());
                            Ok(DataType::String)
                        }
                        BinaryOperator::Equal | BinaryOperator::NotEqual => {
                            // LHS on stack, RHS in A/X
                            self.emit("  JSR Runtime_StringCompare".to_string());
                            if matches!(op, BinaryOperator::NotEqual) {
                                self.emit("  EOR #$FF".to_string()); // Invert boolean
                            }
                            Ok(DataType::Bool)
                        }
                        _ => Err("Unsupported operator for String".to_string()),
                    }
                } else if is_16 {
                    if tr == DataType::Byte || tr == DataType::Bool {
                        self.emit("  STA $00".to_string());
                        self.emit("  LDA #0".to_string());
                        self.emit("  STA $01".to_string());
                    } else {
                        self.emit("  STA $00".to_string());
                        self.emit("  STX $01".to_string());
                    }

                    if tl == DataType::Byte || tl == DataType::Bool {
                        self.emit("  PLA".to_string());
                        self.emit("  LDX #0".to_string());
                    } else {
                        self.emit("  PLA".to_string());
                        self.emit("  TAX".to_string());
                        self.emit("  PLA".to_string());
                    }

                    match op {
                        BinaryOperator::Add => {
                            self.emit("  CLC".to_string());
                            self.emit("  ADC $00".to_string());
                            self.emit("  PHA".to_string());
                            self.emit("  TXA".to_string());
                            self.emit("  ADC $01".to_string());
                            self.emit("  TAX".to_string());
                            self.emit("  PLA".to_string());
                            if is_signed {
                                Ok(DataType::Int)
                            } else {
                                Ok(DataType::Word)
                            }
                        }
                        BinaryOperator::Xor => {
                            self.emit("  EOR $00".to_string());
                            self.emit("  PHA".to_string());
                            self.emit("  TXA".to_string());
                            self.emit("  EOR $01".to_string());
                            self.emit("  TAX".to_string());
                            self.emit("  PLA".to_string());
                            if is_signed {
                                Ok(DataType::Int)
                            } else {
                                Ok(DataType::Word)
                            }
                        }
                        BinaryOperator::Subtract => {
                            self.emit("  SEC".to_string());
                            self.emit("  SBC $00".to_string());
                            self.emit("  PHA".to_string());
                            self.emit("  TXA".to_string());
                            self.emit("  SBC $01".to_string());
                            self.emit("  TAX".to_string());
                            self.emit("  PLA".to_string());
                            if is_signed {
                                Ok(DataType::Int)
                            } else {
                                Ok(DataType::Word)
                            }
                        }
                        BinaryOperator::Multiply => {
                            self.emit("  JSR Math_Mul16".to_string());
                            if is_signed {
                                Ok(DataType::Int)
                            } else {
                                Ok(DataType::Word)
                            }
                        }
                        BinaryOperator::Divide => {
                            if is_signed {
                                self.emit("  JSR Math_Div16_Signed".to_string());
                                Ok(DataType::Int)
                            } else {
                                self.emit("  JSR Math_Div16".to_string());
                                Ok(DataType::Word)
                            }
                        }
                        BinaryOperator::Modulo => {
                            if is_signed {
                                self.emit("  JSR Math_Div16_Signed".to_string());
                                self.emit("  LDA $08".to_string());
                                self.emit("  LDX $09".to_string());
                                Ok(DataType::Int)
                            } else {
                                self.emit("  JSR Math_Div16".to_string());
                                self.emit("  LDA $08".to_string());
                                self.emit("  LDX $09".to_string());
                                Ok(DataType::Word)
                            }
                        }
                        BinaryOperator::And => {
                            self.emit("  AND $00".to_string());
                            self.emit("  PHA".to_string());
                            self.emit("  TXA".to_string());
                            self.emit("  AND $01".to_string());
                            self.emit("  TAX".to_string());
                            self.emit("  PLA".to_string());
                            if is_signed {
                                Ok(DataType::Int)
                            } else {
                                Ok(DataType::Word)
                            }
                        }
                        BinaryOperator::Or => {
                            self.emit("  ORA $00".to_string());
                            self.emit("  PHA".to_string());
                            self.emit("  TXA".to_string());
                            self.emit("  ORA $01".to_string());
                            self.emit("  TAX".to_string());
                            self.emit("  PLA".to_string());
                            if is_signed {
                                Ok(DataType::Int)
                            } else {
                                Ok(DataType::Word)
                            }
                        }
                        BinaryOperator::ShiftLeft => {
                            self.emit("  JSR Math_Shl16".to_string());
                            if is_signed {
                                Ok(DataType::Int)
                            } else {
                                Ok(DataType::Word)
                            }
                        }
                        BinaryOperator::ShiftRight => {
                            self.emit("  JSR Math_Shr16".to_string());
                            if is_signed {
                                Ok(DataType::Int)
                            } else {
                                Ok(DataType::Word)
                            }
                        }
                        _ => {
                            let true_lbl = self.new_label();
                            let false_lbl = self.new_label();
                            let end_lbl = self.new_label();

                            match op {
                                BinaryOperator::Equal | BinaryOperator::NotEqual => {
                                    self.emit("  CPX $01".to_string());
                                    self.emit(format!("  BNE {}", false_lbl));
                                    self.emit("  CMP $00".to_string());
                                    if matches!(op, BinaryOperator::Equal) {
                                        self.emit(format!("  BNE {}", false_lbl));
                                    } else {
                                        self.emit(format!("  BEQ {}", false_lbl));
                                    }
                                    self.emit(format!("  JMP {}", true_lbl));
                                }
                                BinaryOperator::LessThan
                                | BinaryOperator::GreaterThan
                                | BinaryOperator::LessThanOrEqual
                                | BinaryOperator::GreaterThanOrEqual => {
                                    self.emit("  SEC".to_string());
                                    self.emit("  SBC $00".to_string());
                                    self.emit("  PHA".to_string());
                                    self.emit("  TXA".to_string());
                                    self.emit("  SBC $01".to_string());
                                    self.emit("  TAX".to_string());
                                    self.emit("  PLA".to_string());

                                    if is_signed {
                                        let overflow_lbl = self.new_label();
                                        match op {
                                            BinaryOperator::LessThan => {
                                                self.emit(format!("  BVS {}", overflow_lbl));
                                                self.emit(format!("  BMI {}", true_lbl));
                                                self.emit(format!("  JMP {}", false_lbl));
                                                self.emit(format!("{}:", overflow_lbl));
                                                self.emit(format!("  BPL {}", true_lbl));
                                                self.emit(format!("  JMP {}", false_lbl));
                                            }
                                            BinaryOperator::GreaterThanOrEqual => {
                                                self.emit(format!("  BVS {}", overflow_lbl));
                                                self.emit(format!("  BPL {}", true_lbl));
                                                self.emit(format!("  JMP {}", false_lbl));
                                                self.emit(format!("{}:", overflow_lbl));
                                                self.emit(format!("  BMI {}", true_lbl));
                                                self.emit(format!("  JMP {}", false_lbl));
                                            }
                                            BinaryOperator::GreaterThan => {
                                                self.emit(format!("  BVS {}", overflow_lbl));
                                                self.emit(format!("  BMI {}", false_lbl));
                                                self.emit("  STX $01".to_string());
                                                self.emit("  ORA $01".to_string());
                                                self.emit(format!("  BEQ {}", false_lbl));
                                                self.emit(format!("  JMP {}", true_lbl));

                                                self.emit(format!("{}:", overflow_lbl));
                                                self.emit(format!("  BPL {}", false_lbl));
                                                self.emit(format!("  JMP {}", true_lbl));
                                            }
                                            BinaryOperator::LessThanOrEqual => {
                                                self.emit(format!("  BVS {}", overflow_lbl));
                                                self.emit(format!("  BMI {}", true_lbl));
                                                self.emit("  STX $01".to_string());
                                                self.emit("  ORA $01".to_string());
                                                self.emit(format!("  BEQ {}", true_lbl));
                                                self.emit(format!("  JMP {}", false_lbl));

                                                self.emit(format!("{}:", overflow_lbl));
                                                self.emit(format!("  BPL {}", true_lbl));
                                                self.emit(format!("  JMP {}", false_lbl));
                                            }
                                            _ => {}
                                        }
                                    } else {
                                        match op {
                                            BinaryOperator::LessThan => {
                                                self.emit(format!("  BCC {}", true_lbl));
                                                self.emit(format!("  JMP {}", false_lbl));
                                            }
                                            BinaryOperator::GreaterThanOrEqual => {
                                                self.emit(format!("  BCS {}", true_lbl));
                                                self.emit(format!("  JMP {}", false_lbl));
                                            }
                                            BinaryOperator::GreaterThan => {
                                                self.emit(format!("  BCC {}", false_lbl));
                                                self.emit("  STX $01".to_string());
                                                self.emit("  ORA $01".to_string());
                                                self.emit(format!("  BNE {}", true_lbl));
                                                self.emit(format!("  JMP {}", false_lbl));
                                            }
                                            BinaryOperator::LessThanOrEqual => {
                                                self.emit(format!("  BCC {}", true_lbl));
                                                self.emit("  STX $01".to_string());
                                                self.emit("  ORA $01".to_string());
                                                self.emit(format!("  BEQ {}", true_lbl));
                                                self.emit(format!("  JMP {}", false_lbl));
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                _ => {}
                            }
                            self.emit(format!("{}:", false_lbl));
                            self.emit("  LDA #0".to_string());
                            self.emit(format!("  JMP {}", end_lbl));
                            self.emit(format!("{}:", true_lbl));
                            self.emit("  LDA #$FF".to_string());
                            self.emit(format!("{}:", end_lbl));
                            self.emit("  LDX #0".to_string());
                            Ok(DataType::Bool)
                        }
                    }
                } else {
                    self.emit("  STA $00".to_string());
                    self.emit("  PLA".to_string());
                    match op {
                        BinaryOperator::Add => {
                            self.emit("  CLC".to_string());
                            self.emit("  ADC $00".to_string());
                        }
                        BinaryOperator::Subtract => {
                            self.emit("  SEC".to_string());
                            self.emit("  SBC $00".to_string());
                        }
                        BinaryOperator::Multiply => {
                            self.emit("  JSR Math_Mul8".to_string());
                        }
                        BinaryOperator::Divide => {
                            self.emit("  JSR Math_Div8".to_string());
                        }
                        BinaryOperator::Modulo => {
                            self.emit("  JSR Math_Div8".to_string());
                            self.emit("  LDA $01".to_string()); // Remainder
                        }
                        BinaryOperator::And => {
                            self.emit("  AND $00".to_string());
                        }
                        BinaryOperator::Or => {
                            self.emit("  ORA $00".to_string());
                        }
                        BinaryOperator::Xor => {
                            self.emit("  EOR $00".to_string());
                        }
                        BinaryOperator::ShiftLeft => {
                            self.emit("  JSR Math_Shl8".to_string());
                        }
                        BinaryOperator::ShiftRight => {
                            self.emit("  JSR Math_Shr8".to_string());
                        }
                        _ => {}
                    }
                    self.emit("  LDX #0".to_string());
                    Ok(DataType::Byte)
                }
            }
            Expression::MemberAccess(base, member) => {
                let base_type = self.resolve_type(base).ok_or("Unknown base type")?;
                if let DataType::Enum(enum_name) = base_type {
                    if let Some(sym) = self.symbol_table.resolve(&enum_name) {
                        if let Some(variants) = &sym.variants {
                            for (v_name, val) in variants {
                                if v_name == member {
                                    let low = (val & 0xFF) as u8;
                                    let high = ((val >> 8) & 0xFF) as u8;
                                    self.emit(format!("  LDA #${:02X}", low));
                                    self.emit(format!("  LDX #${:02X}", high));
                                    return Ok(DataType::Int);
                                }
                            }
                        }
                    }
                    return Err(format!("Unknown enum variant {}.{}", enum_name, member));
                }

                if let Ok(addr) = self.get_static_address(expr) {
                    let dtype = self
                        .resolve_type(expr)
                        .ok_or("Cannot resolve type for static address expression")?;
                    match dtype {
                        DataType::Word => {
                            self.emit(format!("  LDA ${:04X}", addr));
                            self.emit(format!("  LDX ${:04X}", addr + 1));
                        }
                        DataType::Int => {
                            self.emit(format!("  LDA ${:04X}", addr));
                            self.emit("  CMP #$80".to_string());
                            let pos_lbl = self.new_label();
                            let done_lbl = self.new_label();
                            self.emit(format!("  BCC {}", pos_lbl));
                            self.emit("  LDX #$FF".to_string());
                            self.emit(format!("  JMP {}", done_lbl));
                            self.emit(format!("{}:", pos_lbl));
                            self.emit("  LDX #0".to_string());
                            self.emit(format!("{}:", done_lbl));
                        }
                        _ => {
                            self.emit(format!("  LDA ${:04X}", addr));
                            self.emit("  LDX #0".to_string());
                        }
                    }
                    Ok(dtype)
                } else {
                    self.generate_address_expression(expr)?;
                    let dtype = self
                        .resolve_type(expr)
                        .ok_or("Cannot resolve type for dynamic address expression")?;
                    self.emit("  LDY #0".to_string());
                    match dtype {
                        DataType::Word => {
                            self.emit("  LDA ($02),Y".to_string());
                            self.emit("  PHA".to_string());
                            self.emit("  INY".to_string());
                            self.emit("  LDA ($02),Y".to_string());
                            self.emit("  TAX".to_string());
                            self.emit("  PLA".to_string());
                        }
                        DataType::Int => {
                            self.emit("  LDA ($02),Y".to_string());
                            self.emit("  CMP #$80".to_string());
                            let pos_lbl = self.new_label();
                            let done_lbl = self.new_label();
                            self.emit(format!("  BCC {}", pos_lbl));
                            self.emit("  LDX #$FF".to_string());
                            self.emit(format!("  JMP {}", done_lbl));
                            self.emit(format!("{}:", pos_lbl));
                            self.emit("  LDX #0".to_string());
                            self.emit(format!("{}:", done_lbl));
                        }
                        _ => {
                            self.emit("  LDA ($02),Y".to_string());
                            self.emit("  LDX #0".to_string());
                        }
                    }
                    Ok(dtype)
                }
            }
            Expression::UnaryOp(op, operand) => {
                let dtype = self.generate_expression(operand)?;
                match op {
                    UnaryOperator::Negate => {
                        // Two's complement
                        if dtype == DataType::Word || dtype == DataType::Int {
                            // Negate A/X (Low/High)
                            self.emit("  EOR #$FF".to_string()); // Low
                            self.emit("  CLC".to_string());
                            self.emit("  ADC #1".to_string());
                            self.emit("  PHA".to_string()); // Save Low
                            self.emit("  TXA".to_string());
                            self.emit("  EOR #$FF".to_string()); // High
                            self.emit("  ADC #0".to_string()); // Carry
                            self.emit("  TAX".to_string());
                            self.emit("  PLA".to_string());
                            Ok(DataType::Int)
                        } else {
                            // Byte
                            self.emit("  EOR #$FF".to_string());
                            self.emit("  CLC".to_string());
                            self.emit("  ADC #1".to_string());
                            // Sign extend to X (Result is Int)
                            self.emit("  CMP #$80".to_string());
                            let sign_lbl = self.new_label();
                            let done_lbl = self.new_label();
                            self.emit(format!("  BCC {}", sign_lbl));
                            self.emit("  LDX #$FF".to_string());
                            self.emit(format!("  JMP {}", done_lbl));
                            self.emit(format!("{}:", sign_lbl));
                            self.emit("  LDX #0".to_string());
                            self.emit(format!("{}:", done_lbl));
                            Ok(DataType::Int) // -Byte is Int
                        }
                    }
                    UnaryOperator::Not => {
                        // One's complement
                        if dtype == DataType::Word || dtype == DataType::Int {
                            self.emit("  EOR #$FF".to_string());
                            self.emit("  PHA".to_string());
                            self.emit("  TXA".to_string());
                            self.emit("  EOR #$FF".to_string());
                            self.emit("  TAX".to_string());
                            self.emit("  PLA".to_string());
                            Ok(dtype)
                        } else {
                            self.emit("  EOR #$FF".to_string());
                            Ok(dtype)
                        }
                    }
                }
            }
            Expression::Peek(addr_expr) => {
                // Optimization: Check for constant address
                let const_addr = match &**addr_expr {
                    Expression::Integer(val) => Some(*val as u16),
                    Expression::Identifier(name) => self
                        .symbol_table
                        .resolve(name)
                        .and_then(|sym| sym.value.map(|val| val as u16)),
                    _ => None,
                };

                if let Some(addr) = const_addr {
                    self.emit(format!("  LDA ${:04X}", addr));
                    self.emit("  LDX #0".to_string());
                    Ok(DataType::Byte)
                } else {
                    self.generate_address_expression(addr_expr)?;
                    self.emit("  LDY #0".to_string());
                    self.emit("  LDA ($02),Y".to_string());
                    self.emit("  LDX #0".to_string());
                    Ok(DataType::Byte)
                }
            }
        }
    }
}
