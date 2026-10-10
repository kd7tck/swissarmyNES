//! World maps use linker-checked segments in otherwise unused MMC1 PRG banks.
//! A versioned directory contains one five-byte bank/address/length reference
//! per cell; duplicate references share a single 1024-byte tile/attribute image.

use crate::compiler::codegen::Assembly;
use crate::server::project::ProjectAssets;
use std::collections::{BTreeMap, BTreeSet, HashMap};

const BANK_BYTES: usize = 16384;
const MAP_BYTES: usize = 1024;
pub const DIRECTORY_VERSION: u8 = 1;
pub const DIRECTORY_RECORD_BYTES: usize = 5;

/// Validate the public packer input before any narrowing cast or code generation.
pub fn validate_assets(assets: &ProjectAssets) -> Result<(), String> {
    let Some(world) = &assets.world else {
        return Ok(());
    };
    let cells = world
        .width
        .checked_mul(world.height)
        .ok_or("World dimensions overflow")?;
    if world.width == 0
        || world.height == 0
        || world.width > 255
        || world.height > 255
        || cells as usize != world.data.len()
    {
        return Err("World dimensions must be 1..255 and match the cell count".into());
    }
    let bytes = world
        .data
        .len()
        .checked_mul(DIRECTORY_RECORD_BYTES)
        .and_then(|bytes| bytes.checked_add(4))
        .ok_or("World directory size overflow")?;
    if bytes > BANK_BYTES {
        return Err(format!(
            "World directory exceeds its 16384-byte PRG bank ({bytes} bytes)"
        ));
    }
    for index in &world.data {
        if *index < -1 || (*index >= 0 && *index as usize >= assets.nametables.len()) {
            return Err(format!("World refers to missing nametable {index}"));
        }
        if *index >= 0 {
            let map = &assets.nametables[*index as usize];
            if map.data.len() != 960 || (!map.attrs.is_empty() && map.attrs.len() != 64) {
                return Err(format!(
                    "World nametable '{index}' requires 960 tile bytes and 64 attributes"
                ));
            }
        }
    }
    Ok(())
}

