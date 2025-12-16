use super::ast::{
    BinaryOperator, DataType, Expression, Program, Statement, TopLevel, UnaryOperator,
};
use super::lexer::Token;

pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

#[derive(PartialEq, PartialOrd)]
enum Precedence {
    None,
    Or,         // OR
    And,        // AND
    Equality,   // = <>
    Comparison, // < > <= >=
    Term,       // + -
    Factor,     // * /
    Unary,      // NOT -
    Call,       // . ()
    Primary,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    pub fn parse(&mut self) -> Result<Program, String> {
        self.parse_program()
    }

    pub fn parse_program(&mut self) -> Result<Program, String> {
        let mut declarations = Vec::new();

        while !self.is_at_end() {
            // Skip top-level newlines
            while self.match_token(Token::Newline) {
                // just consume
            }
            if self.is_at_end() {
                break;
            }
            let decl = self.parse_top_level()?;
            declarations.push(decl);
        }

        Ok(Program { declarations })
    }

    fn parse_top_level(&mut self) -> Result<TopLevel, String> {
        if self.match_token(Token::Const) {
            // CONST Name = Value
            let name = if let Token::Identifier(n) = self.advance().clone() {
                n
            } else {
                return Err("Expected identifier after CONST".to_string());
            };
            self.consume(Token::Equal, "Expected '=' in CONST declaration")?;
            let val = self.parse_expression()?;
            // Optional newline
            self.match_token(Token::Newline);
            return Ok(TopLevel::Const(name, val));
        }

        if self.match_token(Token::Dim) {
            // DIM Name AS Type
            let name = if let Token::Identifier(n) = self.advance().clone() {
                n
            } else {
                return Err("Expected identifier after DIM".to_string());
            };

            self.consume(Token::As, "Expected AS after DIM name")?;
            let data_type = self.parse_type()?;

            // Optional newline
            self.match_token(Token::Newline);
            return Ok(TopLevel::Dim(name, data_type));
        }

        if self.match_token(Token::Sub) {
            let name = if let Token::Identifier(n) = self.advance().clone() {
                n
            } else {
                return Err("Expected identifier after SUB".to_string());
            };
            self.consume(Token::LParen, "Expected '(' after SUB name")?;

            let mut params = Vec::new();
            if !self.check(Token::RParen) {
                loop {
                    let param_name = if let Token::Identifier(n) = self.advance().clone() {
                        n
                    } else {
                        return Err("Expected parameter name".to_string());
                    };

                    // Name AS Type
                    self.consume(Token::As, "Expected AS after parameter name")?;
                    let param_type = self.parse_type()?;
                    params.push((param_name, param_type));

                    if !self.match_token(Token::Comma) {
                        break;
                    }
                }
            }
            self.consume(Token::RParen, "Expected ')' after SUB parameters")?;
            self.consume(Token::Newline, "Expected newline after SUB definition")?;

            let body = self.parse_block()?;
            self.consume(Token::End, "Expected END SUB")?;
            self.consume(Token::Sub, "Expected SUB after END")?;

            return Ok(TopLevel::Sub(name, params, body));
        }

        if self.match_token(Token::Interrupt) {
            let name = if let Token::Identifier(n) = self.advance().clone() {
                n
            } else {
                return Err("Expected identifier after INTERRUPT".to_string());
            };
            // Interrupts usually take no arguments?
            // AST: Interrupt(String, Vec<Statement>)
            // Syntax: INTERRUPT NMI() ... END INTERRUPT
            self.consume(Token::LParen, "Expected '(' after INTERRUPT name")?;
            self.consume(Token::RParen, "Expected ')' after INTERRUPT name")?;
            self.consume(
                Token::Newline,
                "Expected newline after INTERRUPT definition",
            )?;

            let body = self.parse_block()?;
            self.consume(Token::End, "Expected END INTERRUPT")?;
            self.consume(Token::Interrupt, "Expected INTERRUPT after END")?;

            return Ok(TopLevel::Interrupt(name, body));
        }

        if self.match_token(Token::Asm) {
            // Top level ASM
            // Same logic as Statement::Asm essentially, but returns TopLevel::Asm
            let mut lines = Vec::new();
            while !self.check(Token::End) && !self.is_at_end() {
                let mut line = String::new();
                while !self.check(Token::Newline) && !self.is_at_end() {
                    let t = self.advance();
                    match t {
                        Token::Identifier(s) => line.push_str(s),
                        Token::Integer(n) => line.push_str(&n.to_string()),
                        Token::StringLiteral(s) => line.push_str(&format!("\"{}\"", s)),
                        Token::Comma => line.push(','),
                        Token::Hash => line.push('#'),
                        Token::Colon => line.push(':'),
                        Token::SemiColon => line.push(';'),
                        _ => line.push('?'),
                    }
                    line.push(' ');
                }
                lines.push(line.trim().to_string());
                if !self.is_at_end() {
                    self.advance();
                }
            }
            self.consume(Token::End, "Expected END after ASM block")?;
            self.consume(Token::Asm, "Expected ASM after END")?;
            return Ok(TopLevel::Asm(lines));
        }

        Err(format!("Unexpected token at top level: {:?}", self.peek()))
    }

