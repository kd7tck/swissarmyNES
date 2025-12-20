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
        let mut analyzer = Self {
            symbol_table: SymbolTable::new(),
            errors: Vec::new(),
        };
        analyzer.register_stdlib();
        analyzer
    }

    fn register_stdlib(&mut self) {
        // Button Enum
        let buttons = vec![
            ("A".to_string(), 0x80),
            ("B".to_string(), 0x40),
            ("Select".to_string(), 0x20),
            ("Start".to_string(), 0x10),
            ("Up".to_string(), 0x08),
            ("Down".to_string(), 0x04),
            ("Left".to_string(), 0x02),
            ("Right".to_string(), 0x01),
        ];
        let _ = self.symbol_table.define_enum("Button".to_string(), buttons);
    }

    pub fn analyze(&mut self, program: &Program) -> Result<(), Vec<String>> {
        // First pass: register all top-level symbols
        for decl in &program.declarations {
            match decl {
                TopLevel::Const(name, val) => {
                    if let Err(e) =
                        self.symbol_table
                            .define(name.clone(), DataType::Byte, SymbolKind::Constant)
                    {
                        self.errors.push(e);
                    } else if let Expression::Integer(v) = val {
                        if let Err(e) = self.symbol_table.assign_value(name, *v) {
                            self.errors.push(e);
                        }
                    }
                }
                TopLevel::Dim(name, dtype, init_expr) => {
                    if let Err(e) =
                        self.symbol_table
                            .define(name.clone(), dtype.clone(), SymbolKind::Variable)
                    {
                        self.errors.push(e);
                    }
                    if let Some(init) = init_expr {
                        match dtype {
                            DataType::String => {
                                if let Expression::StringLiteral(_) = init {
                                    // OK
                                } else {
                                    self.errors.push(format!("Variable '{}' of type STRING must be initialized with a string literal", name));
                                }
                            }
                            DataType::Array(_, _) => {
                                self.errors.push(format!(
                                    "Array '{}' cannot be initialized with assignment",
                                    name
                                ));
                            }
                            _ => {}
                        }
                    }
                }
                TopLevel::Sub(name, params, _body) => {
                    let param_types = params.iter().map(|(_, t)| t.clone()).collect();
                    if let Err(e) = self.symbol_table.define_with_params(
                        name.clone(),
                        DataType::Byte, // Placeholder
                        SymbolKind::Sub,
                        Some(param_types),
                    ) {
                        self.errors.push(e);
                    }
                }
                TopLevel::Metasprite(name, _) => {
                    if let Err(e) = self.symbol_table.define(
                        name.clone(),
                        DataType::Word, // It's a pointer
                        SymbolKind::Metasprite,
                    ) {
                        self.errors.push(e);
                    }
                }
                TopLevel::Interrupt(name, _body) => {
                    if let Err(e) =
                        self.symbol_table
                            .define(name.clone(), DataType::Byte, SymbolKind::Sub)
                    {
                        self.errors.push(e);
                    }
                }
                TopLevel::TypeDecl(name, members) => {
                    let mut offset = 0;
                    let mut member_defs = Vec::new();
                    let mut error = false;

                    for (m_name, m_type) in members {
                        let size = self.get_type_size(m_type);
                        if size == 0 && matches!(m_type, DataType::Struct(_)) {
                            self.errors.push(format!(
                                "Undefined or invalid type for member '{}' in struct '{}'",
                                m_name, name
                            ));
                            error = true;
                        }
                        member_defs.push((m_name.clone(), m_type.clone(), offset));
                        offset += size;
                    }

                    if !error {
                        if let Err(e) =
                            self.symbol_table
                                .define_struct(name.clone(), member_defs, offset)
                        {
                            self.errors.push(e);
                        }
                    }
                }
                TopLevel::Enum(name, variants) => {
                    let mut variant_defs = Vec::new();
                    let mut current_val = 0;
                    for (v_name, v_val) in variants {
                        let val = if let Some(v) = v_val {
                            current_val = *v + 1;
                            *v
                        } else {
                            let v = current_val;
                            current_val += 1;
                            v
                        };
                        variant_defs.push((v_name.clone(), val));
                    }
                    if let Err(e) = self.symbol_table.define_enum(name.clone(), variant_defs) {
                        self.errors.push(e);
                    }
                }
                _ => {}
            }
        }

        // Second pass: analyze bodies
        for decl in &program.declarations {
            match decl {
                TopLevel::Sub(_name, params, body) => {
                    self.symbol_table.enter_scope();
                    for (p_name, p_type) in params {
                        if let Err(e) = self.symbol_table.define(
                            p_name.clone(),
                            p_type.clone(),
                            SymbolKind::Param,
                        ) {
                            self.errors.push(e);
                        }
                    }
                    self.analyze_block(body);
                    self.symbol_table.exit_scope();
                }
                TopLevel::Interrupt(_name, body) => {
                    self.symbol_table.enter_scope();
                    self.analyze_block(body);
                    self.symbol_table.exit_scope();
                }
                _ => {}
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
            Statement::Let(target, expr) => {
                // Check target validity (LValue)
                match target {
                    Expression::Identifier(name) => {
                        if let Some(sym) = self.symbol_table.resolve(name) {
                            if sym.kind == SymbolKind::Constant {
                                self.errors
                                    .push(format!("Cannot assign to constant '{}'", name));
                            }
                        } else if let Err(e) = self.symbol_table.define(
                            name.clone(),
                            DataType::Byte,
                            SymbolKind::Local,
                        ) {
                            self.errors.push(e);
                        }
                    }
                    Expression::MemberAccess(base, member) => {
                        self.analyze_expression(base);
                        let base_type = self.resolve_type(base);
                        match base_type {
                            Some(DataType::Struct(struct_name)) => {
                                if let Some(sym) = self.symbol_table.resolve(&struct_name) {
                                    if let Some(members) = &sym.members {
                                        if !members.iter().any(|(n, _, _)| n == member) {
                                            self.errors.push(format!(
                                                "Struct '{}' has no member '{}'",
                                                struct_name, member
                                            ));
                                        }
                                    }
                                } else {
                                    self.errors
                                        .push(format!("Undefined struct type '{}'", struct_name));
                                }
                            }
                            Some(DataType::Enum(enum_name)) => {
                                self.errors.push(format!(
                                    "Cannot assign to enum member '{}.{}'",
                                    enum_name, member
                                ));
                            }
                            _ => {}
                        }
                    }
                    Expression::Call(callee, _args) => {
                        // Array assignment? e.g. x(i) = 1
                        self.analyze_expression(target); // Recursively check (validates array existence and args)

                        // Check if it resolves to an Array
                        if let Some(dtype) = self.resolve_type(callee) {
                            match dtype {
                                DataType::Array(_, _) => {
                                    // Good
                                }
                                _ => {
                                    // Trying to assign to function call? "MyFunc() = 1" -> Error
                                    self.errors
                                        .push("Cannot assign to function call".to_string());
                                }
                            }
                        }
                    }
                    _ => self.errors.push("Invalid assignment target".to_string()),
                }
                self.analyze_expression(expr);
            }
            Statement::Call(target, args) => {
                // Check for Member Access Call (Controller, Text)
                if let Expression::MemberAccess(base, member) = target {
                    if let Expression::Identifier(base_name) = &**base {
                        if base_name.eq_ignore_ascii_case("Controller") {
                            if member.eq_ignore_ascii_case("Read") {
                                if !args.is_empty() {
                                    self.errors
                                        .push("Controller.Read expects 0 arguments".to_string());
                                }
                                return;
                            } else {
                                self.errors.push(format!(
                                    "Unknown Controller command '{}' (did you mean Read?)",
                                    member
                                ));
                                return;
                            }
                        } else if base_name.eq_ignore_ascii_case("Sprite") {
                            if member.eq_ignore_ascii_case("Draw") {
                                if args.len() != 3 {
                                    self.errors.push(
                                        "Sprite.Draw expects 3 arguments (x, y, metasprite)"
                                            .to_string(),
                                    );
                                } else {
                                    self.analyze_expression(&args[0]);
                                    self.analyze_expression(&args[1]);
                                    self.analyze_expression(&args[2]);
                                }
                                return;
                            } else if member.eq_ignore_ascii_case("Clear") {
                                if !args.is_empty() {
                                    self.errors
                                        .push("Sprite.Clear expects 0 arguments".to_string());
                                }
                                return;
                            } else {
                                self.errors.push(format!(
                                    "Unknown Sprite command '{}' (Draw, Clear)",
                                    member
                                ));
                                return;
                            }
                        } else if base_name.eq_ignore_ascii_case("Text") {
                            if member.eq_ignore_ascii_case("Print") {
                                if args.len() != 3 {
                                    self.errors.push(
                                        "Text.Print expects 3 arguments (x, y, string)".to_string(),
                                    );
                                } else {
                                    self.analyze_expression(&args[0]);
                                    self.analyze_expression(&args[1]);
                                    self.analyze_expression(&args[2]);
                                    if let Some(dtype) = self.resolve_type(&args[2]) {
                                        if dtype != DataType::String {
                                            self.errors.push(
                                                "Text.Print expects string as 3rd argument"
                                                    .to_string(),
                                            );
                                        }
                                    }
                                }
                                return;
                            } else if member.eq_ignore_ascii_case("SetOffset") {
                                if args.len() != 1 {
                                    self.errors.push(
                                        "Text.SetOffset expects 1 argument (offset)".to_string(),
                                    );
                                } else {
                                    self.analyze_expression(&args[0]);
                                }
                                return;
                            } else {
                                self.errors.push(format!(
                                    "Unknown Text command '{}' (Print, SetOffset)",
                                    member
                                ));
                                return;
                            }
                        }
                    }
                }

                // Should resolve to Sub
                // If target is Identifier, check if Sub exists
                if let Expression::Identifier(name) = target {
                    match self.symbol_table.resolve(name) {
                        Some(sym) => {
                            if sym.kind != SymbolKind::Sub {
                                // Allow calling variables if implicit? No, Call x is invalid.
                                // But `Call arr(i)` might be parsed as `Call(ArrayAccess, [])`? No.
                                // Parser: `Call expr`.
                            }
                            if let Some(params) = &sym.params {
                                if params.len() != args.len() {
                                    self.errors.push(format!(
                                        "Sub '{}' expects {} arguments, got {}",
                                        name,
                                        params.len(),
                                        args.len()
                                    ));
                                }
                            }
                        }
                        None => {
                            self.errors.push(format!("Undefined sub '{}'", name));
                        }
                    }
                } else {
                    // Indirect call not supported, or complex expression call
                    // e.g. Call Struct.Method()
                    self.analyze_expression(target);
                }
                for arg in args {
                    self.analyze_expression(arg);
                }
            }
            Statement::If(cond, then_b, else_b) => {
                self.analyze_expression(cond);
                self.analyze_block(then_b);
                if let Some(b) = else_b {
                    self.analyze_block(b);
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
                if self.symbol_table.resolve(var).is_none() {
                    let _ =
                        self.symbol_table
                            .define(var.clone(), DataType::Byte, SymbolKind::Local);
                }
                self.analyze_expression(start);
                self.analyze_expression(end);
                if let Some(s) = step {
                    self.analyze_expression(s);
                }
                self.analyze_block(body);
            }
            Statement::Return(Some(expr)) => {
                self.analyze_expression(expr);
            }
            Statement::Return(None) => {}
            Statement::Poke(addr, val) => {
                self.analyze_expression(addr);
                self.analyze_expression(val);
            }
            Statement::PlaySfx(id) => {
                self.analyze_expression(id);
            }
            Statement::Print(args) => {
                for arg in args {
                    self.analyze_expression(arg);
                }
            }
            Statement::Read(vars) => {
                for var in vars {
                    if self.symbol_table.resolve(var).is_none() {
                        let _ = self.symbol_table.define(
                            var.clone(),
                            DataType::Byte,
                            SymbolKind::Local,
                        );
                    }
                }
            }
            Statement::Select(expr, cases, case_else) => {
                self.analyze_expression(expr);
                for (val, block) in cases {
                    self.analyze_expression(val);
                    self.analyze_block(block);
                }
                if let Some(b) = case_else {
                    self.analyze_block(b);
                }
            }
            _ => {}
        }
    }

    fn analyze_expression(&mut self, expr: &Expression) {
        match expr {
            Expression::Identifier(name) => {
                if self.symbol_table.resolve(name).is_none() {
                    self.errors.push(format!("Undefined variable '{}'", name));
                }
            }
            Expression::MemberAccess(base, member) => {
                // Check for Controller or Text
                if let Expression::Identifier(base_name) = &**base {
                    if base_name.eq_ignore_ascii_case("Controller") {
                        return; // No direct member access check needed here, already safe
                    }
                    if base_name.eq_ignore_ascii_case("Text") {
                        return;
                    }
                    if base_name.eq_ignore_ascii_case("Sprite") {
                        return;
                    }
                }

                self.analyze_expression(base);
                let base_type = self.resolve_type(base);
                match base_type {
                    Some(DataType::Struct(name)) => {
                        if let Some(sym) = self.symbol_table.resolve(&name) {
                            if let Some(members) = &sym.members {
                                if !members.iter().any(|(n, _, _)| n == member) {
                                    self.errors.push(format!(
                                        "Struct '{}' has no member '{}'",
                                        name, member
                                    ));
                                }
                            }
                        }
                    }
                    Some(DataType::Enum(name)) => {
                        if let Some(sym) = self.symbol_table.resolve(&name) {
                            if let Some(variants) = &sym.variants {
                                if !variants.iter().any(|(n, _)| n == member) {
                                    self.errors.push(format!(
                                        "Enum '{}' has no variant '{}'",
                                        name, member
                                    ));
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            Expression::Call(callee, args) => {
                // Built-ins check first
                if let Expression::Identifier(name) = &**callee {
                    if name.eq_ignore_ascii_case("LEN") {
                        if args.len() != 1 {
                            self.errors.push("LEN expects 1 argument".to_string());
                        } else {
                            // Check argument type
                            self.analyze_expression(&args[0]);
                            if let Some(dtype) = self.resolve_type(&args[0]) {
                                if dtype != DataType::String {
                                    self.errors
                                        .push("LEN expects a string argument".to_string());
                                }
                            }
                        }
                        return;
                    } else if name.eq_ignore_ascii_case("ABS") {
                        if args.len() != 1 {
                            self.errors.push("ABS expects 1 argument".to_string());
                        } else {
                            self.analyze_expression(&args[0]);
                        }
                        return;
                    } else if name.eq_ignore_ascii_case("SGN") {
                        if args.len() != 1 {
                            self.errors.push("SGN expects 1 argument".to_string());
                        } else {
                            self.analyze_expression(&args[0]);
                        }
                        return;
                    } else if name.eq_ignore_ascii_case("ASC") {
                        if args.len() != 1 {
                            self.errors.push("ASC expects 1 argument".to_string());
                        } else {
                            self.analyze_expression(&args[0]);
                            if let Some(dtype) = self.resolve_type(&args[0]) {
                                if dtype != DataType::String {
                                    self.errors
                                        .push("ASC expects a string argument".to_string());
                                }
                            }
                        }
                        return;
                    } else if name.eq_ignore_ascii_case("VAL") {
                        if args.len() != 1 {
                            self.errors.push("VAL expects 1 argument".to_string());
                        } else {
                            self.analyze_expression(&args[0]);
                            if let Some(dtype) = self.resolve_type(&args[0]) {
                                if dtype != DataType::String {
                                    self.errors
                                        .push("VAL expects a string argument".to_string());
                                }
                            }
                        }
                        return;
                    } else if name.eq_ignore_ascii_case("CHR") {
                        if args.len() != 1 {
                            self.errors.push("CHR expects 1 argument".to_string());
                        } else {
                            self.analyze_expression(&args[0]);
                            if let Some(dtype) = self.resolve_type(&args[0]) {
                                match dtype {
                                    DataType::Byte | DataType::Word | DataType::Int => {}
                                    _ => self
                                        .errors
                                        .push("CHR expects a numeric argument".to_string()),
                                }
                            }
                        }
                        return;
                    } else if name.eq_ignore_ascii_case("STR") {
                        if args.len() != 1 {
                            self.errors.push("STR expects 1 argument".to_string());
                        } else {
                            self.analyze_expression(&args[0]);
                            if let Some(dtype) = self.resolve_type(&args[0]) {
                                match dtype {
                                    DataType::Byte | DataType::Word | DataType::Int => {}
                                    _ => self
                                        .errors
                                        .push("STR expects a numeric argument".to_string()),
                                }
                            }
                        }
                        return;
                    } else if name.eq_ignore_ascii_case("LEFT") {
                        if args.len() != 2 {
                            self.errors.push("LEFT expects 2 arguments".to_string());
                        } else {
                            self.analyze_expression(&args[0]);
                            self.analyze_expression(&args[1]);
                            if let Some(dtype) = self.resolve_type(&args[0]) {
                                if dtype != DataType::String {
                                    self.errors
                                        .push("LEFT expects string as first argument".to_string());
                                }
                            }
                        }
                        return;
                    } else if name.eq_ignore_ascii_case("RIGHT") {
                        if args.len() != 2 {
                            self.errors.push("RIGHT expects 2 arguments".to_string());
                        } else {
                            self.analyze_expression(&args[0]);
                            self.analyze_expression(&args[1]);
                            if let Some(dtype) = self.resolve_type(&args[0]) {
                                if dtype != DataType::String {
                                    self.errors
                                        .push("RIGHT expects string as first argument".to_string());
                                }
                            }
                        }
                        return;
                    } else if name.eq_ignore_ascii_case("MID") {
                        if args.len() != 3 {
                            self.errors.push("MID expects 3 arguments".to_string());
                        } else {
                            self.analyze_expression(&args[0]);
                            self.analyze_expression(&args[1]);
                            self.analyze_expression(&args[2]);
                            if let Some(dtype) = self.resolve_type(&args[0]) {
                                if dtype != DataType::String {
                                    self.errors
                                        .push("MID expects string as first argument".to_string());
                                }
                            }
                        }
                        return;
                    }
                }

                // Controller Methods
                if let Expression::MemberAccess(base, member) = &**callee {
                    if let Expression::Identifier(base_name) = &**base {
                        if base_name.eq_ignore_ascii_case("Controller") {
                            if member.eq_ignore_ascii_case("IsPressed")
                                || member.eq_ignore_ascii_case("IsHeld")
                                || member.eq_ignore_ascii_case("IsReleased")
                            {
                                if args.len() != 1 {
                                    self.errors
                                        .push(format!("Controller.{} expects 1 argument", member));
                                } else {
                                    self.analyze_expression(&args[0]);
                                }
                                return;
                            } else {
                                self.errors
                                    .push(format!("Unknown Controller function '{}'", member));
                                return;
                            }
                        }
                    }
                }

                // Check if it's Array Access or Function Call
                self.analyze_expression(callee);

                // If callee is Identifier
                if let Expression::Identifier(name) = &**callee {
                    if let Some(sym) = self.symbol_table.resolve(name) {
                        match &sym.data_type {
                            DataType::Array(_, _) => {
                                // Array Access
                                if args.len() != 1 {
                                    self.errors.push(format!(
                                        "Array '{}' expects 1 index, got {}",
                                        name,
                                        args.len()
                                    ));
                                }
                            }
                            _ => {
                                if sym.kind == SymbolKind::Sub {
                                    // Function Call
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
                                } else {
                                    self.errors
                                        .push(format!("'{}' is not a function or array", name));
                                }
                            }
                        }
                    }
                } else {
                    // Indirect Call / Member Array access
                    // If type is Array, check args
                    if let Some(DataType::Array(_, _)) = self.resolve_type(callee) {
                        if args.len() != 1 {
                            self.errors
                                .push(format!("Array access expects 1 index, got {}", args.len()));
                        }
                    }
                }

                for arg in args {
                    self.analyze_expression(arg);
                }
            }
            Expression::BinaryOp(l, _, r) => {
                self.analyze_expression(l);
                self.analyze_expression(r);
            }
            Expression::UnaryOp(_, e) => self.analyze_expression(e),
            Expression::Peek(e) => self.analyze_expression(e),
            _ => {}
        }
    }

    fn resolve_type(&self, expr: &Expression) -> Option<DataType> {
        match expr {
            Expression::Identifier(name) => {
                self.symbol_table.resolve(name).map(|s| s.data_type.clone())
            }
            Expression::Call(callee, args) => {
                // Built-ins
                if let Expression::Identifier(name) = &**callee {
                    if name.eq_ignore_ascii_case("LEN") {
                        return Some(DataType::Word);
                    } else if name.eq_ignore_ascii_case("ABS") {
                        if let Some(arg_type) = args.first().and_then(|a| self.resolve_type(a)) {
                            return Some(arg_type);
                        }
                        return Some(DataType::Int);
                    } else if name.eq_ignore_ascii_case("SGN") {
                        return Some(DataType::Int);
                    } else if name.eq_ignore_ascii_case("ASC") || name.eq_ignore_ascii_case("VAL") {
                        return Some(DataType::Word);
                    } else if name.eq_ignore_ascii_case("CHR")
                        || name.eq_ignore_ascii_case("STR")
                        || name.eq_ignore_ascii_case("LEFT")
                        || name.eq_ignore_ascii_case("RIGHT")
                        || name.eq_ignore_ascii_case("MID")
                    {
                        return Some(DataType::String);
                    }
                }

                // Controller
                if let Expression::MemberAccess(base, member) = &**callee {
                    if let Expression::Identifier(base_name) = &**base {
                        if base_name.eq_ignore_ascii_case("Controller")
                            && (member.eq_ignore_ascii_case("IsPressed")
                                || member.eq_ignore_ascii_case("IsHeld")
                                || member.eq_ignore_ascii_case("IsReleased"))
                        {
                            return Some(DataType::Bool);
                        }
                    }
                }

                // If Array, return inner type
                // If Function, return Word/Byte (Implicit)
                if let Some(DataType::Array(inner, _)) = self.resolve_type(callee) {
                    return Some(*inner);
                }
                Some(DataType::Word) // Default function return
            }
            Expression::MemberAccess(base, member) => {
                // Controller check
                if let Expression::Identifier(base_name) = &**base {
                    if base_name.eq_ignore_ascii_case("Controller") {
                        return None;
                    }
                    if base_name.eq_ignore_ascii_case("Text") {
                        return None;
                    }
                    if base_name.eq_ignore_ascii_case("Sprite") {
                        return None;
                    }
                }

                let base_type = self.resolve_type(base)?;
                match base_type {
                    DataType::Struct(name) => {
                        if let Some(sym) = self.symbol_table.resolve(&name) {
                            if let Some(members) = &sym.members {
                                for (m_name, m_type, _) in members {
                                    if m_name == member {
                                        return Some(m_type.clone());
                                    }
                                }
                            }
                        }
                    }
                    DataType::Enum(name) => {
                        // Check if variant exists
                        if let Some(sym) = self.symbol_table.resolve(&name) {
                            if let Some(variants) = &sym.variants {
                                for (v_name, _) in variants {
                                    if v_name == member {
                                        return Some(DataType::Int); // Enums resolve to Int
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
                None
            }
            Expression::Integer(_) => Some(DataType::Word),
            Expression::StringLiteral(_) => Some(DataType::String),
            Expression::BinaryOp(_, _, _) => Some(DataType::Word),
            Expression::UnaryOp(_, _) => Some(DataType::Int),
            Expression::Peek(_) => Some(DataType::Byte),
        }
    }

    fn get_type_size(&self, dt: &DataType) -> u16 {
        match dt {
            DataType::Byte | DataType::Int | DataType::Bool | DataType::Enum(_) => 1,
            DataType::Word | DataType::String => 2,
            DataType::Struct(name) => {
                if let Some(sym) = self.symbol_table.resolve(name) {
                    if let Some(size) = sym.value {
                        return size as u16;
                    }
                }
                0
            }
            DataType::Array(inner, size) => self.get_type_size(inner) * (*size as u16),
        }
    }
}
