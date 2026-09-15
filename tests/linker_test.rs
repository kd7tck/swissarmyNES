use std::collections::HashMap;
use swissarmynes::compiler::assembler::Assembler;

fn lines(source: &str) -> Vec<String> {
    source.lines().map(str::to_owned).collect()
}

#[test]
fn data_forward_labels_and_vectors_are_emitted() {
    let banks = HashMap::from([(7, lines(".ORG $C000\nstart: LDA #<data\nLDX #>data\ndata: DB 78,69,83,0\nWORD start\n.ORG $FFFA\nWORD start,start,start"))]);
    let (rom, layout) = Assembler::new()
        .assemble_with_layout(&banks, None, vec![])
        .unwrap();
    let base = 16 + 7 * 16384;
    assert_eq!(
        &rom[base..base + 10],
        &[0xa9, 4, 0xa2, 0xc0, 78, 69, 83, 0, 0, 0xc0]
    );
    assert_eq!(
        &rom[base + 0x3ffa..base + 0x4000],
        &[0, 0xc0, 0, 0xc0, 0, 0xc0]
    );
    assert_eq!(layout[2].address, 0xc004);
    assert_eq!(layout[2].length, 4);
}

#[test]
fn long_branches_are_relaxed_in_both_directions() {
    let mut source = lines(".ORG $8000\nstart: BNE end");
    source.extend(std::iter::repeat_n("NOP".to_owned(), 140));
    source.extend(lines("end: BEQ start"));
    let (rom, layout) = Assembler::new()
        .assemble_with_layout(&HashMap::from([(0, source)]), None, vec![])
        .unwrap();
    assert_eq!(&rom[16..21], &[0xf0, 3, 0x4c, 0x91, 0x80]);
    assert_eq!(&rom[16 + 145..16 + 150], &[0xd0, 3, 0x4c, 0, 0x80]);
    assert_eq!(layout.last().unwrap().address, 0x8091);
    assert_eq!(layout.last().unwrap().length, 5);
}

#[test]
fn overlapping_and_overflowing_segments_are_rejected() {
    for source in [
        ".ORG $BFFF\nWORD 1",
        ".ORG $8000\nDB 1\n.ORG $8000\nDB 2",
        "JMP missing",
    ] {
        assert!(
            Assembler::new()
                .assemble(&lines(source), None, vec![])
                .is_err(),
            "{source}"
        );
    }
    assert!(Assembler::new()
        .assemble(&lines(".ORG $D500\nDB 1"), None, vec![(0xd500, vec![2])])
        .is_err());
    assert!(Assembler::new()
        .assemble_banks(
            &HashMap::from([(3, lines(".ORG $C000\nNOP"))]),
            None,
            vec![]
        )
        .is_err());
}

#[test]
fn expression_subtraction_is_left_associative() {
    let rom = Assembler::new()
        .assemble(&lines("DB 10-3-2"), None, vec![])
        .unwrap();
    assert_eq!(rom[16], 5);
}