    fn parse_type(&mut self) -> Result<DataType, String> {
        if self.match_token(Token::Byte) {
            return Ok(DataType::Byte);
        }
        if self.match_token(Token::Word) {
            return Ok(DataType::Word);
        }
        if self.match_token(Token::Bool) {
            return Ok(DataType::Bool);
        }
        Err(format!(
            "Expected type (BYTE, WORD, BOOL), found {:?}",
            self.peek()
        ))
    }

    // --- Statement Parsing ---

    fn parse_statement(&mut self) -> Result<Statement, String> {
        if self.match_token(Token::Let) {
            // Explicit Let
            let name = if let Token::Identifier(n) = self.advance().clone() {
                n
            } else {
                return Err("Expected identifier after LET".to_string());
            };
            self.consume(Token::Equal, "Expected '=' after variable name in LET")?;
            let expr = self.parse_expression()?;
            return Ok(Statement::Let(name, expr));
        }
        if self.match_token(Token::If) {
            return self.parse_if();
        }
        if self.match_token(Token::While) {
            return self.parse_while();
        }
        if self.match_token(Token::Do) {
            return self.parse_do_while();
        }
        if self.match_token(Token::For) {
            return self.parse_for();
        }
        if self.match_token(Token::Return) {
            if self.check(Token::Newline) || self.check(Token::EOF) {
                // Basic Return
                return Ok(Statement::Return(None));
            }
            // Return expr
            let expr = self.parse_expression()?;
            return Ok(Statement::Return(Some(expr)));
        }
        if self.match_token(Token::Poke) {
            self.consume(Token::LParen, "Expected '(' after POKE")?;
            let addr = self.parse_expression()?;
            self.consume(Token::Comma, "Expected ',' after POKE address")?;
            let val = self.parse_expression()?;
            self.consume(Token::RParen, "Expected ')' after POKE value")?;
            return Ok(Statement::Poke(addr, val));
        }
        if self.match_token(Token::PlaySfx) {
            // PLAY_SFX(id)
            self.consume(Token::LParen, "Expected '(' after PLAY_SFX")?;
            let id = self.parse_expression()?;
            self.consume(Token::RParen, "Expected ')' after id")?;
            return Ok(Statement::PlaySfx(id));
        }
        if self.match_token(Token::Print) {
            let mut args = Vec::new();
            loop {
                if self.check(Token::Newline) || self.check(Token::EOF) || self.check(Token::Colon)
                {
                    break;
                }
                args.push(self.parse_expression()?);
                if !self.match_token(Token::Comma) {
                    break;
                }
            }
            return Ok(Statement::Print(args));
        }
        if self.match_token(Token::Asm) {
            let mut lines = Vec::new();
            // Consume until END ASM
            while !self.check(Token::End) && !self.is_at_end() {
                // In a real implementation we might want to capture the raw text line
                // For now, we will just skip tokens until newline and store string representation?
                // The lexer might need to support "Raw mode" or we just reconstruct from tokens.
                // For simplicity, let's assume one string per line, composed of tokens.
                // This is a bit hacky because we lose whitespace.
                // Ideally, ASM block should be handled by Lexer differently or we just store tokens.
                // But AST expects Vec<String>.

                // Let's just consume tokens until newline and stringify them
                let mut line = String::new();
                while !self.check(Token::Newline) && !self.is_at_end() {
                    let t = self.advance();
                    match t {
                        Token::Identifier(s) => line.push_str(s),
                        Token::Integer(n) => line.push_str(&n.to_string()),
                        Token::StringLiteral(s) => line.push_str(&format!("\"{}\"", s)),
                        Token::Comma => line.push(','),
                        Token::Hash => line.push('#'),
                        Token::Colon => line.push(':'),
                        Token::SemiColon => line.push(';'),
                        // ... other tokens
                        _ => line.push('?'), // simplified
                    }
                    line.push(' ');
                }
                lines.push(line.trim().to_string());
                if !self.is_at_end() {
                    self.advance(); // consume newline
                }
            }
            self.consume(Token::End, "Expected END after ASM block")?;
            self.consume(Token::Asm, "Expected ASM after END")?;
            return Ok(Statement::Asm(lines));
        }
        if self.match_token(Token::Call) {
            if let Token::Identifier(name) = self.advance().clone() {
                self.consume(Token::LParen, "Expected '(' after CALL name")?;
                let mut args = Vec::new();
                if !self.check(Token::RParen) {
                    loop {
                        args.push(self.parse_expression()?);
                        if !self.match_token(Token::Comma) {
                            break;
                        }
                    }
                }
                self.consume(Token::RParen, "Expected ')' after CALL args")?;
                return Ok(Statement::Call(name, args));
            } else {
                return Err("Expected function name after CALL".to_string());
            }
        }
        if self.match_token(Token::On) {
            // ON <Vector> DO <Routine>
            // <Vector> is usually NMI or IRQ (Identifiers)
            let vector = if let Token::Identifier(n) = self.advance().clone() {
                n
            } else {
                return Err("Expected vector name (NMI/IRQ) after ON".to_string());
            };
            // Expect DO
            // Note: Token::Do is usually for DO...LOOP, but here it acts as a separator.
            // If the lexer emits Token::Do for "DO", we consume it.
            if !self.match_token(Token::Do) {
                return Err("Expected DO after vector name".to_string());
            }
            // Routine Name
            let routine = if let Token::Identifier(n) = self.advance().clone() {
                n
            } else {
                return Err("Expected routine name after DO".to_string());
            };

            return Ok(Statement::On(vector, routine));
        }
        // Identifier start: Assignment or Label (not supported yet) or Implicit CALL (if we allowed it, but we don't seem to have implicit call syntax in AST?)
        // AST has Let.
        // If it's an identifier, check for =
        if let Token::Identifier(name) = self.peek().clone() {
            self.advance();
            if self.match_token(Token::Equal) {
                let expr = self.parse_expression()?;
                return Ok(Statement::Let(name, expr));
            }
            // Fallback: If not assignment, maybe it is a bare function call?
            // "SwissBASIC" example showed `WaitFrame()`.
            // So if Identifier then LParen, it is a Call statement (implicit).
            // But `Call` variant in Statement is `Call(String, Vec<Expression>)`.
            if self.match_token(Token::LParen) {
                let mut args = Vec::new();
                if !self.check(Token::RParen) {
                    loop {
                        args.push(self.parse_expression()?);
                        if !self.match_token(Token::Comma) {
                            break;
                        }
                    }
                }
                self.consume(Token::RParen, "Expected ')' after arguments")?;
                // We map this to Call statement
                return Ok(Statement::Call(name, args));
            }

            return Err(format!(
                "Unexpected token after identifier {}: {:?}",
                name,
                self.peek()
            ));
        }

        Err(format!("Expected statement, found {:?}", self.peek()))
    }

