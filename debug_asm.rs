
use swissarmynes::compiler::analysis::SemanticAnalyzer;
use swissarmynes::compiler::codegen::CodeGenerator;
use swissarmynes::compiler::lexer::Lexer;
use swissarmynes::compiler::parser::Parser;

fn main() {
    let source = "
        CONST MY_VAL = 42
        DIM x AS BYTE

        SUB Main()
            LET x = MY_VAL + 1
        END SUB
    ";

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().expect("Lexing failed");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Parsing failed");
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(&program).expect("Analysis failed");
    let symbol_table = analyzer.symbol_table;
    let mut codegen = CodeGenerator::new(symbol_table);
    let asm_lines = codegen.generate(&program).expect("Codegen failed");
    for (i, line) in asm_lines.iter().enumerate() {
        println!("{:03}: {}", i + 1, line);
    }
}