/// Append generated data to the same assembler input as guest code, never
/// patching a completed ROM or bypassing placement/overlap/source-map checks.
pub fn append_assets(
    banks: &mut HashMap<u8, Assembly>,
    assets: Option<&ProjectAssets>,
) -> Result<(), String> {
    if let Some(assets) = assets {
        validate_assets(assets)?;
    }
    let Some((assets, world)) = assets.and_then(|a| a.world.as_ref().map(|w| (a, w))) else {
        banks.entry(0).or_default().extend([
            "WORLD_WIDTH = 0".into(),
            "WORLD_HEIGHT = 0".into(),
            "WORLD_DIRECTORY_BANK = 0".into(),
            "WORLD_DIRECTORY_ADDR = $8000".into(),
        ]);
        return Ok(());
    };
    let directory_len = world
        .data
        .len()
        .checked_mul(DIRECTORY_RECORD_BYTES)
        .and_then(|bytes| bytes.checked_add(4))
        .ok_or("World directory size overflow")?;
    if directory_len > BANK_BYTES {
        return Err(format!(
            "World directory exceeds its 16384-byte PRG bank ({directory_len} bytes)"
        ));
    }
    let available: Vec<u8> = (1..=6).filter(|bank| !banks.contains_key(bank)).collect();
    let Some(&directory_bank) = available.first() else {
        return Err(
            "World assets need an unused PRG bank in 1..6; all are occupied by code".into(),
        );
    };
    let referenced: BTreeSet<usize> = world
        .data
        .iter()
        .filter(|index| **index >= 0)
        .map(|index| *index as usize)
        .collect();
    let mut segments: BTreeMap<u8, Vec<u8>> =
        BTreeMap::from([(directory_bank, vec![0; directory_len])]);
    let mut references = BTreeMap::new();
    let mut bank_index = 0;
    let mut offset = directory_len.div_ceil(MAP_BYTES) * MAP_BYTES;
    for index in referenced {
        if offset + MAP_BYTES > BANK_BYTES {
            bank_index += 1;
            offset = 0;
        }
        let Some(&bank) = available.get(bank_index) else {
            return Err(format!(
                "World map data exceeds {} unused PRG banks ({} bytes including directory/padding)",
                available.len(),
                available.len() * BANK_BYTES
            ));
        };
        let map = assets
            .nametables
            .get(index)
            .ok_or_else(|| format!("World refers to missing nametable {index}"))?;
        if map.data.len() != 960 || (!map.attrs.is_empty() && map.attrs.len() != 64) {
            return Err(format!(
                "World nametable '{index}' requires 960 tile bytes and 64 attributes"
            ));
        }
        let segment = segments.entry(bank).or_default();
        segment.resize(offset, 0);
        segment.extend_from_slice(&map.data);
        if map.attrs.is_empty() {
            segment.extend([0; 64]);
        } else {
            segment.extend_from_slice(&map.attrs);
        }
        references.insert(index, (bank, 0x8000u16 + offset as u16));
        offset += MAP_BYTES;
    }
    let directory = segments
        .get_mut(&directory_bank)
        .expect("directory bank allocated");
    directory[..4].copy_from_slice(&[
        DIRECTORY_VERSION,
        world.width as u8,
        world.height as u8,
        DIRECTORY_RECORD_BYTES as u8,
    ]);
    for (cell, map_index) in world.data.iter().enumerate() {
        let start = 4 + cell * DIRECTORY_RECORD_BYTES;
        if *map_index < 0 {
            directory[start..start + DIRECTORY_RECORD_BYTES].copy_from_slice(&[0xFF, 0, 0, 0, 0]);
        } else {
            let (bank, address) = references[&(*map_index as usize)];
            directory[start..start + DIRECTORY_RECORD_BYTES].copy_from_slice(&[
                bank,
                address as u8,
                (address >> 8) as u8,
                0,
                4,
            ]);
        }
    }
    for (bank, bytes) in segments {
        let output = banks.entry(bank).or_default();
        output.extend([
            ";@source {\"file\":\"\",\"line\":0}".into(),
            ".ORG $8000".into(),
        ]);
        if bank == directory_bank {
            output.extend([
                format!("WORLD_WIDTH = {}", world.width),
                format!("WORLD_HEIGHT = {}", world.height),
                format!("WORLD_DIRECTORY_BANK = {bank}"),
                "WORLD_DIRECTORY_ADDR = $8004".into(),
                "World_Directory:".into(),
            ]);
        }
        for chunk in bytes.chunks(16) {
            output.push(format!(
                "  DB {}",
                chunk
                    .iter()
                    .map(|byte| format!("${byte:02X}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }
    Ok(())
}

/// The caller evaluates x/y into $00/$01 and $02/$03. World.Load returns
/// $FF on success and zero for an empty/out-of-bounds cell, without replacing
/// the selected map. Full-screen transitions deliberately turn rendering off.
pub fn runtime_assembly() -> Assembly {
    r#"
Runtime_World_Load:
  LDA $01
  ORA $03
  BNE World_Load_Fail
  LDA $00
  CMP #WORLD_WIDTH
  BCS World_Load_Fail
  LDA $02
  CMP #WORLD_HEIGHT
  BCS World_Load_Fail
  PHP
  SEI
  LDA $07F0
  PHA
  LDA $F8
  PHA
  AND #$7B
  STA $F8
  STA $2000
  LDA $F9
  PHA
  LDA #0
  STA $2001
  STA $04
  STA $05
World_Load_RowOffset:
  LDA $02
  BEQ World_Load_CellOffset
  CLC
  LDA $04
  ADC #WORLD_WIDTH
  STA $04
  LDA $05
  ADC #0
  STA $05
  DEC $02
  JMP World_Load_RowOffset
World_Load_CellOffset:
  CLC
  LDA $04
  ADC $00
  STA $04
  LDA $05
  ADC #0
  STA $05
  LDA $04
  STA $00
  LDA $05
  STA $01
  ASL $04
  ROL $05
  ASL $04
  ROL $05
  CLC
  LDA $04
  ADC $00
  STA $04
  LDA $05
  ADC $01
  STA $05
  CLC
  LDA $04
  ADC #<WORLD_DIRECTORY_ADDR
  STA $04
  LDA $05
  ADC #>WORLD_DIRECTORY_ADDR
  STA $05
  LDA #WORLD_DIRECTORY_BANK
  JSR MMC1_SetPrgBank
  LDY #0
  LDA ($04),Y
  CMP #$FF
  BEQ World_Load_Empty
  STA $07
  INY
  LDA ($04),Y
  STA $00
  INY
  LDA ($04),Y
  STA $01
  LDA $07
  STA $07FA
  LDA $00
  STA $07FB
  LDA $01
  STA $07FC
  LDA $07
  JSR MMC1_SetPrgBank
  LDA $2002
  LDA $F8
  AND #3
  ASL
  ASL
  CLC
  ADC #$20
  STA $2006
  LDA #0
  STA $2006
  LDY #0
  LDX #4
World_Load_Page:
  LDA ($00),Y
  STA $2007
  INY
  BNE World_Load_Page
  INC $01
  DEX
  BNE World_Load_Page
  LDA #0
  STA $0380
  LDA #$FF
  STA $06
  JMP World_Load_Restore
World_Load_Empty:
  LDA #0
  STA $06
World_Load_Restore:
  PLA
  STA $08
  PLA
  STA $09
  PLA
  JSR MMC1_SetPrgBank
  LDA $09
  STA $F8
  STA $2000
  LDA $2002
  LDA $E0
  STA $2005
  LDA $E1
  STA $2005
  LDA $08
  STA $F9
  STA $2001
  PLP
  LDA $06
  LDX #0
  RTS
World_Load_Fail:
  LDA #0
  LDX #0
  RTS

Runtime_World_Tile:
  LDA $01
  ORA $03
  BNE World_Tile_Fail
  LDA $02
  CMP #240
  BCS World_Tile_Fail
  LSR
  LSR
  LSR
  STA $02
  LSR
  LSR
  LSR
  STA $03
  LDA $02
  ASL
  ASL
  ASL
  ASL
  ASL
  STA $02
  LDA $00
  LSR
  LSR
  LSR
  CLC
  ADC $02
  STA $02
  JMP Runtime_World_ReadTile
World_Tile_Fail:
  LDA #0
  LDX #0
  RTS

Runtime_World_ReadTile:
  LDA $07FA
  BEQ World_Read_Legacy
  CLC
  LDA $02
  ADC $07FB
  STA $02
  LDA $03
  ADC $07FC
  STA $03
  LDA $07F0
  PHA
  LDA $07FA
  JSR MMC1_SetPrgBank
  LDY #0
  LDA ($02),Y
  STA $00
  PLA
  JSR MMC1_SetPrgBank
  LDA $00
  LDX #0
  RTS
World_Read_Legacy:
  CLC
  LDA $02
  ADC #$00
  STA $02
  LDA $03
  ADC #$D5
  STA $03
  LDY #0
  LDA ($02),Y
  LDX #0
  RTS
"#
    .lines()
    .map(str::trim)
    .filter(|line| !line.is_empty())
    .map(str::to_string)
    .collect()
}
