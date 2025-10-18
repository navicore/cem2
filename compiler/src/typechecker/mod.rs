pub mod checker;
/**
Type checker for Cem

This module implements bidirectional type checking with:
- Stack effect inference
- Row polymorphism
- Linear type tracking
- Pattern matching exhaustiveness
*/
pub mod environment;
pub mod errors;
pub mod unification;

pub use checker::TypeChecker;
pub use errors::{TypeError, TypeResult};
