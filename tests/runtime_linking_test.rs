use swiss_emulator::Emulator;
use swissarmynes::compiler::{
    analysis::SemanticAnalyzer, codegen::CodeGenerator, lexer::Lexer, parser::Parser,
};
use swissarmynes::server::api::compile_source;

fn assembly(source: &str) -> Vec<String> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(&program).unwrap();
    let mut generator = CodeGenerator::new(analyzer.symbol_table);
    let (banks, _) = generator.generate_banks(&program).unwrap();
    banks.get(&7).unwrap().clone()
}

#[test]
fn fixed_bank_keeps_required_transitive_helpers_and_symbolic_inline_asm_targets() {
    let unused = assembly("SUB Main()\nEND SUB");
    assert!(!unused.iter().any(|line| line == "Math_Mul16:"));
    assert!(!unused.iter().any(|line| line == "Runtime_SpriteDraw:"));
    assert!(unused.iter().any(|line| line == "Sound_Init:"));

    let source = "DIM result AS WORD\nBANK 3\nSUB Main()\nresult = VAL(STR(1234))\nEND SUB";
    let linked = assembly(source);
    for label in ["Runtime_Val:", "Runtime_Str:", "Math_Mul16:", "Math_Div16:"] {
        assert!(
            linked.iter().any(|line| line == label),
            "missing transitive dependency {label}"
        );
    }
    let (rom, _) = compile_source(Some(source.into()), None, None).unwrap();
    let mut emulator = Emulator::new();
    emulator.load_rom(&rom).unwrap();
    for _ in 0..10 {
        emulator.step().unwrap();
    }
    assert_eq!(
        u16::from_le_bytes([emulator.peek_cpu(0x5c0), emulator.peek_cpu(0x5c1)]),
        1234
    );

    // Inline assembly has no typed AST helper call; its symbolic references
    // must still keep a callable target and its branch bodies alive.
    let inline = "SUB Main()\nASM\nJSR Runtime_Random\nEND ASM\nEND SUB";
    let linked = assembly(inline);
    assert!(linked.iter().any(|line| line == "Runtime_Random:"));
    compile_source(Some(inline.into()), None, None).unwrap();
}

#[test]
fn decimal_string_conversions_preserve_cursors_and_unsigned_word_boundaries() {
    for (expression, expected, numeric) in [
        ("0", "0", 0u16),
        ("1234", "1234", 1234),
        ("32768", "32768", 32768),
        ("65535", "65535", 65535),
        ("-128", "-128", (-128i16) as u16),
    ] {
        let source = format!("DIM text AS STRING\nDIM value AS WORD\nSUB Main()\ntext = STR({expression})\nvalue = VAL(text)\nEND SUB");
        let (rom, _) = compile_source(Some(source), None, None).unwrap();
        let mut emulator = Emulator::new();
        emulator.load_rom(&rom).unwrap();
        for _ in 0..10 {
            emulator.step().unwrap();
        }
        let pointer = u16::from_le_bytes([emulator.peek_cpu(0x5c0), emulator.peek_cpu(0x5c1)]);
        let mut actual = Vec::new();
        for offset in 0..16 {
            let byte = emulator.peek_cpu(pointer + offset);
            if byte == 0 {
                break;
            }
            actual.push(byte);
        }
        assert_eq!(actual, expected.as_bytes(), "STR({expression})");
        assert_eq!(
            u16::from_le_bytes([emulator.peek_cpu(0x5c2), emulator.peek_cpu(0x5c3)]),
            numeric,
            "VAL(STR({expression}))"
        );
    }
}

#[test]
fn dynamic_stores_use_destination_width_without_overwriting_loop_or_neighbor() {
    let source = "DIM bytes(4) AS BYTE\nDIM i AS BYTE\nDIM words(2) AS WORD\nDIM value AS BYTE\nDIM sum AS WORD\nDIM signed(1) AS INT\nDIM promoted AS WORD\nSUB Main()\nFOR i=0 TO 3\nbytes(i)=i+40\nNEXT i\nvalue=99\ni=1\nwords(i)=value\nFOR i=0 TO 10\nsum=sum+i\nNEXT i\nsigned(0)=-128\ni=0\npromoted=signed(i)\nEND SUB";
    let (rom, _) = compile_source(Some(source.into()), None, None).unwrap();
    let mut emulator = Emulator::new();
    emulator.load_rom(&rom).unwrap();
    for _ in 0..10 {
        emulator.step().unwrap();
    }
    assert_eq!(&emulator.ram_snapshot()[0x5c0..0x5c4], &[40, 41, 42, 43]);
    assert_eq!(
        emulator.peek_cpu(0x5c4),
        0,
        "program must return past the completed loops"
    );
    assert_eq!(
        &emulator.ram_snapshot()[0x5c7..0x5c9],
        &[99, 0],
        "BYTE to WORD store must zero extend"
    );
    assert_eq!(
        u16::from_le_bytes([emulator.peek_cpu(0x5ca), emulator.peek_cpu(0x5cb)]),
        55
    );
    assert_eq!(
        &emulator.ram_snapshot()[0x5cd..0x5cf],
        &[0x80, 0xff],
        "dynamic INT reads must sign extend"
    );
}
