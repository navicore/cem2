/// Recursive descent parser for Cem
use crate::ast::types::{Effect, Type};
use crate::ast::{Expr, MatchBranch, Pattern, Program, TypeDef, Variant, WordDef};
use crate::parser::lexer::{Lexer, Token, TokenKind};
use std::fmt;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Parse error at {}:{}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for ParseError {}

const MAX_NESTING_DEPTH: usize = 100;

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
    nesting_depth: usize,
    /// Arc-wrapped filename to avoid duplication across all SourceLocs
    filename: Arc<str>,
}

impl Parser {
    pub fn new(input: &str) -> Self {
        Self::new_with_filename(input, "<input>")
    }

    pub fn new_with_filename(input: &str, filename: &str) -> Self {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();
        Parser {
            tokens,
            current: 0,
            nesting_depth: 0,
            filename: Arc::from(filename),
        }
    }

    /// Helper: Create SourceLoc from current token
    fn current_loc(&self) -> crate::ast::SourceLoc {
        let token = self.peek();
        crate::ast::SourceLoc::new(token.line, token.column, Arc::clone(&self.filename))
    }

    /// Helper: Create SourceLoc from a specific token
    fn loc_from_token(&self, token: &Token) -> crate::ast::SourceLoc {
        crate::ast::SourceLoc::new(token.line, token.column, Arc::clone(&self.filename))
    }

    pub fn parse(&mut self) -> Result<Program, ParseError> {
        let mut type_defs = Vec::new();
        let mut word_defs = Vec::new();

        while !self.is_at_end() {
            if self.check(&TokenKind::Type) {
                type_defs.push(self.parse_type_def()?);
            } else if self.check(&TokenKind::Colon) {
                word_defs.push(self.parse_word_def()?);
            } else {
                return Err(self.error("Expected 'type' or ':'"));
            }
        }

        Ok(Program {
            type_defs,
            word_defs,
        })
    }

    fn parse_type_def(&mut self) -> Result<TypeDef, ParseError> {
        self.consume(&TokenKind::Type, "Expected 'type'")?;

        let name = self.consume_ident("Expected type name")?;

        // Optional type parameters
        let mut type_params = Vec::new();
        if self.check(&TokenKind::LeftParen) {
            self.advance();
            while !self.check(&TokenKind::RightParen) && !self.is_at_end() {
                type_params.push(self.consume_ident("Expected type parameter")?);
                if self.check(&TokenKind::RightParen) {
                    break;
                }
            }
            self.consume(&TokenKind::RightParen, "Expected ')'")?;
        }

        self.consume(&TokenKind::Pipe, "Expected '|' before first variant")?;

        // Parse variants
        let mut variants = Vec::new();
        loop {
            let variant_name = self.consume_ident("Expected variant name")?;

            // Parse variant fields (optional)
            let mut fields = Vec::new();
            if self.check(&TokenKind::LeftParen) {
                self.advance();
                while !self.check(&TokenKind::RightParen) && !self.is_at_end() {
                    fields.push(self.parse_type()?);

                    // If there's a comma, consume it and continue
                    // If there's no comma, we're done with fields
                    if self.check(&TokenKind::Comma) {
                        self.advance();
                    } else {
                        break;
                    }
                }
                self.consume(&TokenKind::RightParen, "Expected ')'")?;
            }

            variants.push(Variant {
                name: variant_name,
                fields,
            });

            // Check for more variants
            if self.check(&TokenKind::Pipe) {
                self.advance();
            } else {
                break;
            }
        }

        Ok(TypeDef {
            name,
            type_params,
            variants,
        })
    }