    fn parse_block(&mut self) -> Result<Vec<Statement>, String> {
        let mut statements = Vec::new();
        // Read statements until we hit a block terminator (END, ELSE, WEND, NEXT, LOOP, UNTIL?)
        while !self.check_block_end() && !self.is_at_end() {
            if self.match_token(Token::Newline) {
                continue;
            }
            statements.push(self.parse_statement()?);
        }
        Ok(statements)
    }

    fn check_block_end(&self) -> bool {
        let t = self.peek();
        matches!(
            t,
            Token::End | Token::Else | Token::Wend | Token::Next | Token::Loop
        )
    }

    fn parse_if(&mut self) -> Result<Statement, String> {
        let condition = self.parse_expression()?;
        self.consume(Token::Then, "Expected THEN after IF condition")?;
        self.consume(Token::Newline, "Expected newline after THEN")?; // Enforce block style for now?
                                                                      // Or handle single line IF?
                                                                      // Design doc says "Blocks: Uses BEGIN...END or specific terminators like NEXT, WEND, END IF"
                                                                      // Example: IF Joypad.A THEN \n PlaySound() \n END IF

        let then_block = self.parse_block()?;
        let mut else_block = None;

        if self.match_token(Token::Else) {
            self.consume(Token::Newline, "Expected newline after ELSE")?;
            else_block = Some(self.parse_block()?);
        }

        self.consume(Token::End, "Expected END IF")?;
        self.consume(Token::If, "Expected IF after END")?;

        Ok(Statement::If(condition, then_block, else_block))
    }

