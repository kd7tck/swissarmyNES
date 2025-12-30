#[derive(Debug, PartialEq, Clone)]
pub enum Expression {
    Integer(i32),
    StringLiteral(String),
    Identifier(String),
    BinaryOp(Box<Expression>, BinaryOperator, Box<Expression>),
    UnaryOp(UnaryOperator, Box<Expression>),
    Call(Box<Expression>, Vec<Expression>), // Replaces FunctionCall. Covers Funcs and Arrays.
    Peek(Box<Expression>),
    MemberAccess(Box<Expression>, String), // structure.member
}

#[derive(Debug, PartialEq, Clone)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Equal,
    NotEqual,
    LessThan,
    GreaterThan,
    LessThanOrEqual,
    GreaterThanOrEqual,
    And,
    Or,
    Xor,
    Modulo,
    ShiftLeft,
    ShiftRight,
}

#[derive(Debug, PartialEq, Clone)]
pub enum UnaryOperator {
    Not,
    Negate, // for negative numbers like -5
}

#[derive(Debug, PartialEq, Clone)]
pub enum StatementKind {
    Let(Expression, Expression), // target, value (target must be lvalue)
    If(Expression, Vec<Statement>, Option<Vec<Statement>>), // condition, then_block, else_block
    While(Expression, Vec<Statement>),
    DoWhile(Vec<Statement>, Expression), // DO ... LOOP WHILE expr
    For(
        String,
        Expression,
        Expression,
        Option<Expression>,
        Vec<Statement>,
    ), // var, start, end, step, body
    Return(Option<Expression>),
    Call(Expression, Vec<Expression>), // CALL Expr(args). Usually Expr is Identifier.
    Poke(Expression, Expression),      // address, value
    PlaySfx(Expression),               // sfx_id
    Print(Vec<Expression>),
    Asm(Vec<String>), // Raw assembly lines
    Comment(String),
    On(String, String),      // ON NMI DO RoutineName
    Read(Vec<String>),       // READ var1, var2
    Restore(Option<String>), // RESTORE [Label]
    Select(
        Expression,
        Vec<(Expression, Vec<Statement>)>,
        Option<Vec<Statement>>,
    ), // SELECT CASE expr, cases, case_else
    WaitVBlank,
    Randomize(Expression), // RANDOMIZE seed
}

#[derive(Debug, PartialEq, Clone)]
pub struct Statement {
    pub kind: StatementKind,
    pub line: usize,
}

#[derive(Debug, PartialEq, Clone)]
pub struct MetaspriteTile {
    pub x: Expression,
    pub y: Expression,
    pub tile: Expression,
    pub attr: Expression,
}

#[derive(Debug, PartialEq, Clone)]
pub struct AnimationFrame {
    pub metasprite: String,
    pub duration: u8,
}

#[derive(Debug, PartialEq, Clone)]
pub enum TopLevelKind {
    Sub(String, Vec<(String, DataType)>, Vec<Statement>), // Name, Params, Body
    TypeDecl(String, Vec<(String, DataType)>),            // TYPE Name ... END TYPE
    Interrupt(String, Vec<Statement>),                    // Interrupt Name (NMI/IRQ), Body
    Const(String, Expression),                            // Global Const
    Dim(String, DataType, Option<Expression>),            // Global Dim with optional initialization
    Asm(Vec<String>),                                     // Top-level ASM block
    Data(Option<String>, Vec<Expression>),                // [Label:] DATA 1, 2, 3
    Include(String),                                      // INCLUDE "filename"
    Enum(String, Vec<(String, Option<i32>)>), // ENUM Name, Members(Name, Optional Value)
    Macro(String, Vec<String>, Vec<Statement>), // MACRO Name, Params, Body
    Metasprite(String, Vec<MetaspriteTile>),  // METASPRITE Name, Tiles
    Animation(String, Vec<AnimationFrame>, bool), // ANIMATION Name, Frames, Loops
    Metatile(String, [u8; 4], u8),            // METATILE Name, Tiles[4], Attr
    World(u32, u32, Vec<i32>),                // WORLD Width, Height, Data (Nametable Indices)
}

