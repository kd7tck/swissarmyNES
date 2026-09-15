use swissarmynes::server::api::compile_source;

#[test]
fn include_and_macro_ranges_preserve_call_site() {
    use swissarmynes::compiler::{
        analysis::SemanticAnalyzer, assembler::Assembler, codegen::CodeGenerator, lexer::Lexer,
        parser::Parser, preprocessor, source_map::LinkedSourceMap,
    };
    let source = "INCLUDE \"library.swiss\"\nSUB Main()\nSetX(17)\nEND SUB";
    let library =
        "DIM x AS BYTE\nDEF MACRO SetX(value)\nx = value\nEND MACRO\nSUB Helper()\nx = 23\nEND SUB";
    let program = Parser::new(Lexer::new(source).tokenize().unwrap())
        .parse()
        .unwrap();
    let program = preprocessor::process_includes(program, &|name| {
        assert_eq!(name, "library.swiss");
        Ok(library.to_string())
    })
    .unwrap();
    let program = preprocessor::expand_macros(program).unwrap();
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(&program).unwrap();
    let (banks, _) = CodeGenerator::new(analyzer.symbol_table)
        .generate_banks(&program)
        .unwrap();
    let (rom, layout) = Assembler::new()
        .assemble_with_layout(&banks, None, vec![])
        .unwrap();
    let map = LinkedSourceMap::from_layout(layout);
    for (file, line, value) in [("main.swiss", 3, 17), ("library.swiss", 6, 23)] {
        let entry = map
            .entries
            .iter()
            .find(|entry| entry.file == file && entry.line == line)
            .unwrap();
        assert_eq!(&rom[entry.rom_offset..entry.rom_offset + 2], &[0xa9, value]);
    }
    assert!(!map
        .entries
        .iter()
        .any(|entry| entry.file == "library.swiss" && entry.line == 3));
}

#[test]
fn source_ranges_match_rom_bytes_in_each_bank() {
    let source = "DIM x AS BYTE\nBANK 3\nSUB Other()\nLET x = 42\nEND SUB\nBANK 0\nSUB Main()\nOther()\nEND SUB";
    let (rom, map) = compile_source(Some(source.into()), None, None).unwrap();
    assert_eq!(map.version, 1);
    assert_eq!(map.sources["main.swiss"], source);
    let entry = map.entries.iter().find(|entry| entry.line == 4).unwrap();
    assert_eq!(entry.bank, 3);
    assert_eq!(entry.file, "main.swiss");
    assert_eq!(&rom[entry.rom_offset..entry.rom_offset + 2], &[0xa9, 42]);
    assert_eq!(entry.cpu_end, u32::from(entry.cpu_start) + 2);
    assert!(map
        .entries
        .iter()
        .any(|entry| entry.bank == 0 && entry.line == 8));
}