    fn parse_word_def(&mut self) -> Result<WordDef, ParseError> {
        let colon_token = self.peek().clone();
        self.consume(&TokenKind::Colon, "Expected ':'")?;

        let name = self.consume_ident("Expected word name")?;

        // Parse effect signature
        self.consume(&TokenKind::LeftParen, "Expected '(' for effect signature")?;
        let effect = self.parse_effect()?;
        self.consume(
            &TokenKind::RightParen,
            "Expected ')' after effect signature",
        )?;

        // Parse body until ';'
        let mut body = Vec::new();
        while !self.check_ident(";") && !self.is_at_end() {
            body.push(self.parse_expr()?);
        }

        self.consume_ident_value(";", "Expected ';' at end of word definition")?;

        Ok(WordDef {
            name,
            effect,
            body,
            loc: self.loc_from_token(&colon_token),
        })
    }

    fn parse_effect(&mut self) -> Result<Effect, ParseError> {
        // Parse input stack types
        let mut inputs = Vec::new();
        while !self.check(&TokenKind::Dash) && !self.is_at_end() {
            inputs.push(self.parse_type()?);
        }

        self.consume(&TokenKind::Dash, "Expected '--' in effect signature")?;

        // Parse output stack types
        let mut outputs = Vec::new();
        while !self.check(&TokenKind::RightParen) && !self.is_at_end() {
            outputs.push(self.parse_type()?);
        }

        Ok(Effect::from_vecs(inputs, outputs))
    }

    fn parse_type(&mut self) -> Result<Type, ParseError> {
        self.enter_nesting()?;
        let result = self.parse_type_inner();
        self.exit_nesting();
        result
    }