#[derive(Debug, PartialEq, Clone)]
pub struct TopLevel {
    pub kind: TopLevelKind,
    pub line: usize,
}

#[derive(Debug, PartialEq, Clone)]
pub enum DataType {
    Byte,
    Word,
    Int,
    Bool,
    String,
    Struct(String),
    Enum(String),
    Array(Box<DataType>, usize), // Array of Type, Size
}

#[derive(Debug, PartialEq, Clone)]
pub struct Program {
    pub declarations: Vec<TopLevel>,
}

impl Statement {
    #[allow(non_snake_case)]
    pub fn Let(target: Expression, value: Expression) -> Self {
        Self { kind: StatementKind::Let(target, value), line: 0 }
    }
    #[allow(non_snake_case)]
    pub fn If(cond: Expression, then_block: Vec<Statement>, else_block: Option<Vec<Statement>>) -> Self {
        Self { kind: StatementKind::If(cond, then_block, else_block), line: 0 }
    }
    #[allow(non_snake_case)]
    pub fn While(cond: Expression, body: Vec<Statement>) -> Self {
        Self { kind: StatementKind::While(cond, body), line: 0 }
    }
    #[allow(non_snake_case)]
    pub fn DoWhile(body: Vec<Statement>, cond: Expression) -> Self {
        Self { kind: StatementKind::DoWhile(body, cond), line: 0 }
    }
    #[allow(non_snake_case)]
    pub fn For(var: String, start: Expression, end: Expression, step: Option<Expression>, body: Vec<Statement>) -> Self {
        Self { kind: StatementKind::For(var, start, end, step, body), line: 0 }
    }
    #[allow(non_snake_case)]
    pub fn Return(expr: Option<Expression>) -> Self {
        Self { kind: StatementKind::Return(expr), line: 0 }
    }
    #[allow(non_snake_case)]
    pub fn Call(target: Expression, args: Vec<Expression>) -> Self {
        Self { kind: StatementKind::Call(target, args), line: 0 }
    }
    #[allow(non_snake_case)]
    pub fn Poke(addr: Expression, val: Expression) -> Self {
        Self { kind: StatementKind::Poke(addr, val), line: 0 }
    }
    #[allow(non_snake_case)]
    pub fn PlaySfx(id: Expression) -> Self {
        Self { kind: StatementKind::PlaySfx(id), line: 0 }
    }
    #[allow(non_snake_case)]
    pub fn Print(args: Vec<Expression>) -> Self {
        Self { kind: StatementKind::Print(args), line: 0 }
    }
    #[allow(non_snake_case)]
    pub fn Asm(lines: Vec<String>) -> Self {
        Self { kind: StatementKind::Asm(lines), line: 0 }
    }
    #[allow(non_snake_case)]
    pub fn Comment(text: String) -> Self {
        Self { kind: StatementKind::Comment(text), line: 0 }
    }
    #[allow(non_snake_case)]
    pub fn On(evt: String, handler: String) -> Self {
        Self { kind: StatementKind::On(evt, handler), line: 0 }
    }
    #[allow(non_snake_case)]
    pub fn Read(vars: Vec<String>) -> Self {
        Self { kind: StatementKind::Read(vars), line: 0 }
    }
    #[allow(non_snake_case)]
    pub fn Restore(label: Option<String>) -> Self {
        Self { kind: StatementKind::Restore(label), line: 0 }
    }
    #[allow(non_snake_case)]
    pub fn Select(expr: Expression, cases: Vec<(Expression, Vec<Statement>)>, else_block: Option<Vec<Statement>>) -> Self {
        Self { kind: StatementKind::Select(expr, cases, else_block), line: 0 }
    }
    #[allow(non_snake_case)]
    pub fn Randomize(expr: Expression) -> Self {
        Self { kind: StatementKind::Randomize(expr), line: 0 }
    }
    #[allow(non_snake_case)]
    pub const WAIT_VBLANK: Self = Self { kind: StatementKind::WaitVBlank, line: 0 };
    #[allow(non_upper_case_globals)]
    pub const WaitVBlank: Self = Self { kind: StatementKind::WaitVBlank, line: 0 };
}

