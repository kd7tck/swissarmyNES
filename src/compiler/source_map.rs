use super::assembler::Location;
use serde::Serialize;

/// Version one uses physical 16 KiB PRG banks and half-open CPU ranges.
#[derive(Debug, Serialize)]
pub struct LinkedSourceMap {
    pub version: u8,
    /// Exact source snapshots identify the revision without hash collisions.
    pub sources: std::collections::BTreeMap<String, String>,
    pub entries: Vec<SourceRange>,
}

#[derive(Debug, Serialize)]
pub struct SourceRange {
    pub file: String,
    pub line: usize,
    pub bank: u8,
    pub cpu_start: u16,
    pub cpu_end: u32,
    pub rom_offset: usize,
    pub kind: &'static str,
}

impl LinkedSourceMap {
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn from_layout(layout: Vec<Location>) -> Self {
        let entries = layout
            .into_iter()
            .filter(|location| location.executable && location.source_line > 0)
            .map(|location| SourceRange {
                file: if location.source_file.is_empty() {
                    "main.swiss".into()
                } else {
                    location.source_file
                },
                line: location.source_line,
                bank: location.bank,
                cpu_start: location.address,
                cpu_end: u32::from(location.address) + location.length as u32,
                // File-relative offset includes the sixteen-byte iNES header.
                rom_offset: 16 + usize::from(location.bank) * 16384 + usize::from(location.address)
                    - if location.bank == 7 { 0xc000 } else { 0x8000 },
                kind: "instruction",
            })
            .collect();
        Self {
            version: 1,
            sources: Default::default(),
            entries,
        }
    }
}
