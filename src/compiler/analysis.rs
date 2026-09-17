// KEEP
use crate::compiler::ast::{DataType, Expression, Program, Statement, StatementKind, TopLevelKind};
use crate::compiler::symbol_table::{SymbolKind, SymbolTable};

pub const MAX_RECURSION_DEPTH: usize = 256;

pub struct SemanticAnalyzer {
    pub symbol_table: SymbolTable,
    errors: Vec<String>,
    unsafe_return_depth: usize,
    current_bank: u8,
    recursion_depth: usize,
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
            unsafe_return_depth: 0,
            current_bank: 0,
            recursion_depth: 0,
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

        // AnimState Struct
        let anim_state_members = vec![
            ("ptr".to_string(), DataType::Word, 0),
            ("frame_index".to_string(), DataType::Byte, 2),
            ("timer".to_string(), DataType::Byte, 3),
            ("finished".to_string(), DataType::Byte, 4),
        ];
        let _ = self
            .symbol_table
            .define_struct("AnimState".to_string(), anim_state_members, 5);
    }

    pub fn fold_constants_expr(expr: &Expression) -> Expression {
        match expr {
            Expression::BinaryOp(left, op, right) => {
                let l_folded = Self::fold_constants_expr(left);
                let r_folded = Self::fold_constants_expr(right);
                if let (Expression::Integer(lv), Expression::Integer(rv)) = (&l_folded, &r_folded) {
                    match op {
                        crate::compiler::ast::BinaryOperator::Add => {
                            Expression::Integer(lv.wrapping_add(*rv))
                        }
                        crate::compiler::ast::BinaryOperator::Subtract => {
                            Expression::Integer(lv.wrapping_sub(*rv))
                        }
                        crate::compiler::ast::BinaryOperator::Multiply => {
                            Expression::Integer(lv.wrapping_mul(*rv))
                        }
                        crate::compiler::ast::BinaryOperator::Divide if *rv != 0 => {
                            Expression::Integer(lv.wrapping_div(*rv))
                        }
                        crate::compiler::ast::BinaryOperator::And => Expression::Integer(lv & rv),
                        crate::compiler::ast::BinaryOperator::Or => Expression::Integer(lv | rv),
                        crate::compiler::ast::BinaryOperator::Xor => Expression::Integer(lv ^ rv),
                        _ => {
                            Expression::BinaryOp(Box::new(l_folded), op.clone(), Box::new(r_folded))
                        }
                    }
                } else {
                    Expression::BinaryOp(Box::new(l_folded), op.clone(), Box::new(r_folded))
                }
            }
            Expression::UnaryOp(op, operand) => {
                let o_folded = Self::fold_constants_expr(operand);
                if let Expression::Integer(val) = o_folded {
                    match op {
                        crate::compiler::ast::UnaryOperator::Negate => {
                            Expression::Integer(val.wrapping_neg())
                        }
                        crate::compiler::ast::UnaryOperator::Not => Expression::Integer(!val),
                    }
                } else {
                    Expression::UnaryOp(op.clone(), Box::new(o_folded))
                }
            }
            Expression::Call(callee, args) => {
                let args_folded: Vec<Expression> =
                    args.iter().map(Self::fold_constants_expr).collect();
                if let Expression::Identifier(name) = &**callee {
                    if name.eq_ignore_ascii_case("BITAND") && args_folded.len() == 2 {
                        if let (Expression::Integer(v1), Expression::Integer(v2)) =
                            (&args_folded[0], &args_folded[1])
                        {
                            return Expression::Integer(v1 & v2);
                        }
                    } else if name.eq_ignore_ascii_case("BITOR") && args_folded.len() == 2 {
                        if let (Expression::Integer(v1), Expression::Integer(v2)) =
                            (&args_folded[0], &args_folded[1])
                        {
                            return Expression::Integer(v1 | v2);
                        }
                    } else if name.eq_ignore_ascii_case("BITXOR") && args_folded.len() == 2 {
                        if let (Expression::Integer(v1), Expression::Integer(v2)) =
                            (&args_folded[0], &args_folded[1])
                        {
                            return Expression::Integer(v1 ^ v2);
                        }
                    } else if name.eq_ignore_ascii_case("BITNOT") && args_folded.len() == 1 {
                        if let Expression::Integer(v) = &args_folded[0] {
                            return Expression::Integer(!v);
                        }
                    } else if name.eq_ignore_ascii_case("BITSHL") && args_folded.len() == 2 {
                        if let (Expression::Integer(v1), Expression::Integer(v2)) =
                            (&args_folded[0], &args_folded[1])
                        {
                            return Expression::Integer(v1.wrapping_shl((*v2 & 31) as u32));
                        }
                    } else if name.eq_ignore_ascii_case("BITSHR") && args_folded.len() == 2 {
                        if let (Expression::Integer(v1), Expression::Integer(v2)) =
                            (&args_folded[0], &args_folded[1])
                        {
                            return Expression::Integer(v1.wrapping_shr((*v2 & 31) as u32));
                        }
                    }
                }
                Expression::Call(callee.clone(), args_folded)
            }
            _ => expr.clone(),
        }
    }

    pub fn analyze(&mut self, program: &Program) -> Result<(), Vec<String>> {
        // First pass: register all top-level symbols
        self.current_bank = 0;
        for decl in &program.declarations {
            match &decl.kind {
                TopLevelKind::Bank(b) => {
                    self.current_bank = *b;
                }
                TopLevelKind::Const(name, val) => {
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
                TopLevelKind::Dim(name, dtype, init_expr) => {
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
                TopLevelKind::Sub(name, params, _body) => {
                    let param_types = params.iter().map(|(_, t)| t.clone()).collect();
                    if let Err(e) = self.symbol_table.define_with_params(
                        name.clone(),
                        DataType::Byte, // Placeholder
                        SymbolKind::Sub,
                        Some(param_types),
                        Some(self.current_bank),
                    ) {
                        self.errors.push(e);
                    }
                }
                TopLevelKind::Animation(name, _, _) => {
                    if let Err(e) = self.symbol_table.define_animation(name.clone()) {
                        self.errors.push(e);
                    }
                }
                TopLevelKind::Interrupt(name, _body) => {
                    if let Err(e) =
                        self.symbol_table
                            .define(name.clone(), DataType::Byte, SymbolKind::Sub)
                    {
                        self.errors.push(e);
                    }
                }
                TopLevelKind::TypeDecl(name, members) => {
                    let mut offset = 0u16;
                    let mut member_defs = Vec::new();
                    let mut error = false;

                    for (m_name, m_type) in members {
                        let Some(size) = self.get_type_size(m_type) else {
                            self.errors.push(format!(
                                "Type size overflow for member '{m_name}' in struct '{name}'"
                            ));
                            error = true;
                            continue;
                        };
                        if size == 0 && matches!(m_type, DataType::Struct(_)) {
                            self.errors.push(format!(
                                "Undefined or invalid type for member '{}' in struct '{}'",
                                m_name, name
                            ));
                            error = true;
                        }
                        member_defs.push((m_name.clone(), m_type.clone(), offset));
                        if let Some(next) = offset.checked_add(size) {
                            offset = next;
                        } else {
                            self.errors
                                .push(format!("Type size overflow in struct '{name}'"));
                            error = true;
                        }
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
                TopLevelKind::Enum(name, variants) => {
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
                TopLevelKind::Metasprite(name, _) => {
                    if let Err(e) = self.symbol_table.define_metasprite(name.clone()) {
                        self.errors.push(e);
                    }
                }
                _ => {}
            }
        }

        // Second pass: analyze bodies
        self.current_bank = 0;
        for decl in &program.declarations {
            match &decl.kind {
                TopLevelKind::Bank(b) => {
                    self.current_bank = *b;
                }
                TopLevelKind::Sub(_name, params, body) => {
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
                TopLevelKind::Interrupt(_name, body) => {
                    self.symbol_table.enter_scope();
                    self.analyze_block(body);
                    self.symbol_table.exit_scope();
                }
                TopLevelKind::Animation(name, frames, _) => {
                    for frame in frames {
                        if let Some(sym) = self.symbol_table.resolve(&frame.metasprite) {
                            if sym.kind != SymbolKind::Metasprite {
                                self.errors.push(format!(
                                    "Animation '{}' frame references '{}' which is not a metasprite",
                                    name, frame.metasprite
                                ));
                            }
                        } else {
                            self.errors.push(format!(
                                "Animation '{}' references undefined metasprite '{}'",
                                name, frame.metasprite
                            ));
                        }
                    }
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
        if self.recursion_depth >= MAX_RECURSION_DEPTH {
            self.errors.push(format!(
                "Line {}: Maximum recursion depth exceeded ({})",
                stmt.line, MAX_RECURSION_DEPTH
            ));
            return;
        }
        self.recursion_depth += 1;
        self.analyze_statement_internal(stmt);
        self.recursion_depth -= 1;
    }

    fn analyze_statement_internal(&mut self, stmt: &Statement) {
        match &stmt.kind {
            StatementKind::Let(target, expr) => {
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
            StatementKind::Call(target, args) => {
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
                                    // 3rd arg is metasprite name, but we can't easily validate type here
                                    // if it's passed as a variable (pointer).
                                    // If it's an identifier, we can check if it's a metasprite.
                                    if let Expression::Identifier(name) = &args[2] {
                                        if let Some(sym) = self.symbol_table.resolve(name) {
                                            if sym.kind != SymbolKind::Metasprite {
                                                self.errors.push(format!(
                                                    "Sprite.Draw expects a Metasprite, got '{}'",
                                                    name
                                                ));
                                            }
                                        } else {
                                            self.errors
                                                .push(format!("Undefined symbol '{}'", name));
                                        }
                                    } else {
                                        // Could be an expression returning a pointer?
                                        self.analyze_expression(&args[2]);
                                    }
                                }
                                return;
                            } else if member.eq_ignore_ascii_case("Clear") {
                                if !args.is_empty() {
                                    self.errors
                                        .push("Sprite.Clear expects 0 arguments".to_string());
                                }
                                return;
                            } else if member.eq_ignore_ascii_case("SetFlicker") {
                                if args.len() != 1 {
                                    self.errors.push(
                                        "Sprite.SetFlicker expects 1 argument (enable)".to_string(),
                                    );
                                } else {
                                    self.analyze_expression(&args[0]);
                                }
                                return;
                            } else {
                                self.errors.push(format!(
                                    "Unknown Sprite command '{}' (Draw, Clear, SetFlicker)",
                                    member
                                ));
                                return;
                            }
                        } else if base_name.eq_ignore_ascii_case("Animation") {
                            if member.eq_ignore_ascii_case("Play") {
                                if args.len() != 2 {
                                    self.errors.push(
                                        "Animation.Play expects 2 arguments (state, animation)"
                                            .to_string(),
                                    );
                                } else {
                                    self.analyze_expression(&args[0]);
                                    // Arg 0 must be AnimState
                                    if let Some(DataType::Struct(name)) =
                                        self.resolve_type(&args[0])
                                    {
                                        if name != "AnimState" {
                                            self.errors.push(
                                                "Animation.Play first argument must be AnimState"
                                                    .to_string(),
                                            );
                                        }
                                    } else {
                                        self.errors.push(
                                            "Animation.Play first argument must be AnimState"
                                                .to_string(),
                                        );
                                    }
                                    // Arg 1 must be Animation
                                    if let Expression::Identifier(name) = &args[1] {
                                        if let Some(sym) = self.symbol_table.resolve(name) {
                                            if sym.kind != SymbolKind::Animation {
                                                self.errors.push(format!(
                                                    "Animation.Play expects an Animation, got '{}'",
                                                    name
                                                ));
                                            }
                                        } else {
                                            self.errors
                                                .push(format!("Undefined symbol '{}'", name));
                                        }
                                    } else {
                                        // Could be dynamic? Assume ok if expr
                                        self.analyze_expression(&args[1]);
                                    }
                                }
                                return;
                            } else if member.eq_ignore_ascii_case("Update") {
                                if args.len() != 1 {
                                    self.errors.push(
                                        "Animation.Update expects 1 argument (state)".to_string(),
                                    );
                                } else {
                                    self.analyze_expression(&args[0]);
                                    if let Some(DataType::Struct(name)) =
                                        self.resolve_type(&args[0])
                                    {
                                        if name != "AnimState" {
                                            self.errors.push(
                                                "Animation.Update argument must be AnimState"
                                                    .to_string(),
                                            );
                                        }
                                    }
                                }
                                return;
                            } else if member.eq_ignore_ascii_case("Draw") {
                                if args.len() != 3 {
                                    self.errors.push(
                                        "Animation.Draw expects 3 arguments (x, y, state)"
                                            .to_string(),
                                    );
                                } else {
                                    self.analyze_expression(&args[0]);
                                    self.analyze_expression(&args[1]);
                                    self.analyze_expression(&args[2]);
                                    if let Some(DataType::Struct(name)) =
                                        self.resolve_type(&args[2])
                                    {
                                        if name != "AnimState" {
                                            self.errors.push(
                                                "Animation.Draw 3rd argument must be AnimState"
                                                    .to_string(),
                                            );
                                        }
                                    }
                                }
                                return;
                            } else {
                                self.errors
                                    .push(format!("Unknown Animation command '{}'", member));
                                return;
                            }
                        } else if base_name.eq_ignore_ascii_case("Pool") {
                            if member.eq_ignore_ascii_case("Despawn") {
                                if args.len() != 2 {
                                    self.errors.push(
                                        "Pool.Despawn expects 2 arguments (array, index)"
                                            .to_string(),
                                    );
                                } else {
                                    self.analyze_expression(&args[0]);
                                    if let Some(DataType::Array(_, _)) = self.resolve_type(&args[0])
                                    {
                                        // OK
                                    } else {
                                        self.errors.push(
                                            "Pool.Despawn first argument must be an array"
                                                .to_string(),
                                        );
                                    }
                                    self.analyze_expression(&args[1]);
                                }
                                return;
                            } else if member.eq_ignore_ascii_case("Spawn") {
                                if args.len() != 1 {
                                    self.errors
                                        .push("Pool.Spawn expects 1 argument (array)".to_string());
                                } else {
                                    self.analyze_expression(&args[0]);
                                    if let Some(DataType::Array(_, _)) = self.resolve_type(&args[0])
                                    {
                                        // OK
                                    } else {
                                        self.errors.push(
                                            "Pool.Spawn argument must be an array".to_string(),
                                        );
                                    }
                                }
                                return;
                            } else {
                                self.errors
                                    .push(format!("Unknown Pool command '{}'", member));
                                return;
                            }
                        } else if base_name.eq_ignore_ascii_case("Scroll") {
                            if member.eq_ignore_ascii_case("Set") {
                                if args.len() != 2 {
                                    self.errors
                                        .push("Scroll.Set expects 2 arguments (x, y)".to_string());
                                } else {
                                    self.analyze_expression(&args[0]);
                                    self.analyze_expression(&args[1]);
                                }
                                return;
                            } else if member.eq_ignore_ascii_case("LoadColumn") {
                                if args.len() != 2 {
                                    self.errors.push(
                                        "Scroll.LoadColumn expects 2 arguments (x, array)"
                                            .to_string(),
                                    );
                                } else {
                                    self.analyze_expression(&args[0]);
                                    self.analyze_expression(&args[1]);
                                    if let Some(DataType::Array(_, _)) = self.resolve_type(&args[1])
                                    {
                                        // OK
                                    } else {
                                        self.errors.push(
                                            "Scroll.LoadColumn expects an array as 2nd argument"
                                                .to_string(),
                                        );
                                    }
                                }
                                return;
                            } else if member.eq_ignore_ascii_case("LoadRow") {
                                if args.len() != 2 {
                                    self.errors.push(
                                        "Scroll.LoadRow expects 2 arguments (y, array)".to_string(),
                                    );
                                } else {
                                    self.analyze_expression(&args[0]);
                                    self.analyze_expression(&args[1]);
                                    if let Some(DataType::Array(_, _)) = self.resolve_type(&args[1])
                                    {
                                        // OK
                                    } else {
                                        self.errors.push(
                                            "Scroll.LoadRow expects an array as 2nd argument"
                                                .to_string(),
                                        );
                                    }
                                }
                                return;
                            } else {
                                self.errors.push(format!(
                                    "Unknown Scroll command '{}' (Set, LoadColumn, LoadRow)",
                                    member
                                ));
                                return;
                            }
                        } else if base_name.eq_ignore_ascii_case("PPU") {
                            if member.eq_ignore_ascii_case("Ctrl")
                                || member.eq_ignore_ascii_case("Mask")
                            {
                                if args.len() != 1 {
                                    self.errors
                                        .push(format!("PPU.{} expects 1 argument", member));
                                } else {
                                    self.analyze_expression(&args[0]);
                                }
                                return;
                            } else if member.eq_ignore_ascii_case("SetScroll") {
                                if args.len() != 2 {
                                    self.errors.push(
                                        "PPU.SetScroll expects 2 arguments (x, y)".to_string(),
                                    );
                                } else {
                                    self.analyze_expression(&args[0]);
                                    self.analyze_expression(&args[1]);
                                }
                                return;
                            } else {
                                self.errors
                                    .push(format!("Unknown PPU command '{}'", member));
                                return;
                            }
                        } else if base_name.eq_ignore_ascii_case("Memory") {
                            if member.eq_ignore_ascii_case("Fill") {
                                if args.len() != 3 {
                                    self.errors.push(
                                        "Memory.Fill expects 3 arguments (address, length, value)"
                                            .to_string(),
                                    );
                                } else {
                                    self.analyze_expression(&args[0]);
                                    self.analyze_expression(&args[1]);
                                    self.analyze_expression(&args[2]);
                                }
                                return;
                            } else if member.eq_ignore_ascii_case("Copy") {
                                if args.len() != 3 {
                                    self.errors.push(
                                        "Memory.Copy expects 3 arguments (src_address, dst_address, length)"
                                            .to_string(),
                                    );
                                } else {
                                    self.analyze_expression(&args[0]);
                                    self.analyze_expression(&args[1]);
                                    self.analyze_expression(&args[2]);
                                }
                                return;
                            } else {
                                self.errors.push(format!(
                                    "Unknown Memory command '{}' (Fill, Copy)",
                                    member
                                ));
                                return;
                            }
                        } else if base_name.eq_ignore_ascii_case("Sound") {
                            if member.eq_ignore_ascii_case("Stop") {
                                if !args.is_empty() {
                                    self.errors
                                        .push("Sound.Stop expects 0 arguments".to_string());
                                }
                                return;
                            } else {
                                self.errors
                                    .push(format!("Unknown Sound command '{}' (Stop)", member));
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
            StatementKind::If(cond, then_b, else_b) => {
                self.analyze_expression(cond);
                self.analyze_block(then_b);
                if let Some(b) = else_b {
                    self.analyze_block(b);
                }
            }
            StatementKind::While(cond, body) => {
                self.analyze_expression(cond);
                self.unsafe_return_depth += 1;
                self.analyze_block(body);
                self.unsafe_return_depth -= 1;
            }
            StatementKind::DoWhile(body, cond) => {
                self.unsafe_return_depth += 1;
                self.analyze_block(body);
                self.unsafe_return_depth -= 1;
                self.analyze_expression(cond);
            }
            StatementKind::For(var, start, end, step, body) => {
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
                self.unsafe_return_depth += 1;
                self.analyze_block(body);
                self.unsafe_return_depth -= 1;
            }
            StatementKind::Return(Some(expr)) => {
                if self.unsafe_return_depth > 0 {
                    self.errors
                        .push("Cannot RETURN from inside a loop or SELECT CASE block".to_string());
                }
                self.analyze_expression(expr);
            }
            StatementKind::Return(None) => {
                if self.unsafe_return_depth > 0 {
                    self.errors
                        .push("Cannot RETURN from inside a loop or SELECT CASE block".to_string());
                }
            }
            StatementKind::Poke(addr, val) => {
                self.analyze_expression(addr);
                self.analyze_expression(val);
            }
            StatementKind::PlaySfx(id) => {
                self.analyze_expression(id);
            }
            StatementKind::Print(args) => {
                for arg in args {
                    self.analyze_expression(arg);
                }
            }
            StatementKind::Read(vars) => {
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
            StatementKind::Select(expr, cases, case_else) => {
                self.analyze_expression(expr);
                self.unsafe_return_depth += 1;
                for (val, block) in cases {
                    self.analyze_expression(val);
                    self.analyze_block(block);
                }
                if let Some(b) = case_else {
                    self.analyze_block(b);
                }
                self.unsafe_return_depth -= 1;
            }
            StatementKind::On(event, handler) => {
                if !matches!(event.to_ascii_uppercase().as_str(), "NMI" | "IRQ") {
                    self.errors
                        .push(format!("Unsupported interrupt event {event}"));
                }
                match self.symbol_table.resolve(handler) {
                    Some(symbol)
                        if symbol.kind == SymbolKind::Sub
                            && symbol
                                .params
                                .as_ref()
                                .is_none_or(|params| params.is_empty()) => {}
                    _ => self.errors.push(format!(
                        "Interrupt handler {handler} must be a zero-argument routine"
                    )),
                }
            }
            StatementKind::WaitVBlank => {}
            StatementKind::Randomize(expr) => {
                self.analyze_expression(expr);
            }
            _ => {}
        }
    }

    fn analyze_expression(&mut self, expr: &Expression) {
        if self.recursion_depth >= MAX_RECURSION_DEPTH {
            self.errors.push(format!(
                "Maximum recursion depth exceeded ({})",
                MAX_RECURSION_DEPTH
            ));
            return;
        }
        self.recursion_depth += 1;
        self.analyze_expression_internal(expr);
        self.recursion_depth -= 1;
    }

    fn analyze_expression_internal(&mut self, expr: &Expression) {
        match expr {
            Expression::Identifier(name) => {
                if self.symbol_table.resolve(name).is_none() {
                    self.errors.push(format!("Undefined variable '{}'", name));
                }
            }
            Expression::MemberAccess(base, member) => {
                // Check for Controller or Text or Sprite
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
                    if base_name.eq_ignore_ascii_case("Animation") {
                        return;
                    }
                    if base_name.eq_ignore_ascii_case("Pool") {
                        return;
                    }
                    if base_name.eq_ignore_ascii_case("Collision") {
                        return;
                    }
                    if base_name.eq_ignore_ascii_case("Scroll") {
                        return;
                    }
                    if base_name.eq_ignore_ascii_case("PPU") {
                        return;
                    }
                    if base_name.eq_ignore_ascii_case("Math") {
                        return;
                    }
                    if base_name.eq_ignore_ascii_case("Memory") {
                        return;
                    }
                    if base_name.eq_ignore_ascii_case("Sound") {
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
                    } else if name.eq_ignore_ascii_case("RND") {
                        if args.len() != 1 {
                            self.errors.push("RND expects 1 argument (max)".to_string());
                        } else {
                            self.analyze_expression(&args[0]);
                        }
                        return;
                    } else if name.eq_ignore_ascii_case("BITAND")
                        || name.eq_ignore_ascii_case("BITOR")
                        || name.eq_ignore_ascii_case("BITXOR")
                        || name.eq_ignore_ascii_case("BITSHL")
                        || name.eq_ignore_ascii_case("BITSHR")
                    {
                        if args.len() != 2 {
                            self.errors
                                .push(format!("{} expects 2 arguments", name.to_uppercase()));
                        } else {
                            self.analyze_expression(&args[0]);
                            self.analyze_expression(&args[1]);
                        }
                        return;
                    } else if name.eq_ignore_ascii_case("BITNOT") {
                        if args.len() != 1 {
                            self.errors.push("BITNOT expects 1 argument".to_string());
                        } else {
                            self.analyze_expression(&args[0]);
                        }
                        return;
                    }
                }

                // Controller Methods
                if let Expression::MemberAccess(base, member) = &**callee {
                    if let Expression::Identifier(base_name) = &**base {
                        if base_name.eq_ignore_ascii_case("Math") {
                            if member.eq_ignore_ascii_case("Min")
                                || member.eq_ignore_ascii_case("Max")
                            {
                                if args.len() != 2 {
                                    self.errors
                                        .push(format!("Math.{} expects 2 arguments", member));
                                } else {
                                    self.analyze_expression(&args[0]);
                                    self.analyze_expression(&args[1]);
                                }
                                return;
                            } else if member.eq_ignore_ascii_case("Clamp")
                                || member.eq_ignore_ascii_case("Wrap")
                            {
                                if args.len() != 3 {
                                    self.errors
                                        .push(format!("Math.{} expects 3 arguments", member));
                                } else {
                                    self.analyze_expression(&args[0]);
                                    self.analyze_expression(&args[1]);
                                    self.analyze_expression(&args[2]);
                                }
                                return;
                            } else if member.eq_ignore_ascii_case("Lerp") {
                                if args.len() != 3 {
                                    self.errors.push(
                                        "Math.Lerp expects 3 arguments (a, b, t)".to_string(),
                                    );
                                } else {
                                    self.analyze_expression(&args[0]);
                                    self.analyze_expression(&args[1]);
                                    self.analyze_expression(&args[2]);
                                }
                                return;
                            } else if member.eq_ignore_ascii_case("Sign") {
                                if args.len() != 1 {
                                    self.errors.push("Math.Sign expects 1 argument".to_string());
                                } else {
                                    self.analyze_expression(&args[0]);
                                }
                                return;
                            } else if member.eq_ignore_ascii_case("Abs") {
                                if args.len() != 1 {
                                    self.errors.push("Math.Abs expects 1 argument".to_string());
                                } else {
                                    self.analyze_expression(&args[0]);
                                }
                                return;
                            }
                        }
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
                            } else if member.eq_ignore_ascii_case("AnyPressed") {
                                if !args.is_empty() {
                                    self.errors.push(
                                        "Controller.AnyPressed expects 0 arguments".to_string(),
                                    );
                                }
                                return;
                            }
                        }
                        if base_name.eq_ignore_ascii_case("Pool")
                            && member.eq_ignore_ascii_case("Spawn")
                        {
                            if args.len() != 1 {
                                self.errors
                                    .push("Pool.Spawn expects 1 argument (array)".to_string());
                            } else {
                                self.analyze_expression(&args[0]);
                                if let Some(DataType::Array(_, _)) = self.resolve_type(&args[0]) {
                                    // OK
                                } else {
                                    self.errors
                                        .push("Pool.Spawn argument must be an array".to_string());
                                }
                            }
                            return;
                        }
                        if base_name.eq_ignore_ascii_case("Collision") {
                            if member.eq_ignore_ascii_case("Rect") {
                                if args.len() != 8 {
                                    self.errors
                                        .push("Collision.Rect expects 8 arguments".to_string());
                                } else {
                                    for arg in args {
                                        self.analyze_expression(arg);
                                    }
                                }
                                return;
                            } else if member.eq_ignore_ascii_case("Point") {
                                if args.len() != 6 {
                                    self.errors
                                        .push("Collision.Point expects 6 arguments".to_string());
                                } else {
                                    for arg in args {
                                        self.analyze_expression(arg);
                                    }
                                }
                                return;
                            } else if member.eq_ignore_ascii_case("Tile") {
                                if args.len() != 2 {
                                    self.errors
                                        .push("Collision.Tile expects 2 arguments".to_string());
                                } else {
                                    for arg in args {
                                        self.analyze_expression(arg);
                                    }
                                }
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
                        match sym.data_type.clone() {
                            DataType::Array(_, size) => {
                                // Array Access
                                if args.len() != 1 {
                                    self.errors.push(format!(
                                        "Array '{}' expects 1 index, got {}",
                                        name,
                                        args.len()
                                    ));
                                } else {
                                    let folded_idx = Self::fold_constants_expr(&args[0]);
                                    if let Expression::Integer(idx) = folded_idx {
                                        if idx < 0 || (idx as usize) >= size {
                                            self.errors.push(format!(
                                                "Array '{}' index out of bounds: index {} for array of size {}",
                                                name, idx, size
                                            ));
                                        }
                                    }
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
            Expression::BinaryOp(l, op, r) => {
                self.analyze_expression(l);
                self.analyze_expression(r);
                if matches!(
                    op,
                    crate::compiler::ast::BinaryOperator::Divide
                        | crate::compiler::ast::BinaryOperator::Modulo
                ) {
                    let folded_r = Self::fold_constants_expr(r);
                    if let Expression::Integer(0) = folded_r {
                        self.errors.push("Division by zero".to_string());
                    }
                }
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
                    } else if name.eq_ignore_ascii_case("RND")
                        || name.eq_ignore_ascii_case("BITAND")
                        || name.eq_ignore_ascii_case("BITOR")
                        || name.eq_ignore_ascii_case("BITXOR")
                        || name.eq_ignore_ascii_case("BITNOT")
                        || name.eq_ignore_ascii_case("BITSHL")
                        || name.eq_ignore_ascii_case("BITSHR")
                    {
                        return Some(DataType::Word);
                    }
                }

                // Controller / Math Intrinsics
                if let Expression::MemberAccess(base, member) = &**callee {
                    if let Expression::Identifier(base_name) = &**base {
                        if base_name.eq_ignore_ascii_case("Math") {
                            if member.eq_ignore_ascii_case("Abs")
                                || member.eq_ignore_ascii_case("Min")
                                || member.eq_ignore_ascii_case("Max")
                                || member.eq_ignore_ascii_case("Clamp")
                                || member.eq_ignore_ascii_case("Wrap")
                                || member.eq_ignore_ascii_case("Lerp")
                            {
                                if let Some(arg_type) =
                                    args.first().and_then(|a| self.resolve_type(a))
                                {
                                    return Some(arg_type);
                                }
                                return Some(DataType::Int);
                            } else if member.eq_ignore_ascii_case("Sign") {
                                return Some(DataType::Int);
                            }
                        }
                        if base_name.eq_ignore_ascii_case("Controller")
                            && (member.eq_ignore_ascii_case("IsPressed")
                                || member.eq_ignore_ascii_case("IsHeld")
                                || member.eq_ignore_ascii_case("IsReleased")
                                || member.eq_ignore_ascii_case("AnyPressed"))
                        {
                            return Some(DataType::Bool);
                        }
                        if base_name.eq_ignore_ascii_case("Pool")
                            && member.eq_ignore_ascii_case("Spawn")
                        {
                            return Some(DataType::Int);
                        }
                        if base_name.eq_ignore_ascii_case("Collision") {
                            if member.eq_ignore_ascii_case("Rect")
                                || member.eq_ignore_ascii_case("Point")
                            {
                                return Some(DataType::Bool);
                            }
                            if member.eq_ignore_ascii_case("Tile") {
                                return Some(DataType::Byte);
                            }
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
                    if base_name.eq_ignore_ascii_case("Animation") {
                        return None;
                    }
                    if base_name.eq_ignore_ascii_case("Pool") {
                        return None;
                    }
                    if base_name.eq_ignore_ascii_case("Collision") {
                        return None;
                    }
                    if base_name.eq_ignore_ascii_case("Scroll") {
                        return None;
                    }
                    if base_name.eq_ignore_ascii_case("PPU") {
                        return None;
                    }
                    if base_name.eq_ignore_ascii_case("Math") {
                        return None;
                    }
                    if base_name.eq_ignore_ascii_case("Memory") {
                        return None;
                    }
                    if base_name.eq_ignore_ascii_case("Sound") {
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

    fn get_type_size(&self, dt: &DataType) -> Option<u16> {
        match dt {
            DataType::Byte | DataType::Int | DataType::Bool | DataType::Enum(_) => Some(1),
            DataType::Word | DataType::String => Some(2),
            DataType::Struct(name) => {
                if let Some(sym) = self.symbol_table.resolve(name) {
                    if let Some(size) = sym.value {
                        return u16::try_from(size).ok();
                    }
                }
                Some(0)
            }
            DataType::Array(inner, size) => self
                .get_type_size(inner)?
                .checked_mul(u16::try_from(*size).ok()?),
        }
    }
}