    fn parse_while(&mut self) -> Result<Statement, String> {
        let condition = self.parse_expression()?;
        self.consume(Token::Newline, "Expected newline after WHILE condition")?;
        let body = self.parse_block()?;
        self.consume(Token::Wend, "Expected WEND")?;
        Ok(Statement::While(condition, body))
    }

    fn parse_do_while(&mut self) -> Result<Statement, String> {
        // Syntax: DO ... LOOP WHILE <expr>
        self.consume(Token::Newline, "Expected newline after DO")?;
        let body = self.parse_block()?;

        self.consume(Token::Loop, "Expected LOOP after DO block")?;
        self.consume(Token::While, "Expected WHILE after LOOP")?;

        let condition = self.parse_expression()?;
        // Optional newline

        Ok(Statement::DoWhile(body, condition))
    }

    fn parse_for(&mut self) -> Result<Statement, String> {
        // FOR i = 0 TO 10 STEP 1
        let var_name = if let Token::Identifier(name) = self.advance().clone() {
            name
        } else {
            return Err("Expected variable name after FOR".to_string());
        };

        self.consume(Token::Equal, "Expected '=' after FOR variable")?;
        let start_expr = self.parse_expression()?;

        self.consume(Token::To, "Expected TO after start expression")?;
        let end_expr = self.parse_expression()?;

        let mut step_expr = None;
        if self.match_token(Token::Step) {
            step_expr = Some(self.parse_expression()?);
        }

        self.consume(Token::Newline, "Expected newline after FOR definition")?;

        let body = self.parse_block()?;

        self.consume(Token::Next, "Expected NEXT")?;
        // Optional: consume variable name after NEXT
        if let Token::Identifier(next_var) = self.peek() {
            if *next_var == var_name {
                self.advance();
            }
        }

        Ok(Statement::For(
            var_name, start_expr, end_expr, step_expr, body,
        ))
    }

    // --- Expression Parsing ---

    pub fn parse_expression(&mut self) -> Result<Expression, String> {
        self.parse_precedence(Precedence::Or)
    }