    fn parse_type_inner(&mut self) -> Result<Type, ParseError> {
        let name = self.consume_ident("Expected type name")?;

        match name.as_str() {
            "Int" => Ok(Type::Int),
            "Bool" => Ok(Type::Bool),
            "String" => Ok(Type::String),
            _ => {
                // Check if it's a generic type variable (single uppercase letter or starts with lowercase)
                let first_char = name.chars().next();

                // Single uppercase letter or lowercase name = type variable
                if (name.len() == 1 && first_char.is_some_and(|c| c.is_uppercase()))
                    || first_char.is_some_and(|c| c.is_lowercase())
                {
                    Ok(Type::Var(name))
                } else {
                    // Named type, possibly with type arguments
                    let args = if self.check(&TokenKind::LeftParen) {
                        self.advance();
                        let mut args = Vec::new();
                        while !self.check(&TokenKind::RightParen) && !self.is_at_end() {
                            args.push(self.parse_type()?);
                            if self.check(&TokenKind::RightParen) {
                                break;
                            }
                        }
                        self.consume(&TokenKind::RightParen, "Expected ')'")?;
                        args
                    } else {
                        Vec::new()
                    };

                    Ok(Type::Named { name, args })
                }
            }
        }
    }

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.enter_nesting()?;
        let result = self.parse_expr_inner();
        self.exit_nesting();
        result
    }

    fn parse_expr_inner(&mut self) -> Result<Expr, ParseError> {
        match &self.peek().kind {
            TokenKind::IntLiteral => {
                let value = self.peek().lexeme.parse::<i64>().map_err(|_| {
                    let token = self.peek();
                    ParseError {
                        message: format!("Invalid integer: {}", token.lexeme),
                        line: token.line,
                        column: token.column,
                    }
                })?;
                let loc = self.current_loc();
                self.advance();
                Ok(Expr::IntLit(value, loc))
            }

            TokenKind::BoolLiteral => {
                let value = self.peek().lexeme == "true";
                let loc = self.current_loc();
                self.advance();
                Ok(Expr::BoolLit(value, loc))
            }

            TokenKind::StringLiteral => {
                let value = self.peek().lexeme.clone();
                let loc = self.current_loc();
                self.advance();
                Ok(Expr::StringLit(value, loc))
            }

            TokenKind::LeftBracket => {
                let loc = self.current_loc();
                self.advance(); // consume '['
                let mut exprs = Vec::new();
                while !self.check(&TokenKind::RightBracket) && !self.is_at_end() {
                    exprs.push(self.parse_expr()?);
                }
                self.consume(&TokenKind::RightBracket, "Expected ']'")?;
                Ok(Expr::Quotation(exprs, loc))
            }

            TokenKind::Match => {
                let loc = self.current_loc();
                self.advance(); // consume 'match'
                let mut branches = Vec::new();

                while !self.check(&TokenKind::End) && !self.is_at_end() {
                    let variant_name = self.consume_ident("Expected variant name")?;
                    self.consume(&TokenKind::Arrow, "Expected '=>'")?;

                    // Parse branch body (quotation)
                    self.consume(&TokenKind::LeftBracket, "Expected '[' for branch body")?;
                    let mut body = Vec::new();
                    while !self.check(&TokenKind::RightBracket) && !self.is_at_end() {
                        body.push(self.parse_expr()?);
                    }
                    self.consume(&TokenKind::RightBracket, "Expected ']'")?;

                    branches.push(MatchBranch {
                        pattern: Pattern::Variant { name: variant_name },
                        body,
                    });
                }

                self.consume(&TokenKind::End, "Expected 'end'")?;
                Ok(Expr::Match { branches, loc })
            }

            TokenKind::If => {
                let loc = self.current_loc();
                self.advance(); // consume 'if'

                // Expect two quotations: then-branch and else-branch
                let then_loc = self.current_loc();
                self.consume(&TokenKind::LeftBracket, "Expected '[' for then branch")?;
                let mut then_exprs = Vec::new();
                while !self.check(&TokenKind::RightBracket) && !self.is_at_end() {
                    then_exprs.push(self.parse_expr()?);
                }
                self.consume(&TokenKind::RightBracket, "Expected ']'")?;

                let else_loc = self.current_loc();
                self.consume(&TokenKind::LeftBracket, "Expected '[' for else branch")?;
                let mut else_exprs = Vec::new();
                while !self.check(&TokenKind::RightBracket) && !self.is_at_end() {
                    else_exprs.push(self.parse_expr()?);
                }
                self.consume(&TokenKind::RightBracket, "Expected ']'")?;

                Ok(Expr::If {
                    then_branch: Box::new(Expr::Quotation(then_exprs, then_loc)),
                    else_branch: Box::new(Expr::Quotation(else_exprs, else_loc)),
                    loc,
                })
            }

            TokenKind::Ident => {
                let name = self.peek().lexeme.clone();
                let loc = self.current_loc();
                self.advance();
                Ok(Expr::WordCall(name, loc))
            }

            _ => {
                let token = self.peek();
                Err(ParseError {
                    message: format!("Unexpected token: {:?}", token.kind),
                    line: token.line,
                    column: token.column,
                })
            }
        }
    }

    // Helper methods

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn is_at_end(&self) -> bool {
        self.peek().kind == TokenKind::Eof
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        &self.tokens[self.current - 1]
    }

    fn check(&self, kind: &TokenKind) -> bool {
        if self.is_at_end() {
            return false;
        }
        &self.peek().kind == kind
    }

    fn check_ident(&self, value: &str) -> bool {
        if self.is_at_end() {
            return false;
        }
        let token = self.peek();
        token.kind == TokenKind::Ident && token.lexeme == value
    }

    fn consume(&mut self, kind: &TokenKind, message: &str) -> Result<&Token, ParseError> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            Err(self.error(message))
        }
    }

    fn consume_ident(&mut self, message: &str) -> Result<String, ParseError> {
        if self.peek().kind == TokenKind::Ident {
            let lexeme = self.peek().lexeme.clone();
            self.advance();
            Ok(lexeme)
        } else {
            Err(self.error(message))
        }
    }

    fn consume_ident_value(&mut self, value: &str, message: &str) -> Result<(), ParseError> {
        if self.check_ident(value) {
            self.advance();
            Ok(())
        } else {
            Err(self.error(message))
        }
    }

    fn error(&self, message: &str) -> ParseError {
        let token = self.peek();
        ParseError {
            message: message.to_string(),
            line: token.line,
            column: token.column,
        }
    }

    fn enter_nesting(&mut self) -> Result<(), ParseError> {
        self.nesting_depth += 1;
        if self.nesting_depth > MAX_NESTING_DEPTH {
            Err(ParseError {
                message: format!("Maximum nesting depth of {} exceeded", MAX_NESTING_DEPTH),
                line: self.peek().line,
                column: self.peek().column,
            })
        } else {
            Ok(())
        }
    }

    fn exit_nesting(&mut self) {
        self.nesting_depth = self.nesting_depth.saturating_sub(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_word() {
        let input = ": square ( Int -- Int ) dup * ;";
        let mut parser = Parser::new(input);
        let program = parser.parse().unwrap();

        assert_eq!(program.word_defs.len(), 1);
        assert_eq!(program.word_defs[0].name, "square");
        assert_eq!(program.word_defs[0].body.len(), 2); // dup, *
    }

    #[test]
    fn test_parse_type_def() {
        let input = "type Option (T) | Some(T) | None";
        let mut parser = Parser::new(input);
        let program = parser.parse().unwrap();

        assert_eq!(program.type_defs.len(), 1);
        assert_eq!(program.type_defs[0].name, "Option");
        assert_eq!(program.type_defs[0].type_params.len(), 1);
        assert_eq!(program.type_defs[0].variants.len(), 2);
    }

    #[test]
    fn test_parse_literals() {
        let input = ": test ( -- Int ) 42 ;";
        let mut parser = Parser::new(input);
        let program = parser.parse().unwrap();

        assert_eq!(program.word_defs[0].body.len(), 1);
        match &program.word_defs[0].body[0] {
            Expr::IntLit(42, _) => (),
            _ => panic!("Expected IntLit(42)"),
        }
    }

    #[test]
    fn test_parse_quotation() {
        let input = ": test ( -- ) [ 1 2 + ] ;";
        let mut parser = Parser::new(input);
        let program = parser.parse().unwrap();

        assert_eq!(program.word_defs[0].body.len(), 1);
        match &program.word_defs[0].body[0] {
            Expr::Quotation(exprs, _) => assert_eq!(exprs.len(), 3),
            _ => panic!("Expected Quotation"),
        }
    }

    #[test]
    fn test_recursion_depth_limit() {
        // Create deeply nested quotations that exceed MAX_NESTING_DEPTH
        let mut input = String::from(": test ( -- ) ");
        for _ in 0..105 {
            input.push_str("[ ");
        }
        input.push_str("42 ");
        for _ in 0..105 {
            input.push_str("] ");
        }
        input.push(';');

        let mut parser = Parser::new(&input);
        let result = parser.parse();

        // Should fail with nesting depth error
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("nesting depth"));
    }

    #[test]
    fn test_source_location_tracking() {
        // Test that line/column numbers are captured correctly
        let input = ": test ( -- Int )\n  42 ;";
        let mut parser = Parser::new_with_filename(input, "test.cem");
        let program = parser.parse().unwrap();

        // Check word definition location (line 1, column 1 for ':')
        let word_loc = &program.word_defs[0].loc;
        assert_eq!(word_loc.line, 1);
        assert_eq!(word_loc.column, 1);
        assert_eq!(word_loc.file.as_ref(), "test.cem");

        // Check integer literal location (line 2, column 2 for '42')
        // The lexer uses 0-based columns internally but reports 1-based
        match &program.word_defs[0].body[0] {
            Expr::IntLit(42, loc) => {
                assert_eq!(loc.line, 2);
                assert_eq!(loc.column, 2); // Column for '4' in '42' after two spaces
                assert_eq!(loc.file.as_ref(), "test.cem");
            }
            _ => panic!("Expected IntLit"),
        }
    }

    #[test]
    fn test_source_location_shared_filename() {
        // Test that all locations share the same Arc<str> for filename
        let input = ": foo ( -- Int ) 1 2 + ;";
        let mut parser = Parser::new_with_filename(input, "shared.cem");
        let program = parser.parse().unwrap();

        let word_loc = &program.word_defs[0].loc;

        // Extract locations from expressions
        let mut locs = vec![word_loc];
        for expr in &program.word_defs[0].body {
            locs.push(expr.loc());
        }

        // Verify all locations point to the same Arc<str> instance
        // (Arc::ptr_eq checks if they point to the same allocation)
        for i in 1..locs.len() {
            assert!(
                Arc::ptr_eq(&locs[0].file, &locs[i].file),
                "SourceLoc filenames should share the same Arc allocation"
            );
        }
    }

    #[test]
    fn test_loc_accessor() {
        // Test the loc() accessor method on Expr
        let input = ": test ( -- ) 42 true \"hello\" word [ 1 ] ;";
        let mut parser = Parser::new_with_filename(input, "test.cem");
        let program = parser.parse().unwrap();

        // Verify loc() works for all expression types
        for expr in &program.word_defs[0].body {
            let loc = expr.loc();
            assert_eq!(loc.file.as_ref(), "test.cem");
            assert!(loc.line > 0);
            assert!(loc.column > 0);
        }
    }

    #[test]
    fn test_multiline_location_tracking() {
        // Test location tracking across multiple lines
        let input = ":\ntest\n(\n--\nInt\n)\n42\n;";
        let mut parser = Parser::new(input);
        let program = parser.parse().unwrap();

        // The integer 42 should be on line 7
        match &program.word_defs[0].body[0] {
            Expr::IntLit(42, loc) => {
                assert_eq!(loc.line, 7, "Integer literal should be on line 7");
            }
            _ => panic!("Expected IntLit"),
        }
    }

    #[test]
    fn test_source_loc_unknown() {
        // Test SourceLoc::unknown() utility
        let loc = crate::ast::SourceLoc::unknown();
        assert_eq!(loc.line, 0);
        assert_eq!(loc.column, 0);
        assert_eq!(loc.file.as_ref(), "<unknown>");
    }

    #[test]
    fn test_source_loc_display() {
        // Test SourceLoc Display impl
        let loc = crate::ast::SourceLoc::new(10, 5, "test.cem");
        assert_eq!(format!("{}", loc), "test.cem:10:5");
    }

    #[test]
    fn test_parse_multifield_variant() {
        // Test that multi-field variants parse correctly
        // Bug: comma between fields was being parsed as a field type
        let input = r#"
            type List(T)
              | Cons(T, List(T))
              | Nil
        "#;

        let mut parser = Parser::new(input);
        let program = parser.parse().unwrap();

        assert_eq!(program.type_defs.len(), 1);
        let typedef = &program.type_defs[0];
        assert_eq!(typedef.name, "List");
        assert_eq!(typedef.type_params.len(), 1);
        assert_eq!(typedef.variants.len(), 2);

        // Check Cons variant has exactly 2 fields
        let cons_variant = &typedef.variants[0];
        assert_eq!(cons_variant.name, "Cons");
        assert_eq!(
            cons_variant.fields.len(),
            2,
            "Cons should have 2 fields, not {}. Fields: {:?}",
            cons_variant.fields.len(),
            cons_variant.fields
        );

        // Verify no comma in fields (bug check)
        for field in &cons_variant.fields {
            if let crate::ast::types::Type::Named { name, .. } = field {
                assert_ne!(name, ",", "Comma should not be parsed as a field type");
            }
        }

        // Verify the fields are the correct types
        match &cons_variant.fields[0] {
            crate::ast::types::Type::Var(name) => assert_eq!(name, "T"),
            other => panic!("First field should be Var(T), got {:?}", other),
        }

        match &cons_variant.fields[1] {
            crate::ast::types::Type::Named { name, args } => {
                assert_eq!(name, "List");
                assert_eq!(args.len(), 1);
            }
            other => panic!("Second field should be Named(List), got {:?}", other),
        }

        // Check Nil variant has 0 fields
        let nil_variant = &typedef.variants[1];
        assert_eq!(nil_variant.name, "Nil");
        assert_eq!(nil_variant.fields.len(), 0);
    }
}
