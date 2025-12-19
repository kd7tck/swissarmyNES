#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    // Keywords
    Rem,
    Begin,
    End,
    Next,
    Wend,
    If,
    Then,
    Else,
    Sub,
    Interrupt,
    Asm,
    On,
    Type, // TYPE StructName
    As,   // Added for DIM Name AS Type
    Do,
    While,
    For,
    To,
    Step,
    Loop, // Added for DO...LOOP
    Const,
    Dim,
    Byte,
    Word,
    Int,
    Bool,
    String,
    Peek,
    Poke,
    Print,
    Return,
    Call,
    And,
    Or,
    Not,
    Let,
    PlaySfx,
    Data,
    Read,
    Restore,
    Include,
    Select,
    Case,

    // Identifiers
    Identifier(String),

    // Literals
    Integer(i32),
    StringLiteral(String),

    // Operators & Symbols
    Plus,         // +
    Minus,        // -
    Star,         // *
    Slash,        // /
    Equal,        // =
    Less,         // <
    Greater,      // >
    LessEqual,    // <=
    GreaterEqual, // >=
    NotEqual,     // <>
    LParen,       // (
    RParen,       // )
    Comma,        // ,
    Colon,        // :
    SemiColon,    // ;
    Dot,          // .
    Hash,         // #

    // Delimiters
    Newline,
    EOF,

    // Unknown/Error
    Illegal(String),
}

