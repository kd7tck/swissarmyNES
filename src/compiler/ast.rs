#[derive(Debug, PartialEq, Clone)]
pub enum Expression {
    Integer(i32),
    StringLiteral(String),
    Identifier(String),
    BinaryOp(Box<Expression>, BinaryOperator, Box<Expression>),
    UnaryOp(UnaryOperator, Box<Expression>),
    FunctionCall(String, Vec<Expression>),
    Peek(Box<Expression>),
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
}

#[derive(Debug, PartialEq, Clone)]
pub enum UnaryOperator {
    Not,
    Negate, // for negative numbers like -5
}

#[derive(Debug, PartialEq, Clone)]
pub enum Statement {
    Let(String, Expression),
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
    Call(String, Vec<Expression>), // CALL SubName(args)
    Poke(Expression, Expression),  // address, value
    PlaySfx(Expression),           // sfx_id
    Print(Vec<Expression>),
    Asm(Vec<String>), // Raw assembly lines (if inside a SUB, though usually ASM is top-level too, but can be inline)
    Comment(String),
    On(String, String),      // ON NMI DO RoutineName
    Read(Vec<String>),       // READ var1, var2
    Restore(Option<String>), // RESTORE [Label] (reset data pointer)
    Select(
        Expression,
        Vec<(Expression, Vec<Statement>)>,
        Option<Vec<Statement>>,
    ), // SELECT CASE expr, cases, case_else
}

#[derive(Debug, PartialEq, Clone)]
pub enum TopLevel {
    Sub(String, Vec<(String, DataType)>, Vec<Statement>), // Name, Params, Body
    Interrupt(String, Vec<Statement>),                    // Interrupt Name (NMI/IRQ), Body
    Const(String, Expression),                            // Global Const
    Dim(String, DataType, Option<Expression>),            // Global Dim with optional initialization
    Asm(Vec<String>),                                     // Top-level ASM block
    Data(Option<String>, Vec<Expression>),                // [Label:] DATA 1, 2, 3
    Include(String),                                      // INCLUDE "filename"
                                                          // We could allow generic statements at top level if we want a "script" mode,
                                                          // but strict separation is safer for a compiled language targeting NES.
}

#[derive(Debug, PartialEq, Clone)]
pub enum DataType {
    Byte,
    Word,
    Int,
    Bool,
    String,
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
    fn test_toplevel_creation() {
        let sub = TopLevel::Sub(
            "Main".to_string(),
            vec![],
            vec![Statement::Let("x".to_string(), Expression::Integer(1))],
        );

        if let TopLevel::Sub(name, params, body) = sub {
            assert_eq!(name, "Main");
            assert_eq!(params.len(), 0);
            assert_eq!(body.len(), 1);
        } else {
            panic!("Expected Sub");
        }
    }
}
