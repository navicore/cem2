/// Cem2 - A concatenative language with linear types
/// Pronounced "seam"
///
/// This crate implements the Cem compiler, including:
/// - Abstract syntax tree (AST) representation
/// - Type checker with effect inference
/// - Pattern matching exhaustiveness checking
/// - LLVM code generation
pub mod ast;
pub mod codegen;
pub mod parser;
pub mod typechecker;

pub use ast::types::{Effect, StackType, Type};
pub use ast::{Expr, Program, TypeDef, WordDef};
