/**
Abstract Syntax Tree definitions for Cem

This module defines the core AST types representing Cem programs.
*/
pub mod types;

use std::fmt;
use std::sync::Arc;

/// Source code location for debugging and error messages
///
/// Uses Arc<str> for the filename to avoid duplicating it across the AST.
/// This is important because a large program may have thousands of AST nodes
/// all referring to the same file.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceLoc {
    pub line: usize,
    pub column: usize,
    pub file: Arc<str>,
}

impl SourceLoc {
    pub fn new(line: usize, column: usize, file: impl Into<Arc<str>>) -> Self {
        Self {
            line,
            column,
            file: file.into(),
        }
    }

    /// Create an unknown/synthetic location (for generated code or tests)
    pub fn unknown() -> Self {
        Self {
            line: 0,
            column: 0,
            file: Arc::from("<unknown>"),
        }
    }

    /// Create a location with just a file (line/column unknown)
    pub fn file_only(file: impl Into<Arc<str>>) -> Self {
        Self {
            line: 1,
            column: 1,
            file: file.into(),
        }
    }
}

impl fmt::Display for SourceLoc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.file, self.line, self.column)
    }
}

/// A complete Cem program
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub type_defs: Vec<TypeDef>,
    pub word_defs: Vec<WordDef>,
}

/// Type definition (Algebraic Data Type / Sum Type)
#[derive(Debug, Clone, PartialEq)]
pub struct TypeDef {
    pub name: String,
    pub type_params: Vec<String>,
    pub variants: Vec<Variant>,
}

/// A variant of a sum type
#[derive(Debug, Clone, PartialEq)]
pub struct Variant {
    pub name: String,
    pub fields: Vec<types::Type>,
}

/// Word (function) definition
#[derive(Debug, Clone, PartialEq)]
pub struct WordDef {
    pub name: String,
    pub effect: types::Effect,
    pub body: Vec<Expr>,
    pub loc: SourceLoc, // Location of the word definition (: word_name line)
}

/// Expression in the body of a word
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Literal integer
    IntLit(i64, SourceLoc),

    /// Literal boolean
    BoolLit(bool, SourceLoc),

    /// Literal string
    StringLit(String, SourceLoc),

    /// Word call (reference to another word)
    WordCall(String, SourceLoc),

    /// Quotation (code block)
    Quotation(Vec<Expr>, SourceLoc),

    /// Pattern match expression
    Match {
        branches: Vec<MatchBranch>,
        loc: SourceLoc,
    },

    /// If expression (condition is top of stack)
    If {
        then_branch: Box<Expr>,
        else_branch: Box<Expr>,
        loc: SourceLoc,
    },
}

impl Expr {
    /// Get the source location of any expression
    pub fn loc(&self) -> &SourceLoc {
        match self {
            Expr::IntLit(_, loc) => loc,
            Expr::BoolLit(_, loc) => loc,
            Expr::StringLit(_, loc) => loc,
            Expr::WordCall(_, loc) => loc,
            Expr::Quotation(_, loc) => loc,
            Expr::Match { loc, .. } => loc,
            Expr::If { loc, .. } => loc,
        }
    }
}

/// A branch in a pattern match
#[derive(Debug, Clone, PartialEq)]
pub struct MatchBranch {
    pub pattern: Pattern,
    pub body: Vec<Expr>,
}

/// Pattern for matching on sum types
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// Match a specific variant, binding its fields
    Variant {
        name: String,
        // Field patterns could be added later for nested matching
    },
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::IntLit(n, _) => write!(f, "{}", n),
            Expr::BoolLit(b, _) => write!(f, "{}", b),
            Expr::StringLit(s, _) => write!(f, "\"{}\"", s),
            Expr::WordCall(name, _) => write!(f, "{}", name),
            Expr::Quotation(exprs, _) => {
                write!(f, "[ ")?;
                for expr in exprs {
                    write!(f, "{} ", expr)?;
                }
                write!(f, "]")
            }
            Expr::Match { branches, .. } => {
                writeln!(f, "match")?;
                for branch in branches {
                    writeln!(f, "  {:?} => [ ... ]", branch.pattern)?;
                }
                write!(f, "end")
            }
            Expr::If { .. } => write!(f, "if"),
        }
    }
}
