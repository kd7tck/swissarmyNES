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
            while self.match_token(Token::Newline) {}
            if self.is_at_end() {
                break;
            }
            let decl = self.parse_top_level()?;
            declarations.push(decl);
        }

        Ok(Program { declarations })
    }

    fn parse_top_level(&mut self) -> Result<TopLevel, String> {
        if self.match_token(Token::Include) {
            let filename = if let Token::StringLiteral(s) = self.advance().clone() {
                s
            } else {
                return Err("Expected string literal after INCLUDE".to_string());
            };
            self.match_token(Token::Newline);
            return Ok(TopLevel::Include(filename));
        }

        if self.match_token(Token::Const) {
            let name = if let Token::Identifier(n) = self.advance().clone() {
                n
            } else {
                return Err("Expected identifier after CONST".to_string());
            };
            self.consume(Token::Equal, "Expected '=' in CONST declaration")?;
            let val = self.parse_expression()?;
            self.match_token(Token::Newline);
            return Ok(TopLevel::Const(name, val));
        }

        if self.match_token(Token::Dim) {
            let name = if let Token::Identifier(n) = self.advance().clone() {
                n
            } else {
                return Err("Expected identifier after DIM".to_string());
            };

            // Check for Array Size: DIM x(10) AS BYTE
            let mut array_size = None;
            if self.match_token(Token::LParen) {
                let size_expr = self.parse_expression()?;
                if let Expression::Integer(val) = size_expr {
                    if val <= 0 {
                        return Err("Array size must be positive".to_string());
                    }
                    array_size = Some(val as usize);
                } else {
                    return Err("Array size must be an integer literal".to_string());
                }
                self.consume(Token::RParen, "Expected ')' after array size")?;
            }

            self.consume(Token::As, "Expected AS after DIM name")?;
            let mut data_type = self.parse_type()?;

            if let Some(size) = array_size {
                data_type = DataType::Array(Box::new(data_type), size);
            }

            let mut init_expr = None;
            if self.match_token(Token::Equal) {
                init_expr = Some(self.parse_expression()?);
            }

            self.match_token(Token::Newline);
            return Ok(TopLevel::Dim(name, data_type, init_expr));
        }

        if self.match_token(Token::Type) {
            let name = if let Token::Identifier(n) = self.advance().clone() {
                n
            } else {
                return Err("Expected identifier after TYPE".to_string());
            };
            self.consume(Token::Newline, "Expected newline after TYPE name")?;

            let mut members = Vec::new();
            while !self.check(Token::End) && !self.is_at_end() {
                if self.match_token(Token::Newline) {
                    continue;
                }

                let member_name = if let Token::Identifier(n) = self.advance().clone() {
                    n
                } else {
                    return Err("Expected member name in TYPE definition".to_string());
                };

                // Check for array member: member(10) AS Type
                let mut array_size = None;
                if self.match_token(Token::LParen) {
                    let size_expr = self.parse_expression()?;
                    if let Expression::Integer(val) = size_expr {
                        if val <= 0 {
                            return Err("Array size must be positive".to_string());
                        }
                        array_size = Some(val as usize);
                    } else {
                        return Err("Array size must be an integer literal".to_string());
                    }
                    self.consume(Token::RParen, "Expected ')' after array size")?;
                }

                self.consume(Token::As, "Expected AS after member name")?;
                let mut member_type = self.parse_type()?;

                if let Some(size) = array_size {
                    member_type = DataType::Array(Box::new(member_type), size);
                }

                members.push((member_name, member_type));

                self.consume(Token::Newline, "Expected newline after member definition")?;
            }

            self.consume(Token::End, "Expected END TYPE")?;
            self.consume(Token::Type, "Expected TYPE after END")?;

            return Ok(TopLevel::TypeDecl(name, members));
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

                    self.consume(Token::As, "Expected AS after parameter name")?;
                    let param_type = self.parse_type()?;
                    // We don't support array parameters yet (by ref/val issues), keep simple
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

        let mut data_label = None;
        if let Token::Identifier(name) = self.peek().clone() {
            if self.position + 1 < self.tokens.len()
                && self.tokens[self.position + 1] == Token::Colon
                && self.position + 2 < self.tokens.len()
                && self.tokens[self.position + 2] == Token::Data
            {
                self.advance();
                self.advance();
                data_label = Some(name);
            }
        }

        if data_label.is_some() || self.check(Token::Data) {
            self.consume(Token::Data, "Expected DATA keyword")?;
            let mut exprs = Vec::new();
            loop {
                exprs.push(self.parse_expression()?);
                if !self.match_token(Token::Comma) {
                    break;
                }
            }
            self.match_token(Token::Newline);
            return Ok(TopLevel::Data(data_label, exprs));
        }

        if self.match_token(Token::Asm) {
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
        if self.match_token(Token::Int) {
            return Ok(DataType::Int);
        }
        if self.match_token(Token::Bool) {
            return Ok(DataType::Bool);
        }
        if self.match_token(Token::String) {
            return Ok(DataType::String);
        }
        if let Token::Identifier(name) = self.peek().clone() {
            self.advance();
            return Ok(DataType::Struct(name));
        }
        Err(format!(
            "Expected type (BYTE, WORD, BOOL, STRING, StructName), found {:?}",
            self.peek()
        ))
    }

    fn parse_statement(&mut self) -> Result<Statement, String> {
        if self.match_token(Token::Let) {
            let target = self.parse_precedence(Precedence::Comparison)?;
            self.consume(Token::Equal, "Expected '=' after variable name in LET")?;
            let expr = self.parse_expression()?;
            return Ok(Statement::Let(target, expr));
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
                return Ok(Statement::Return(None));
            }
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
        if self.match_token(Token::Read) {
            let mut vars = Vec::new();
            loop {
                if let Token::Identifier(name) = self.advance().clone() {
                    vars.push(name);
                } else {
                    return Err("Expected variable name after READ".to_string());
                }
                if !self.match_token(Token::Comma) {
                    break;
                }
            }
            return Ok(Statement::Read(vars));
        }
        if self.match_token(Token::Select) {
            return self.parse_select();
        }
        if self.match_token(Token::Restore) {
            let mut label = None;
            if let Token::Identifier(name) = self.peek().clone() {
                self.advance();
                label = Some(name);
            }
            return Ok(Statement::Restore(label));
        }
        if self.match_token(Token::Asm) {
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
            return Ok(Statement::Asm(lines));
        }
        if self.match_token(Token::Call) {
            // CALL Identifier(Args) or CALL Expression?
            // "CALL MyFunc(1, 2)"
            // "CALL player.method(1)"
            // Parser logic: Parse expression.
            let expr = self.parse_expression()?;
            // Check if it's a Call expression
            if let Expression::Call(target, args) = expr {
                return Ok(Statement::Call(*target, args));
            } else if let Expression::Identifier(name) = expr {
                // Call Name (Implicit args empty)
                return Ok(Statement::Call(Expression::Identifier(name), vec![]));
            } else {
                return Err("Expected function call after CALL".to_string());
            }
        }
        if self.match_token(Token::On) {
            let vector = if let Token::Identifier(n) = self.advance().clone() {
                n
            } else {
                return Err("Expected vector name (NMI/IRQ) after ON".to_string());
            };
            if !self.match_token(Token::Do) {
                return Err("Expected DO after vector name".to_string());
            }
            let routine = if let Token::Identifier(n) = self.advance().clone() {
                n
            } else {
                return Err("Expected routine name after DO".to_string());
            };
            return Ok(Statement::On(vector, routine));
        }

        // Implicit Let or Call
        if matches!(self.peek(), Token::Identifier(_)) {
            let expr = self.parse_precedence(Precedence::Comparison)?;

            if self.match_token(Token::Equal) {
                let val = self.parse_expression()?;
                return Ok(Statement::Let(expr, val));
            }

            if let Expression::Call(target, args) = expr {
                return Ok(Statement::Call(*target, args));
            }

            // Allow implicit Call without parens? "MyFunc"
            if let Expression::Identifier(name) = expr {
                return Ok(Statement::Call(Expression::Identifier(name), vec![]));
            }

            return Err(format!(
                "Unexpected expression in statement position: {:?}",
                expr
            ));
        }

        Err(format!("Expected statement, found {:?}", self.peek()))
    }

    fn parse_block(&mut self) -> Result<Vec<Statement>, String> {
        let mut statements = Vec::new();
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
            Token::End | Token::Else | Token::Wend | Token::Next | Token::Loop | Token::Case
        )
    }

    fn parse_if(&mut self) -> Result<Statement, String> {
        let condition = self.parse_expression()?;
        self.consume(Token::Then, "Expected THEN after IF condition")?;
        self.consume(Token::Newline, "Expected newline after THEN")?;
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
        self.consume(Token::Newline, "Expected newline after DO")?;
        let body = self.parse_block()?;
        self.consume(Token::Loop, "Expected LOOP after DO block")?;
        self.consume(Token::While, "Expected WHILE after LOOP")?;
        let condition = self.parse_expression()?;
        Ok(Statement::DoWhile(body, condition))
    }

    fn parse_for(&mut self) -> Result<Statement, String> {
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
        if let Token::Identifier(next_var) = self.peek() {
            if *next_var == var_name {
                self.advance();
            }
        }
        Ok(Statement::For(
            var_name, start_expr, end_expr, step_expr, body,
        ))
    }

    fn parse_select(&mut self) -> Result<Statement, String> {
        self.consume(Token::Case, "Expected CASE after SELECT")?;
        let expr = self.parse_expression()?;
        self.consume(Token::Newline, "Expected newline after SELECT CASE <expr>")?;
        let mut cases = Vec::new();
        let mut case_else = None;
        while !self.check(Token::End) && !self.is_at_end() {
            if self.match_token(Token::Newline) {
                continue;
            }
            if self.match_token(Token::Case) {
                if self.match_token(Token::Else) {
                    self.consume(Token::Newline, "Expected newline after CASE ELSE")?;
                    let block = self.parse_block()?;
                    case_else = Some(block);
                } else {
                    let val = self.parse_expression()?;
                    self.consume(Token::Newline, "Expected newline after CASE <val>")?;
                    let block = self.parse_block()?;
                    cases.push((val, block));
                }
            } else {
                return Err(format!(
                    "Expected CASE or END SELECT, found {:?}",
                    self.peek()
                ));
            }
        }
        self.consume(Token::End, "Expected END SELECT")?;
        self.consume(Token::Select, "Expected SELECT after END")?;
        Ok(Statement::Select(expr, cases, case_else))
    }

    pub fn parse_expression(&mut self) -> Result<Expression, String> {
        self.parse_precedence(Precedence::Or)
    }

    fn parse_precedence(&mut self, precedence: Precedence) -> Result<Expression, String> {
        let mut left = self.parse_unary()?;

        while precedence <= self.get_precedence(self.peek()) {
            let op = self.peek().clone(); // Peek first

            if op == Token::Dot {
                self.advance();
                let member = if let Token::Identifier(n) = self.advance().clone() {
                    n
                } else {
                    return Err("Expected member name after '.'".to_string());
                };
                left = Expression::MemberAccess(Box::new(left), member);
            } else if op == Token::LParen {
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
                left = Expression::Call(Box::new(left), args);
            } else {
                self.advance();
                let binary_op = self
                    .token_to_binary_op(&op)
                    .ok_or("Expected binary operator")?;
                let next_precedence = self.get_next_precedence(&op);
                let right = self.parse_precedence(next_precedence)?;
                left = Expression::BinaryOp(Box::new(left), binary_op, Box::new(right));
            }
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
            Token::Identifier(name) => Ok(Expression::Identifier(name)),
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
            Token::Star | Token::Slash | Token::Mod => Precedence::Factor,
            Token::Dot => Precedence::Call,
            Token::LParen => Precedence::Call,
            _ => Precedence::None,
        }
    }

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
            Precedence::Primary => Precedence::Primary,
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
            Token::Mod => Some(BinaryOperator::Modulo),
            _ => None,
        }
    }

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
    fn test_parse_function_call() {
        let input = "MyFunc(1, 2 + 3)";
        let tokens = tokenize(input);
        let mut parser = Parser::new(tokens);
        let expr = parser
            .parse_expression()
            .expect("Failed to parse expression");

        if let Expression::Call(target, args) = expr {
            if let Expression::Identifier(name) = *target {
                assert_eq!(name, "MyFunc");
            } else {
                panic!("Expected identifier target");
            }
            assert_eq!(args.len(), 2);
            assert_eq!(args[0], Expression::Integer(1));
        } else {
            panic!("Expected Call");
        }
    }

    #[test]
    fn test_parse_array_declaration() {
        let input = "DIM x(10) AS BYTE";
        let tokens = tokenize(input);
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().expect("Failed to parse program");

        if let TopLevel::Dim(name, dtype, _) = &program.declarations[0] {
            assert_eq!(name, "x");
            if let DataType::Array(inner, size) = dtype {
                assert_eq!(**inner, DataType::Byte);
                assert_eq!(*size, 10);
            } else {
                panic!("Expected Array type");
            }
        } else {
            panic!("Expected Dim");
        }
    }
}
