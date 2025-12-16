use crate::compiler::ast::{DataType, Expression, Program, Statement, TopLevel};
use crate::compiler::symbol_table::{SymbolKind, SymbolTable};

pub struct SemanticAnalyzer {
    pub symbol_table: SymbolTable,
    errors: Vec<String>,
}

impl Default for SemanticAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        Self {
            symbol_table: SymbolTable::new(),
            errors: Vec::new(),
        }
    }

    pub fn analyze(&mut self, program: &Program) -> Result<(), Vec<String>> {
        // First pass: register all top-level symbols (CONST, DIM, SUB names)
        for decl in &program.declarations {
            match decl {
                TopLevel::Const(name, val) => {
                    // For now, only Integer constants are supported in expression resolution
                    if let Err(e) =
                        self.symbol_table
                            .define(name.clone(), DataType::Byte, SymbolKind::Constant)
                    {
                        self.errors.push(e);
                    } else {
                        // Try to evaluate constant value if it's an integer
                        if let Expression::Integer(v) = val {
                            if let Err(e) = self.symbol_table.assign_value(name, *v) {
                                self.errors.push(e);
                            }
                        }
                    }
                }
                TopLevel::Dim(name, dtype) => {
                    if let Err(e) =
                        self.symbol_table
                            .define(name.clone(), dtype.clone(), SymbolKind::Variable)
                    {
                        self.errors.push(e);
                    }
                }
                TopLevel::Sub(name, params, _body) => {
                    // Register Sub name with parameters
                    let param_types = params.iter().map(|(_, t)| t.clone()).collect();
                    if let Err(e) = self.symbol_table.define_with_params(
                        name.clone(),
                        DataType::Byte, // Placeholder for return type
                        SymbolKind::Sub,
                        Some(param_types),
                    ) {
                        self.errors.push(e);
                    }
                }
                TopLevel::Interrupt(name, _body) => {
                    // Interrupts are special, they aren't called by user usually.
                    // But we should register them to prevent name collision.
                    if let Err(e) =
                        self.symbol_table
                            .define(name.clone(), DataType::Byte, SymbolKind::Sub)
                    {
                        self.errors.push(e);
                    }
                }
                TopLevel::Asm(_) => {} // No symbols in ASM block visible to BASIC usually
            }
        }

        // Second pass: analyze bodies
        for decl in &program.declarations {
            match decl {
                TopLevel::Sub(_name, params, body) => {
                    self.symbol_table.enter_scope();

                    // Register params
                    for (p_name, p_type) in params {
                        if let Err(e) = self.symbol_table.define(
                            p_name.clone(),
                            p_type.clone(),
                            SymbolKind::Param,
                        ) {
                            self.errors.push(e);
                        }
                    }

                    // Analyze body
                    self.analyze_block(body);

                    self.symbol_table.exit_scope();
                }
                TopLevel::Interrupt(_name, body) => {
                    self.symbol_table.enter_scope();
                    self.analyze_block(body);
                    self.symbol_table.exit_scope();
                }
                _ => {} // Const/Dim/Asm don't have bodies to analyze
            }
        }

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    fn analyze_block(&mut self, statements: &[Statement]) {
        for stmt in statements {
            self.analyze_statement(stmt);
        }
    }

    fn analyze_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Let(name, expr) => {
                // Check if variable exists. If not, is it implicit local?
                // For now, let's assume we allow implicit locals (common in BASIC).
                // If resolved, check if it's writable (not CONST).
                if let Some(sym) = self.symbol_table.resolve(name) {
                    if sym.kind == SymbolKind::Constant {
                        self.errors
                            .push(format!("Cannot assign to constant '{}'", name));
                    }
                } else {
                    // Implicit declaration (Integer/Byte default?)
                    // Define in current scope
                    if let Err(e) =
                        self.symbol_table
                            .define(name.clone(), DataType::Byte, SymbolKind::Local)
                    {
                        self.errors.push(e);
                    }
                }
                self.analyze_expression(expr);
            }
            Statement::If(cond, then_block, else_block) => {
                self.analyze_expression(cond);
                // IF blocks don't necessarily create new scope in BASIC, usually function-scoped.
                // But for safety, many modern langs do.
                // Standard BASIC does NOT scope IF blocks. Variables inside are function-local.
                // We will NOT enter_scope here to match BASIC behavior usually.
                self.analyze_block(then_block);
                if let Some(else_b) = else_block {
                    self.analyze_block(else_b);
                }
            }
            Statement::While(cond, body) => {
                self.analyze_expression(cond);
                self.analyze_block(body);
            }
            Statement::DoWhile(body, cond) => {
                self.analyze_block(body);
                self.analyze_expression(cond);
            }
            Statement::For(var, start, end, step, body) => {
                // FOR variable is implicitly defined if not present
                if self.symbol_table.resolve(var).is_none() {
                    if let Err(e) =
                        self.symbol_table
                            .define(var.clone(), DataType::Byte, SymbolKind::Local)
                    {
                        self.errors.push(e);
                    }
                } else {
                    // Check if constant
                    if let Some(sym) = self.symbol_table.resolve(var) {
                        if sym.kind == SymbolKind::Constant {
                            self.errors
                                .push(format!("Cannot use constant '{}' as FOR variable", var));
                        }
                    }
                }

                self.analyze_expression(start);
                self.analyze_expression(end);
                if let Some(s) = step {
                    self.analyze_expression(s);
                }
                self.analyze_block(body);
            }
            Statement::Return(expr_opt) => {
                if let Some(expr) = expr_opt {
                    self.analyze_expression(expr);
                }
            }
            Statement::Call(name, args) => {
                // Check if SUB exists
                match self.symbol_table.resolve(name) {
                    Some(sym) => {
                        // Check parameter count
                        if let Some(params) = &sym.params {
                            if params.len() != args.len() {
                                self.errors.push(format!(
                                    "Function/Sub '{}' expects {} arguments, got {}",
                                    name,
                                    params.len(),
                                    args.len()
                                ));
                            }
                        }
                    }
                    None => {
                        self.errors
                            .push(format!("Undefined function/sub '{}'", name));
                    }
                }
                for arg in args {
                    self.analyze_expression(arg);
                }
            }
            Statement::Poke(addr, val) => {
                self.analyze_expression(addr);
                self.analyze_expression(val);
            }
            Statement::Print(args) => {
                for arg in args {
                    self.analyze_expression(arg);
                }
            }
            Statement::Asm(_) => {}
            Statement::Comment(_) => {}
            Statement::On(_intr_name, handler_name) => {
                // Check handler exists?
                // intr_name is NMI, IRQ usually.
                // handler_name is a SUB.
                if self.symbol_table.resolve(handler_name).is_none() {
                    self.errors
                        .push(format!("Undefined interrupt handler '{}'", handler_name));
                }
            }
            Statement::PlaySfx(id_expr) => {
                self.analyze_expression(id_expr);
            }
        }
    }

    fn analyze_expression(&mut self, expr: &Expression) {
        match expr {
            Expression::Identifier(name) => {
                if self.symbol_table.resolve(name).is_none() {
                    self.errors.push(format!("Undefined variable '{}'", name));
                }
            }
            Expression::BinaryOp(left, _, right) => {
                self.analyze_expression(left);
                self.analyze_expression(right);
            }
            Expression::UnaryOp(_, operand) => {
                self.analyze_expression(operand);
            }
            Expression::FunctionCall(name, args) => {
                match self.symbol_table.resolve(name) {
                    Some(sym) => {
                        // Check parameter count
                        if let Some(params) = &sym.params {
                            if params.len() != args.len() {
                                self.errors.push(format!(
                                    "Function '{}' expects {} arguments, got {}",
                                    name,
                                    params.len(),
                                    args.len()
                                ));
                            }
                        }
                    }
                    None => {
                        self.errors.push(format!("Undefined function '{}'", name));
                    }
                }
                for arg in args {
                    self.analyze_expression(arg);
                }
            }
            Expression::Peek(addr) => {
                self.analyze_expression(addr);
            }
            _ => {} // Literals
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::lexer::tokenize;
    use crate::compiler::parser::Parser;

    fn parse_code(input: &str) -> Program {
        let tokens = tokenize(input);
        let mut parser = Parser::new(tokens);
        parser.parse_program().expect("Failed to parse")
    }

    #[test]
    fn test_valid_program() {
        let input = r#"
        CONST MAX = 10
        DIM GlobalVar AS BYTE

        SUB Main()
            x = 5
            GlobalVar = x + MAX
        END SUB
        "#;
        let program = parse_code(input);
        let mut analyzer = SemanticAnalyzer::new();
        let result = analyzer.analyze(&program);
        assert!(result.is_ok());
    }

    #[test]
    fn test_undefined_variable() {
        let input = r#"
        SUB Main()
            y = x + 1
        END SUB
        "#;
        let program = parse_code(input);
        let mut analyzer = SemanticAnalyzer::new();
        let result = analyzer.analyze(&program);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(errs[0].contains("Undefined variable 'x'"));
    }

    #[test]
    fn test_assign_constant() {
        let input = r#"
        CONST MAX = 10
        SUB Main()
            MAX = 5
        END SUB
        "#;
        let program = parse_code(input);
        let mut analyzer = SemanticAnalyzer::new();
        let result = analyzer.analyze(&program);
        assert!(result.is_err());
        assert!(result.unwrap_err()[0].contains("Cannot assign to constant"));
    }

    #[test]
    fn test_undefined_function() {
        let input = r#"
        SUB Main()
            CallUnknown()
        END SUB
        "#;
        let program = parse_code(input);
        let mut analyzer = SemanticAnalyzer::new();
        let result = analyzer.analyze(&program);
        assert!(result.is_err());
        assert!(result.unwrap_err()[0].contains("Undefined function/sub"));
    }

    #[test]
    fn test_parameter_scope() {
        let input = r#"
        SUB Test(p AS BYTE)
            x = p
        END SUB

        SUB Main()
            y = p ' p should not be visible here
        END SUB
        "#;
        let program = parse_code(input);
        let mut analyzer = SemanticAnalyzer::new();
        let result = analyzer.analyze(&program);
        assert!(result.is_err());
        assert!(result.unwrap_err()[0].contains("Undefined variable 'p'"));
    }

    #[test]
    fn test_argument_count_mismatch() {
        let input = r#"
        SUB Test(p AS BYTE, q AS BYTE)
        END SUB

        SUB Main()
            Call Test(1)
        END SUB
        "#;
        let program = parse_code(input);
        let mut analyzer = SemanticAnalyzer::new();
        let result = analyzer.analyze(&program);
        assert!(result.is_err());
        assert!(result.unwrap_err()[0].contains("expects 2 arguments, got 1"));
    }
}