    fn parse_precedence(&mut self, precedence: Precedence) -> Result<Expression, String> {
        let mut left = self.parse_unary()?;

        while precedence <= self.get_precedence(self.peek()) {
            let op = self.advance().clone();
            let binary_op = self
                .token_to_binary_op(&op)
                .ok_or("Expected binary operator")?;
            let next_precedence = self.get_next_precedence(&op);
            let right = self.parse_precedence(next_precedence)?;
            left = Expression::BinaryOp(Box::new(left), binary_op, Box::new(right));
        }

        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expression, String> {
        let token = self.peek().clone();
        match token {
            Token::Minus => {
                self.advance();
                let expr = self.parse_precedence(Precedence::Unary)?;
                Ok(Expression::UnaryOp(UnaryOperator::Negate, Box::new(expr)))
            }
            Token::Not => {
                self.advance();
                let expr = self.parse_precedence(Precedence::Unary)?;
                Ok(Expression::UnaryOp(UnaryOperator::Not, Box::new(expr)))
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Result<Expression, String> {
        let token = self.advance().clone();
        match token {
            Token::Integer(val) => Ok(Expression::Integer(val)),
            Token::StringLiteral(val) => Ok(Expression::StringLiteral(val)),
            Token::Identifier(name) => {
                // Check for function call or array access (which looks like function call)
                if self.check(Token::LParen) {
                    self.advance(); // consume '('
                    let mut args = Vec::new();
                    if !self.check(Token::RParen) {
                        loop {
                            args.push(self.parse_expression()?);
                            if !self.match_token(Token::Comma) {
                                break;
                            }
                        }
                    }
                    self.consume(Token::RParen, "Expected ')' after arguments")?;
                    Ok(Expression::FunctionCall(name, args))
                } else {
                    Ok(Expression::Identifier(name))
                }
            }
            Token::Peek => {
                self.consume(Token::LParen, "Expected '(' after PEEK")?;
                let expr = self.parse_expression()?;
                self.consume(Token::RParen, "Expected ')' after PEEK address")?;
                Ok(Expression::Peek(Box::new(expr)))
            }
            Token::LParen => {
                let expr = self.parse_expression()?;
                self.consume(Token::RParen, "Expected ')' after expression")?;
                Ok(expr)
            }
            _ => Err(format!("Expected expression, found {:?}", token)),
        }
    }

    fn get_precedence(&self, token: &Token) -> Precedence {
        match token {
            Token::Or => Precedence::Or,
            Token::And => Precedence::And,
            Token::Equal | Token::NotEqual => Precedence::Equality,
            Token::Less | Token::Greater | Token::LessEqual | Token::GreaterEqual => {
                Precedence::Comparison
            }
            Token::Plus | Token::Minus => Precedence::Term,
            Token::Star | Token::Slash => Precedence::Factor,
            _ => Precedence::None,
        }
    }

    // Associativity: Left-associative for all current binary ops
    // So we usually just use precedence + 1 for the right side
    fn get_next_precedence(&self, token: &Token) -> Precedence {
        let p = self.get_precedence(token);
        match p {
            Precedence::Or => Precedence::And,
            Precedence::And => Precedence::Equality,
            Precedence::Equality => Precedence::Comparison,
            Precedence::Comparison => Precedence::Term,
            Precedence::Term => Precedence::Factor,
            Precedence::Factor => Precedence::Unary,
            Precedence::Unary => Precedence::Call,
            Precedence::Call => Precedence::Primary,
            Precedence::Primary => Precedence::Primary, // Top
            Precedence::None => Precedence::None,
        }
    }

    fn token_to_binary_op(&self, token: &Token) -> Option<BinaryOperator> {
        match token {
            Token::Plus => Some(BinaryOperator::Add),
            Token::Minus => Some(BinaryOperator::Subtract),
            Token::Star => Some(BinaryOperator::Multiply),
            Token::Slash => Some(BinaryOperator::Divide),
            Token::Equal => Some(BinaryOperator::Equal),
            Token::NotEqual => Some(BinaryOperator::NotEqual),
            Token::Less => Some(BinaryOperator::LessThan),
            Token::Greater => Some(BinaryOperator::GreaterThan),
            Token::LessEqual => Some(BinaryOperator::LessThanOrEqual),
            Token::GreaterEqual => Some(BinaryOperator::GreaterThanOrEqual),
            Token::And => Some(BinaryOperator::And),
            Token::Or => Some(BinaryOperator::Or),
            _ => None,
        }
    }

    // Helper methods
    fn peek(&self) -> &Token {
        if self.position >= self.tokens.len() {
            return &Token::EOF;
        }
        &self.tokens[self.position]
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.position += 1;
        }
        self.previous()
    }

    fn previous(&self) -> &Token {
        if self.position == 0 {
            return &Token::EOF;
        }
        &self.tokens[self.position - 1]
    }

    fn is_at_end(&self) -> bool {
        self.peek() == &Token::EOF
    }

    fn check(&self, token: Token) -> bool {
        if self.is_at_end() {
            return false;
        }
        *self.peek() == token
    }

    fn match_token(&mut self, token: Token) -> bool {
        if self.check(token) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn consume(&mut self, token: Token, message: &str) -> Result<&Token, String> {
        if self.check(token) {
            Ok(self.advance())
        } else {
            Err(format!("{} Found: {:?}", message, self.peek()))
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::lexer::tokenize;

    #[test]
    fn test_parse_binary_op() {
        let input = "1 + 2 * 3";
        let tokens = tokenize(input);
        let mut parser = Parser::new(tokens);
        let expr = parser
            .parse_expression()
            .expect("Failed to parse expression");

        // Expected: 1 + (2 * 3) -> Add(1, Multiply(2, 3))
        match expr {
            Expression::BinaryOp(left, op, right) => {
                assert_eq!(*left, Expression::Integer(1));
                assert_eq!(op, BinaryOperator::Add);
                match *right {
                    Expression::BinaryOp(l2, op2, r2) => {
                        assert_eq!(*l2, Expression::Integer(2));
                        assert_eq!(op2, BinaryOperator::Multiply);
                        assert_eq!(*r2, Expression::Integer(3));
                    }
                    _ => panic!("Expected Multiply on right"),
                }
            }
            _ => panic!("Expected Add at top level"),
        }
    }

    #[test]
    fn test_parse_parentheses() {
        let input = "(1 + 2) * 3";
        let tokens = tokenize(input);
        let mut parser = Parser::new(tokens);
        let expr = parser
            .parse_expression()
            .expect("Failed to parse expression");

        // Expected: (1 + 2) * 3 -> Multiply(Add(1, 2), 3)
        match expr {
            Expression::BinaryOp(left, op, right) => {
                assert_eq!(op, BinaryOperator::Multiply);
                assert_eq!(*right, Expression::Integer(3));
                match *left {
                    Expression::BinaryOp(l2, op2, r2) => {
                        assert_eq!(*l2, Expression::Integer(1));
                        assert_eq!(op2, BinaryOperator::Add);
                        assert_eq!(*r2, Expression::Integer(2));
                    }
                    _ => panic!("Expected Add on left"),
                }
            }
            _ => panic!("Expected Multiply at top level"),
        }
    }

    #[test]
    fn test_parse_function_call() {
        let input = "MyFunc(1, 2 + 3)";
        let tokens = tokenize(input);
        let mut parser = Parser::new(tokens);
        let expr = parser
            .parse_expression()
            .expect("Failed to parse expression");

        if let Expression::FunctionCall(name, args) = expr {
            assert_eq!(name, "MyFunc");
            assert_eq!(args.len(), 2);
            assert_eq!(args[0], Expression::Integer(1));
            // args[1] should be binary op 2+3
        } else {
            panic!("Expected FunctionCall");
        }
    }

    #[test]
    fn test_parse_peek() {
        let input = "PEEK($2002)";
        let tokens = tokenize(input);
        let mut parser = Parser::new(tokens);
        let expr = parser
            .parse_expression()
            .expect("Failed to parse expression");

        if let Expression::Peek(addr) = expr {
            assert_eq!(*addr, Expression::Integer(8194));
        } else {
            panic!("Expected Peek");
        }
    }

    #[test]
    fn test_parse_if_statement() {
        let input = "IF x = 1 THEN\n y = 2\nEND IF";
        let tokens = tokenize(input);
        let mut parser = Parser::new(tokens);
        let stmt = parser
            .parse_statement()
            .expect("Failed to parse IF statement");

        if let Statement::If(cond, then_block, else_block) = stmt {
            // Check condition: x = 1
            if let Expression::BinaryOp(_left, op, _right) = cond {
                assert_eq!(op, BinaryOperator::Equal);
            } else {
                panic!("Expected BinaryOp in IF condition");
            }
            // Check body
            assert_eq!(then_block.len(), 1);
            if let Statement::Let(name, _val) = &then_block[0] {
                assert_eq!(name, "y");
            } else {
                panic!("Expected Let in THEN block");
            }
            assert!(else_block.is_none());
        } else {
            panic!("Expected If Statement");
        }
    }

    #[test]
    fn test_parse_while_statement() {
        let input = "WHILE 1\n MySub()\nWEND";
        let tokens = tokenize(input);
        let mut parser = Parser::new(tokens);
        let stmt = parser
            .parse_statement()
            .expect("Failed to parse WHILE statement");

        if let Statement::While(cond, body) = stmt {
            assert_eq!(cond, Expression::Integer(1));
            assert_eq!(body.len(), 1);
        } else {
            panic!("Expected While Statement");
        }
    }

    #[test]
    fn test_parse_for_statement() {
        let input = "FOR i = 0 TO 10 STEP 2\n PRINT i\nNEXT i";
        let tokens = tokenize(input);
        let mut parser = Parser::new(tokens);
        let stmt = parser
            .parse_statement()
            .expect("Failed to parse FOR statement");

        if let Statement::For(var, start, end, step, body) = stmt {
            assert_eq!(var, "i");
            assert_eq!(start, Expression::Integer(0));
            assert_eq!(end, Expression::Integer(10));
            assert_eq!(step, Some(Expression::Integer(2)));
            assert_eq!(body.len(), 1);
        } else {
            panic!("Expected For Statement");
        }
    }

    #[test]
    fn test_parse_simple_program() {
        let input = r#"
CONST MyConst = 10
SUB Main()
  x = MyConst
END SUB
"#;
        let tokens = tokenize(input);
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().expect("Failed to parse program");

        assert_eq!(program.declarations.len(), 2);

        // Check Const
        if let TopLevel::Const(name, val) = &program.declarations[0] {
            assert_eq!(name, "MyConst");
            assert_eq!(*val, Expression::Integer(10));
        } else {
            panic!("Expected Const");
        }

        // Check Sub
        if let TopLevel::Sub(name, params, body) = &program.declarations[1] {
            assert_eq!(name, "Main");
            assert!(params.is_empty());
            assert_eq!(body.len(), 1);
            if let Statement::Let(var, expr) = &body[0] {
                assert_eq!(var, "x");
                // Expression is Identifier(MyConst)
                if let Expression::Identifier(idname) = expr {
                    assert_eq!(idname, "MyConst");
                } else {
                    panic!("Expected Identifier in assignment");
                }
            } else {
                panic!("Expected Let statement");
            }
        } else {
            panic!("Expected Sub");
        }
    }

    #[test]
    fn test_parse_dim() {
        let input = "DIM x AS BYTE";
        let tokens = tokenize(input);
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().expect("Failed to parse program");

        if let TopLevel::Dim(name, dtype) = &program.declarations[0] {
            assert_eq!(name, "x");
            assert_eq!(*dtype, DataType::Byte);
        } else {
            panic!("Expected Dim");
        }
    }

    #[test]
    fn test_parse_sub_with_params() {
        let input = r#"
SUB MyFunc(x AS BYTE, y AS WORD)
  RETURN
END SUB
"#;
        let tokens = tokenize(input);
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().expect("Failed to parse program");

        if let TopLevel::Sub(name, params, _) = &program.declarations[0] {
            assert_eq!(name, "MyFunc");
            assert_eq!(params.len(), 2);
            assert_eq!(params[0].0, "x");
            assert_eq!(params[0].1, DataType::Byte);
            assert_eq!(params[1].0, "y");
            assert_eq!(params[1].1, DataType::Word);
        } else {
            panic!("Expected Sub");
        }
    }
}
