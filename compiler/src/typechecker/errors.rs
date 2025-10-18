/**
Type checking errors for Cem
*/
use crate::ast::types::{Effect, StackType, Type};
use std::fmt;

// Box the error type to reduce stack size (clippy::result_large_err)
pub type TypeResult<T> = Result<T, Box<TypeError>>;

#[derive(Debug, Clone, PartialEq)]
pub enum TypeError {
    /// Stack underflow: tried to pop from empty stack
    StackUnderflow {
        word: String,
        required: usize,
        available: usize,
    },

    /// Type mismatch between expected and actual
    TypeMismatch {
        expected: Type,
        actual: Type,
        context: String,
    },

    /// Effect mismatch between expected and actual
    EffectMismatch {
        expected: Effect,
        actual: Effect,
        word: String,
    },

    /// Undefined word reference
    UndefinedWord { name: String },

    /// Undefined type reference
    UndefinedType { name: String },

    /// Non-exhaustive pattern match
    NonExhaustiveMatch {
        type_name: String,
        missing_variants: Vec<String>,
    },

    /// Inconsistent effects across pattern match branches
    InconsistentBranchEffects {
        type_name: String,
        expected: Effect,
        actual: Effect,
        branch: String,
    },

    /// Attempt to duplicate non-Copy type
    CannotDuplicate { ty: Type, operation: String },

    /// Use of value after move (linear type violation)
    UseAfterMove { var: String },

    /// Cannot unify types (for polymorphism)
    UnificationError {
        ty1: Type,
        ty2: Type,
        reason: String,
    },

    /// Cannot unify stack types
    StackUnificationError {
        stack1: StackType,
        stack2: StackType,
        reason: String,
    },

    /// Generic error
    Other { message: String },
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeError::StackUnderflow {
                word,
                required,
                available,
            } => {
                write!(
                    f,
                    "Stack underflow in '{}': requires {} element(s), but only {} available",
                    word, required, available
                )
            }

            TypeError::TypeMismatch {
                expected,
                actual,
                context,
            } => {
                write!(
                    f,
                    "Type mismatch in {}: expected {}, but got {}",
                    context, expected, actual
                )
            }

            TypeError::EffectMismatch {
                expected,
                actual,
                word,
            } => {
                write!(
                    f,
                    "Effect mismatch in '{}': expected {}, but got {}",
                    word, expected, actual
                )
            }

            TypeError::UndefinedWord { name } => {
                write!(f, "Undefined word: '{}'", name)
            }

            TypeError::UndefinedType { name } => {
                write!(f, "Undefined type: '{}'", name)
            }

            TypeError::NonExhaustiveMatch {
                type_name,
                missing_variants,
            } => {
                write!(
                    f,
                    "Non-exhaustive pattern match on type '{}': missing variants: {}",
                    type_name,
                    missing_variants.join(", ")
                )
            }

            TypeError::InconsistentBranchEffects {
                type_name,
                expected,
                actual,
                branch,
            } => {
                write!(
                    f,
                    "Inconsistent effect in pattern match on '{}' in branch '{}': expected {}, but got {}",
                    type_name, branch, expected, actual
                )
            }

            TypeError::CannotDuplicate { ty, operation } => {
                write!(
                    f,
                    "Cannot duplicate non-Copy type {} in operation '{}'.\n\
                     Hint: Use 'clone' to explicitly duplicate this value",
                    ty, operation
                )
            }

            TypeError::UseAfterMove { var } => {
                write!(f, "Use of '{}' after move (linear type violation)", var)
            }

            TypeError::UnificationError { ty1, ty2, reason } => {
                write!(f, "Cannot unify types {} and {}: {}", ty1, ty2, reason)
            }

            TypeError::StackUnificationError {
                stack1,
                stack2,
                reason,
            } => {
                write!(
                    f,
                    "Cannot unify stack types ({}) and ({}): {}",
                    stack1, stack2, reason
                )
            }

            TypeError::Other { message } => {
                write!(f, "{}", message)
            }
        }
    }
}

impl std::error::Error for TypeError {}
