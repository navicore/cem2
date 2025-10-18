/**
Error types for code generation
*/
use std::fmt;

/// Errors that can occur during code generation
#[derive(Debug, Clone, PartialEq)]
pub enum CodegenError {
    /// Feature not yet implemented
    Unimplemented { feature: String },

    /// Internal compiler error
    InternalError(String),

    /// Linker error
    LinkerError { message: String },
}

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CodegenError::Unimplemented { feature } => {
                write!(f, "Feature not yet implemented: {}", feature)
            }
            CodegenError::InternalError(msg) => {
                write!(f, "Internal compiler error: {}", msg)
            }
            CodegenError::LinkerError { message } => {
                write!(f, "Linker error: {}", message)
            }
        }
    }
}

impl std::error::Error for CodegenError {}

/// Result type for code generation operations
pub type CodegenResult<T> = Result<T, CodegenError>;
