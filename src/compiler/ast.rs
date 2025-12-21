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
}

#[derive(Debug, PartialEq, Clone)]
pub enum UnaryOperator {
    Not,
    Negate, // for negative numbers like -5
}

#[derive(Debug, PartialEq, Clone)]
pub enum Statement {
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
pub enum TopLevel {
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
