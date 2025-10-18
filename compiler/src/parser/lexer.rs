/// Lexer for Cem
///
/// Tokenizes Cem source code into a stream of tokens.
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    IntLiteral,
    StringLiteral,
    BoolLiteral,

    // Keywords
    Type,  // type
    Colon, // :
    Pipe,  // |
    Match, // match
    End,   // end
    If,    // if
    Arrow, // =>

    // Delimiters
    LeftParen,    // (
    RightParen,   // )
    LeftBracket,  // [
    RightBracket, // ]
    Dash,         // --

    // Identifier (word name, type name, variant name)
    Ident,

    // End of file
    Eof,

    // Comments (ignored)
    Comment,
}

pub struct Lexer {
    input: Vec<char>,
    position: usize,
    line: usize,
    column: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Lexer {
            input: input.chars().collect(),
            position: 0,
            line: 1,
            column: 1,
        }
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace_and_comments();

        if self.is_at_end() {
            return self.make_token(TokenKind::Eof, "");
        }

        let start_line = self.line;
        let start_column = self.column;
        let c = self.peek();

        // Single-character tokens
        match c {
            '(' => {
                self.advance();
                return Token {
                    kind: TokenKind::LeftParen,
                    lexeme: "(".to_string(),
                    line: start_line,
                    column: start_column,
                };
            }
            ')' => {
                self.advance();
                return Token {
                    kind: TokenKind::RightParen,
                    lexeme: ")".to_string(),
                    line: start_line,
                    column: start_column,
                };
            }
            '[' => {
                self.advance();
                return Token {
                    kind: TokenKind::LeftBracket,
                    lexeme: "[".to_string(),
                    line: start_line,
                    column: start_column,
                };
            }
            ']' => {
                self.advance();
                return Token {
                    kind: TokenKind::RightBracket,
                    lexeme: "]".to_string(),
                    line: start_line,
                    column: start_column,
                };
            }
            ':' => {
                self.advance();
                return Token {
                    kind: TokenKind::Colon,
                    lexeme: ":".to_string(),
                    line: start_line,
                    column: start_column,
                };
            }
            '|' => {
                self.advance();
                return Token {
                    kind: TokenKind::Pipe,
                    lexeme: "|".to_string(),
                    line: start_line,
                    column: start_column,
                };
            }
            '-' => {
                // Check for --, negative number, or dash in identifier
                if self.peek_next() == Some('-') {
                    // It's --
                    self.advance();
                    self.advance();
                    return Token {
                        kind: TokenKind::Dash,
                        lexeme: "--".to_string(),
                        line: start_line,
                        column: start_column,
                    };
                } else if self.peek_next().is_some_and(|c| c.is_ascii_digit()) {
                    // It's a negative number
                    return self.number_literal();
                } else {
                    // It's part of an identifier/operator
                    return self.identifier_or_keyword();
                }
            }
            '=' => {
                self.advance();
                if self.peek() == '>' {
                    self.advance();
                    return Token {
                        kind: TokenKind::Arrow,
                        lexeme: "=>".to_string(),
                        line: start_line,
                        column: start_column,
                    };
                }
                // Just '=' is an identifier (the equals word)
                return Token {
                    kind: TokenKind::Ident,
                    lexeme: "=".to_string(),
                    line: start_line,
                    column: start_column,
                };
            }
            '"' => return self.string_literal(),
            _ => {
                if c.is_ascii_digit()
                    || (c == '-' && self.peek_next().is_some_and(|n| n.is_ascii_digit()))
                {
                    return self.number_literal();
                } else if c.is_alphabetic() || c == '_' || is_operator_char(c) {
                    return self.identifier_or_keyword();
                }
            }
        }