impl TopLevel {
    #[allow(non_snake_case)]
    pub fn Sub(name: String, params: Vec<(String, DataType)>, body: Vec<Statement>) -> Self {
        Self { kind: TopLevelKind::Sub(name, params, body), line: 0 }
    }
    #[allow(non_snake_case)]
    pub fn TypeDecl(name: String, members: Vec<(String, DataType)>) -> Self {
        Self { kind: TopLevelKind::TypeDecl(name, members), line: 0 }
    }
    #[allow(non_snake_case)]
    pub fn Interrupt(name: String, body: Vec<Statement>) -> Self {
        Self { kind: TopLevelKind::Interrupt(name, body), line: 0 }
    }
    #[allow(non_snake_case)]
    pub fn Const(name: String, val: Expression) -> Self {
        Self { kind: TopLevelKind::Const(name, val), line: 0 }
    }
    #[allow(non_snake_case)]
    pub fn Dim(name: String, dtype: DataType, init: Option<Expression>) -> Self {
        Self { kind: TopLevelKind::Dim(name, dtype, init), line: 0 }
    }
    #[allow(non_snake_case)]
    pub fn Asm(lines: Vec<String>) -> Self {
        Self { kind: TopLevelKind::Asm(lines), line: 0 }
    }
    #[allow(non_snake_case)]
    pub fn Data(label: Option<String>, exprs: Vec<Expression>) -> Self {
        Self { kind: TopLevelKind::Data(label, exprs), line: 0 }
    }
    #[allow(non_snake_case)]
    pub fn Include(filename: String) -> Self {
        Self { kind: TopLevelKind::Include(filename), line: 0 }
    }
    #[allow(non_snake_case)]
    pub fn Enum(name: String, members: Vec<(String, Option<i32>)>) -> Self {
        Self { kind: TopLevelKind::Enum(name, members), line: 0 }
    }
    #[allow(non_snake_case)]
    pub fn Macro(name: String, params: Vec<String>, body: Vec<Statement>) -> Self {
        Self { kind: TopLevelKind::Macro(name, params, body), line: 0 }
    }
    #[allow(non_snake_case)]
    pub fn Metasprite(name: String, tiles: Vec<MetaspriteTile>) -> Self {
        Self { kind: TopLevelKind::Metasprite(name, tiles), line: 0 }
    }
    #[allow(non_snake_case)]
    pub fn Animation(name: String, frames: Vec<AnimationFrame>, loops: bool) -> Self {
        Self { kind: TopLevelKind::Animation(name, frames, loops), line: 0 }
    }
    #[allow(non_snake_case)]
    pub fn Metatile(name: String, tiles: [u8; 4], attr: u8) -> Self {
        Self { kind: TopLevelKind::Metatile(name, tiles, attr), line: 0 }
    }
    #[allow(non_snake_case)]
    pub fn World(width: u32, height: u32, data: Vec<i32>) -> Self {
        Self { kind: TopLevelKind::World(width, height, data), line: 0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expression_creation() {
        let expr = Expression::BinaryOp(
            Box::new(Expression::Integer(5)),
            BinaryOperator::Add,
            Box::new(Expression::Integer(10)),
        );

        if let Expression::BinaryOp(left, op, right) = expr {
            assert_eq!(*left, Expression::Integer(5));
            assert_eq!(op, BinaryOperator::Add);
            assert_eq!(*right, Expression::Integer(10));
        } else {
            panic!("Expected BinaryOp");
        }
    }

    #[test]
    fn test_call_creation() {
        let expr = Expression::Call(
            Box::new(Expression::Identifier("MyFunc".to_string())),
            vec![Expression::Integer(1)],
        );
        if let Expression::Call(target, args) = expr {
            assert_eq!(*target, Expression::Identifier("MyFunc".to_string()));
            assert_eq!(args.len(), 1);
        } else {
            panic!("Expected Call");
        }
    }
}
