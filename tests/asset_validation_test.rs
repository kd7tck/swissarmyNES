use swissarmynes::server::{api::compile_source, project::ProjectAssets};

#[test]
fn malformed_assets_are_rejected_before_rom_emission() {
    let base = serde_json::json!({"chr_bank":[],"palettes":[],"nametables":[]});
    for (field, value) in [
        ("chr_bank", serde_json::json!(vec![0; 8193])),
        (
            "palettes",
            serde_json::json!([{"name":"BG0","colors":[0,1,2,255]}]),
        ),
        (
            "nametables",
            serde_json::json!([{"name":"bad","data":vec![0;961],"attrs":vec![0;64]}]),
        ),
        (
            "world",
            serde_json::json!({"width":1,"height":1,"data":[0]}),
        ),
    ] {
        let mut json = base.clone();
        json[field] = value;
        let assets: ProjectAssets = serde_json::from_value(json).unwrap();
        assert!(
            compile_source(Some("SUB Main()\nEND SUB".into()), None, Some(assets)).is_err(),
            "{field}"
        );
    }
}

#[test]
fn code_generator_reuse_is_deterministic() {
    use swissarmynes::compiler::{
        analysis::SemanticAnalyzer, codegen::CodeGenerator, lexer::Lexer, parser::Parser,
    };
    let source = "DIM x AS STRING\nSUB Main()\nLET x = \"repeat\"\nEND SUB";
    let program = Parser::new(Lexer::new(source).tokenize().unwrap())
        .parse()
        .unwrap();
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(&program).unwrap();
    let mut generator = CodeGenerator::new(analyzer.symbol_table);
    let first = generator.generate_banks(&program).unwrap();
    assert_eq!(first, generator.generate_banks(&program).unwrap());
}