        // Unknown character
        self.advance();
        Token {
            kind: TokenKind::Ident,
            lexeme: c.to_string(),
            line: start_line,
            column: start_column,
        }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token();
            let is_eof = token.kind == TokenKind::Eof;
            tokens.push(token);
            if is_eof {
                break;
            }
        }
        tokens
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            if self.is_at_end() {
                return;
            }

            match self.peek() {
                ' ' | '\t' | '\r' => {
                    self.advance();
                }
                '\n' => {
                    self.advance();
                    self.line += 1;
                    self.column = 0; // Will be incremented to 1 by next advance
                }
                '#' => {
                    // Comment until end of line
                    while !self.is_at_end() && self.peek() != '\n' {
                        self.advance();
                    }
                }
                _ => return,
            }
        }
    }

    fn string_literal(&mut self) -> Token {
        const MAX_STRING_LENGTH: usize = 1_000_000; // 1MB limit

        let start_line = self.line;
        let start_column = self.column;
        self.advance(); // consume opening "

        let mut value = String::new();
        while !self.is_at_end() && self.peek() != '"' {
            // Check string length limit
            if value.len() >= MAX_STRING_LENGTH {
                // Return error token
                return Token {
                    kind: TokenKind::Ident, // Use Ident for errors
                    lexeme: format!(
                        "ERROR: String exceeds maximum length of {} bytes",
                        MAX_STRING_LENGTH
                    ),
                    line: start_line,
                    column: start_column,
                };
            }

            if self.peek() == '\n' {
                // Unterminated string (newline before closing quote)
                return Token {
                    kind: TokenKind::Ident,
                    lexeme: "ERROR: Unterminated string literal (newline)".to_string(),
                    line: start_line,
                    column: start_column,
                };
            }

            if self.peek() == '\\' {
                self.advance();
                if !self.is_at_end() {
                    let escaped = match self.peek() {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        '\\' => '\\',
                        '"' => '"',
                        c => c,
                    };
                    value.push(escaped);
                    self.advance();
                }
            } else {
                value.push(self.peek());
                self.advance();
            }
        }

        if self.is_at_end() {
            // Unterminated string (EOF before closing quote)
            return Token {
                kind: TokenKind::Ident,
                lexeme: "ERROR: Unterminated string literal (EOF)".to_string(),
                line: start_line,
                column: start_column,
            };
        }

        self.advance(); // consume closing "

        Token {
            kind: TokenKind::StringLiteral,
            lexeme: value,
            line: start_line,
            column: start_column,
        }
    }

    fn number_literal(&mut self) -> Token {
        let start_line = self.line;
        let start_column = self.column;
        let mut value = String::new();

        // Handle negative sign
        if self.peek() == '-' {
            value.push('-');
            self.advance();
        }

        while !self.is_at_end() && self.peek().is_ascii_digit() {
            value.push(self.peek());
            self.advance();
        }

        Token {
            kind: TokenKind::IntLiteral,
            lexeme: value,
            line: start_line,
            column: start_column,
        }
    }

    fn identifier_or_keyword(&mut self) -> Token {
        let start_line = self.line;
        let start_column = self.column;
        let mut value = String::new();

        while !self.is_at_end() {
            let c = self.peek();
            if c.is_alphanumeric() || c == '_' || c == '-' || is_operator_char(c) {
                value.push(c);
                self.advance();
            } else {
                break;
            }
        }

        let kind = match value.as_str() {
            "type" => TokenKind::Type,
            "match" => TokenKind::Match,
            "end" => TokenKind::End,
            "if" => TokenKind::If,
            "true" | "false" => TokenKind::BoolLiteral,
            _ => TokenKind::Ident,
        };

        Token {
            kind,
            lexeme: value,
            line: start_line,
            column: start_column,
        }
    }

    fn make_token(&self, kind: TokenKind, lexeme: &str) -> Token {
        Token {
            kind,
            lexeme: lexeme.to_string(),
            line: self.line,
            column: self.column,
        }
    }

    fn peek(&self) -> char {
        if self.is_at_end() {
            '\0'
        } else {
            self.input[self.position]
        }
    }

    fn peek_next(&self) -> Option<char> {
        if self.position + 1 < self.input.len() {
            Some(self.input[self.position + 1])
        } else {
            None
        }
    }

    fn advance(&mut self) -> char {
        let c = self.peek();
        self.position += 1;
        self.column += 1;
        c
    }

    fn is_at_end(&self) -> bool {
        self.position >= self.input.len()
    }
}