pub struct Lexer<'a> {
    input: std::iter::Peekable<std::str::Chars<'a>>,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input: input.chars().peekable(),
        }
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace();

        match self.input.peek() {
            Some(&ch) => {
                match ch {
                    '\n' => {
                        self.input.next();
                        Token::Newline
                    }
                    '+' => {
                        self.input.next();
                        Token::Plus
                    }
                    '-' => {
                        self.input.next();
                        Token::Minus
                    }
                    '*' => {
                        self.input.next();
                        Token::Star
                    }
                    '/' => {
                        self.input.next();
                        Token::Slash
                    }
                    '=' => {
                        self.input.next();
                        Token::Equal
                    }
                    '<' => {
                        self.input.next();
                        if let Some(&'=') = self.input.peek() {
                            self.input.next();
                            Token::LessEqual
                        } else if let Some(&'>') = self.input.peek() {
                            self.input.next();
                            Token::NotEqual
                        } else {
                            Token::Less
                        }
                    }
                    '>' => {
                        self.input.next();
                        if let Some(&'=') = self.input.peek() {
                            self.input.next();
                            Token::GreaterEqual
                        } else {
                            Token::Greater
                        }
                    }
                    '(' => {
                        self.input.next();
                        Token::LParen
                    }
                    ')' => {
                        self.input.next();
                        Token::RParen
                    }
                    ',' => {
                        self.input.next();
                        Token::Comma
                    }
                    ':' => {
                        self.input.next();
                        Token::Colon
                    }
                    ';' => {
                        self.input.next();
                        Token::SemiColon
                    }
                    '.' => {
                        self.input.next();
                        Token::Dot
                    }
                    '#' => {
                        self.input.next();
                        Token::Hash
                    }
                    '\'' => {
                        // Comment
                        self.read_comment();
                        self.next_token() // Skip comment and return next token (likely Newline)
                    }
                    '$' => {
                        // Hex literal
                        self.input.next();
                        self.read_hex_number()
                    }
                    '%' => {
                        // Binary literal
                        self.input.next();
                        self.read_binary_number()
                    }
                    '"' => {
                        // String literal
                        self.read_string()
                    }
                    _ => {
                        if ch.is_ascii_digit() {
                            self.read_number()
                        } else if is_letter(ch) {
                            self.read_identifier()
                        } else {
                            self.input.next();
                            Token::Illegal(ch.to_string())
                        }
                    }
                }
            }
            None => Token::EOF,
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(&ch) = self.input.peek() {
            if ch.is_whitespace() && ch != '\n' {
                self.input.next();
            } else {
                break;
            }
        }
    }

    fn read_comment(&mut self) {
        // Consumes until newline or EOF
        while let Some(&ch) = self.input.peek() {
            if ch == '\n' {
                break;
            }
            self.input.next();
        }
    }

    fn read_identifier(&mut self) -> Token {
        let mut ident = String::new();
        while let Some(&ch) = self.input.peek() {
            if is_letter(ch) || ch.is_ascii_digit() || ch == '_' {
                ident.push(ch);
                self.input.next();
            } else {
                break;
            }
        }

        // Check for keywords (case-insensitive)
        match ident.to_uppercase().as_str() {
            "REM" => {
                self.read_comment();
                self.next_token()
            }
            "BEGIN" => Token::Begin,
            "END" => Token::End,
            "NEXT" => Token::Next,
            "WEND" => Token::Wend,
            "IF" => Token::If,
            "THEN" => Token::Then,
            "ELSE" => Token::Else,
            "SUB" => Token::Sub,
            "INTERRUPT" => Token::Interrupt,
            "ASM" => Token::Asm,
            "ON" => Token::On,
            "TYPE" => Token::Type,
            "AS" => Token::As,
            "DO" => Token::Do,
            "WHILE" => Token::While,
            "FOR" => Token::For,
            "TO" => Token::To,
            "STEP" => Token::Step,
            "LOOP" => Token::Loop,
            "CONST" => Token::Const,
            "DIM" => Token::Dim,
            "BYTE" => Token::Byte,
            "WORD" => Token::Word,
            "INT" => Token::Int,
            "BOOL" => Token::Bool,
            "STRING" => Token::String,
            "PEEK" => Token::Peek,
            "POKE" => Token::Poke,
            "PRINT" => Token::Print,
            "RETURN" => Token::Return,
            "CALL" => Token::Call,
            "AND" => Token::And,
            "OR" => Token::Or,
            "NOT" => Token::Not,
            "LET" => Token::Let,
            "PLAY_SFX" => Token::PlaySfx,
            "DATA" => Token::Data,
            "READ" => Token::Read,
            "RESTORE" => Token::Restore,
            "INCLUDE" => Token::Include,
            "SELECT" => Token::Select,
            "CASE" => Token::Case,
            _ => Token::Identifier(ident),
        }
    }

    fn read_number(&mut self) -> Token {
        let mut num_str = String::new();
        while let Some(&ch) = self.input.peek() {
            if ch.is_ascii_digit() {
                num_str.push(ch);
                self.input.next();
            } else {
                break;
            }
        }

        match num_str.parse::<i32>() {
            Ok(n) => Token::Integer(n),
            Err(_) => Token::Illegal(num_str),
        }
    }

    fn read_hex_number(&mut self) -> Token {
        let mut num_str = String::new();
        while let Some(&ch) = self.input.peek() {
            if ch.is_ascii_hexdigit() {
                num_str.push(ch);
                self.input.next();
            } else {
                break;
            }
        }

        if num_str.is_empty() {
            return Token::Illegal("$".to_string());
        }

        match i32::from_str_radix(&num_str, 16) {
            Ok(n) => Token::Integer(n),
            Err(_) => Token::Illegal(format!("${}", num_str)),
        }
    }

    fn read_binary_number(&mut self) -> Token {
        let mut num_str = String::new();
        while let Some(&ch) = self.input.peek() {
            if ch == '0' || ch == '1' {
                num_str.push(ch);
                self.input.next();
            } else {
                break;
            }
        }

        if num_str.is_empty() {
            return Token::Illegal("%".to_string());
        }

        match i32::from_str_radix(&num_str, 2) {
            Ok(n) => Token::Integer(n),
            Err(_) => Token::Illegal(format!("%{}", num_str)),
        }
    }

    fn read_string(&mut self) -> Token {
        self.input.next(); // Skip opening quote
        let mut str_val = String::new();

        while let Some(&ch) = self.input.peek() {
            if ch == '"' {
                self.input.next();
                return Token::StringLiteral(str_val);
            }
            if ch == '\n' || ch == '\r' {
                // String shouldn't span lines in simple BASIC usually, or at least handle it gracefully
                break;
            }
            str_val.push(ch);
            self.input.next();
        }

        Token::Illegal(format!("\"{}", str_val)) // Unterminated string
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token();
            if let Token::Illegal(s) = &token {
                return Err(format!("Illegal token: {}", s));
            }
            if token == Token::EOF {
                tokens.push(token);
                break;
            }
            tokens.push(token);
        }
        Ok(tokens)
    }
}