fn is_operator_char(c: char) -> bool {
    matches!(c, '+' | '-' | '*' | '/' | '<' | '>' | '=' | '!')
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::IntLiteral => write!(f, "INT"),
            TokenKind::StringLiteral => write!(f, "STRING"),
            TokenKind::BoolLiteral => write!(f, "BOOL"),
            TokenKind::Type => write!(f, "type"),
            TokenKind::Colon => write!(f, ":"),
            TokenKind::Pipe => write!(f, "|"),
            TokenKind::Match => write!(f, "match"),
            TokenKind::End => write!(f, "end"),
            TokenKind::If => write!(f, "if"),
            TokenKind::Arrow => write!(f, "=>"),
            TokenKind::LeftParen => write!(f, "("),
            TokenKind::RightParen => write!(f, ")"),
            TokenKind::LeftBracket => write!(f, "["),
            TokenKind::RightBracket => write!(f, "]"),
            TokenKind::Dash => write!(f, "--"),
            TokenKind::Ident => write!(f, "IDENT"),
            TokenKind::Eof => write!(f, "EOF"),
            TokenKind::Comment => write!(f, "COMMENT"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tokens() {
        let mut lexer = Lexer::new(": square ( Int -- Int ) dup * ;");
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::Colon);
        assert_eq!(tokens[1].kind, TokenKind::Ident);
        assert_eq!(tokens[1].lexeme, "square");
        assert_eq!(tokens[2].kind, TokenKind::LeftParen);
        assert_eq!(tokens[3].kind, TokenKind::Ident);
        assert_eq!(tokens[3].lexeme, "Int");
    }

    #[test]
    fn test_numbers() {
        let mut lexer = Lexer::new("42 -17 0");
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::IntLiteral);
        assert_eq!(tokens[0].lexeme, "42");
        assert_eq!(tokens[1].kind, TokenKind::IntLiteral);
        assert_eq!(tokens[1].lexeme, "-17");
        assert_eq!(tokens[2].kind, TokenKind::IntLiteral);
        assert_eq!(tokens[2].lexeme, "0");
    }

    #[test]
    fn test_strings() {
        let mut lexer = Lexer::new(r#""hello" "world\n""#);
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::StringLiteral);
        assert_eq!(tokens[0].lexeme, "hello");
        assert_eq!(tokens[1].kind, TokenKind::StringLiteral);
        assert_eq!(tokens[1].lexeme, "world\n");
    }

    #[test]
    fn test_operators() {
        let mut lexer = Lexer::new("+ - * / < > = dup");
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::Ident);
        assert_eq!(tokens[0].lexeme, "+");
        assert_eq!(tokens[7].kind, TokenKind::Ident);
        assert_eq!(tokens[7].lexeme, "dup");
    }

    #[test]
    fn test_comments() {
        let mut lexer = Lexer::new("# comment\n42");
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::IntLiteral);
        assert_eq!(tokens[0].lexeme, "42");
    }

    #[test]
    fn test_unterminated_string_newline() {
        let mut lexer = Lexer::new("\"hello\n");
        let tokens = lexer.tokenize();

        // Should get an error token
        assert!(tokens[0].lexeme.starts_with("ERROR"));
        assert!(tokens[0].lexeme.contains("Unterminated"));
    }

    #[test]
    fn test_unterminated_string_eof() {
        let mut lexer = Lexer::new("\"hello");
        let tokens = lexer.tokenize();

        // Should get an error token
        assert!(tokens[0].lexeme.starts_with("ERROR"));
        assert!(tokens[0].lexeme.contains("Unterminated"));
    }

    #[test]
    fn test_valid_string() {
        let mut lexer = Lexer::new("\"hello world\"");
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::StringLiteral);
        assert_eq!(tokens[0].lexeme, "hello world");
    }

    #[test]
    fn test_newline_handling() {
        let mut lexer = Lexer::new("42\n43\n44");
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::IntLiteral);
        assert_eq!(tokens[0].line, 1);
        assert_eq!(tokens[1].kind, TokenKind::IntLiteral);
        assert_eq!(tokens[1].line, 2);
        assert_eq!(tokens[2].kind, TokenKind::IntLiteral);
        assert_eq!(tokens[2].line, 3);
    }

    #[test]
    fn test_max_string_length() {
        // Create a string that exceeds MAX_STRING_LENGTH (1MB)
        let mut input = String::from("\"");
        // Add 1,000,001 characters (exceeds 1MB limit)
        for _ in 0..1_000_001 {
            input.push('a');
        }
        input.push('"');

        let mut lexer = Lexer::new(&input);
        let tokens = lexer.tokenize();

        // Should get an error token
        assert!(tokens[0].lexeme.starts_with("ERROR"));
        assert!(tokens[0].lexeme.contains("maximum length"));
    }
}