fn is_letter(ch: char) -> bool {
    ch.is_alphabetic()
}

pub fn tokenize(input: &str) -> Vec<Token> {
    let mut lexer = Lexer::new(input);
    lexer.tokenize().unwrap_or_else(|_| vec![Token::EOF])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operators_delimiters() {
        let input = "+ - * / = < > <= >= <> ( ) , : ; #";
        let tokens = tokenize(input);

        let expected = vec![
            Token::Plus,
            Token::Minus,
            Token::Star,
            Token::Slash,
            Token::Equal,
            Token::Less,
            Token::Greater,
            Token::LessEqual,
            Token::GreaterEqual,
            Token::NotEqual,
            Token::LParen,
            Token::RParen,
            Token::Comma,
            Token::Colon,
            Token::SemiColon,
            Token::Hash,
            Token::EOF,
        ];

        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_keywords() {
        let input = "IF THEN ELSE END SUB WHILE WEND DO CONST DIM AS BYTE WORD INT BOOL PEEK POKE PRINT RETURN CALL AND OR NOT INCLUDE";
        let tokens = tokenize(input);

        let expected = vec![
            Token::If,
            Token::Then,
            Token::Else,
            Token::End,
            Token::Sub,
            Token::While,
            Token::Wend,
            Token::Do,
            Token::Const,
            Token::Dim,
            Token::As,
            Token::Byte,
            Token::Word,
            Token::Int,
            Token::Bool,
            Token::Peek,
            Token::Poke,
            Token::Print,
            Token::Return,
            Token::Call,
            Token::And,
            Token::Or,
            Token::Not,
            Token::Include,
            Token::EOF,
        ];

        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_keywords_case_insensitive() {
        let input = "if Then else";
        let tokens = tokenize(input);

        let expected = vec![Token::If, Token::Then, Token::Else, Token::EOF];

        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_identifiers() {
        let input = "foobar x y123 var_name";
        let tokens = tokenize(input);

        let expected = vec![
            Token::Identifier("foobar".to_string()),
            Token::Identifier("x".to_string()),
            Token::Identifier("y123".to_string()),
            Token::Identifier("var_name".to_string()),
            Token::EOF,
        ];

        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_literals() {
        let input = "123 $FF %1010 \"Hello\"";
        let tokens = tokenize(input);

        let expected = vec![
            Token::Integer(123),
            Token::Integer(255),
            Token::Integer(10),
            Token::StringLiteral("Hello".to_string()),
            Token::EOF,
        ];

        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_comments() {
        let input = "x = 1 ' This is a comment\ny = 2 REM Another comment";
        let tokens = tokenize(input);

        let expected = vec![
            Token::Identifier("x".to_string()),
            Token::Equal,
            Token::Integer(1),
            Token::Newline,
            Token::Identifier("y".to_string()),
            Token::Equal,
            Token::Integer(2),
            Token::EOF,
        ];

        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_example_code() {
        let input = r#"
CONST BG_COLOR = $0F

SUB Main()
    ' Initialize PPU
    POKE(PPU_CTRL, %10000000) ' Enable NMI

    WHILE 1
        WaitFrame()
    WEND
END SUB
"#;
        let tokens = tokenize(input);

        // Basic structure check
        assert_eq!(tokens[0], Token::Newline); // Start with newline
        assert_eq!(tokens[1], Token::Const);
        assert_eq!(tokens[2], Token::Identifier("BG_COLOR".to_string()));
        assert_eq!(tokens[3], Token::Equal);
        assert_eq!(tokens[4], Token::Integer(15));
        // ...
    }
}
