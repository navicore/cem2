/**
Type system definitions for Cem

This module defines the representation of types and effects in the Cem type system.
*/
use std::fmt;

/// A type in the Cem type system
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    /// Integer type (Copy)
    Int,

    /// Boolean type (Copy)
    Bool,

    /// String type (Linear - not Copy)
    String,

    /// Type variable (for polymorphism)
    Var(String),

    /// Named type (user-defined ADT)
    Named { name: String, args: Vec<Type> },

    /// Quotation type (first-class function)
    Quotation(Box<Effect>),
}

/// Stack effect signature: (inputs -- outputs)
///
/// Represents the transformation a word performs on the stack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Effect {
    /// Types consumed from stack (bottom to top)
    pub inputs: StackType,

    /// Types produced to stack (bottom to top)
    pub outputs: StackType,
}

/// A stack type represents the state of the stack
///
/// Uses row polymorphism to allow "rest of stack" variables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StackType {
    /// Empty stack
    Empty,

    /// Concrete type on top of another stack
    Cons { rest: Box<StackType>, top: Type },

    /// Row variable (represents "rest of stack")
    /// Allows polymorphism over unknown stack depths
    RowVar(String),
}

impl StackType {
    /// Create an empty stack
    pub fn empty() -> Self {
        StackType::Empty
    }

    /// Push a type onto the stack
    pub fn push(self, ty: Type) -> Self {
        StackType::Cons {
            rest: Box::new(self),
            top: ty,
        }
    }

    /// Create a stack from a vec of types (first = bottom, last = top)
    pub fn from_vec(types: Vec<Type>) -> Self {
        types
            .into_iter()
            .fold(StackType::Empty, |stack, ty| stack.push(ty))
    }

    /// Pop a type from the stack, returning (rest, top) or None if empty
    pub fn pop(self) -> Option<(StackType, Type)> {
        match self {
            StackType::Cons { rest, top } => Some((*rest, top)),
            StackType::Empty => None,
            StackType::RowVar(_) => None, // Can't pop from unknown stack
        }
    }

    /// Get the depth of the stack (if known)
    pub fn depth(&self) -> Option<usize> {
        match self {
            StackType::Empty => Some(0),
            StackType::Cons { rest, .. } => rest.depth().map(|d| d + 1),
            StackType::RowVar(_) => None, // Unknown depth
        }
    }

    /// Check if this is a row variable
    pub fn is_row_var(&self) -> bool {
        matches!(self, StackType::RowVar(_))
    }
}

impl Effect {
    /// Create a new effect signature
    pub fn new(inputs: StackType, outputs: StackType) -> Self {
        Effect { inputs, outputs }
    }

    /// Create an effect from input/output type vectors
    pub fn from_vecs(inputs: Vec<Type>, outputs: Vec<Type>) -> Self {
        Effect {
            inputs: StackType::from_vec(inputs),
            outputs: StackType::from_vec(outputs),
        }
    }

    /// Compose two effects: first, then second
    ///
    /// The output of `first` must match the input of `second`.
    /// Returns the composed effect or None if incompatible.
    pub fn compose(first: &Effect, second: &Effect) -> Option<Effect> {
        // For now, require exact match (will need unification for polymorphic composition)
        if first.outputs == second.inputs {
            Some(Effect {
                inputs: first.inputs.clone(),
                outputs: second.outputs.clone(),
            })
        } else {
            None
        }
    }
}

impl Type {
    /// Check if this type is Copy (can be duplicated without clone)
    pub fn is_copy(&self) -> bool {
        match self {
            Type::Int | Type::Bool => true,
            Type::String => false,
            Type::Var(_) => false,       // Conservative: assume not Copy
            Type::Named { .. } => false, // Conservative: requires trait analysis
            Type::Quotation(_) => true,  // Quotations are Copy (just code pointers for now)
        }
    }

    /// Check if this type is linear (requires explicit clone)
    pub fn is_linear(&self) -> bool {
        !self.is_copy()
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Int => write!(f, "Int"),
            Type::Bool => write!(f, "Bool"),
            Type::String => write!(f, "String"),
            Type::Var(name) => write!(f, "{}", name),
            Type::Named { name, args } => {
                write!(f, "{}", name)?;
                if !args.is_empty() {
                    write!(f, "<")?;
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", arg)?;
                    }
                    write!(f, ">")?;
                }
                Ok(())
            }
            Type::Quotation(eff) => write!(f, "[{}]", eff),
        }
    }
}

impl fmt::Display for StackType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StackType::Empty => write!(f, ""),
            StackType::Cons { rest, top } => {
                if !matches!(**rest, StackType::Empty) {
                    write!(f, "{} ", rest)?;
                }
                write!(f, "{}", top)
            }
            StackType::RowVar(name) => write!(f, "{}", name),
        }
    }
}

impl fmt::Display for Effect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "( {} -- {} )", self.inputs, self.outputs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stack_operations() {
        let stack = StackType::empty().push(Type::Int).push(Type::Bool);

        assert_eq!(stack.depth(), Some(2));

        let (rest, top) = stack.pop().unwrap();
        assert_eq!(top, Type::Bool);
        assert_eq!(rest.depth(), Some(1));
    }

    #[test]
    fn test_effect_composition() {
        // dup: (A -- A A)
        let dup = Effect::from_vecs(
            vec![Type::Var("A".to_string())],
            vec![Type::Var("A".to_string()), Type::Var("A".to_string())],
        );

        // +: (Int Int -- Int)
        let add = Effect::from_vecs(vec![Type::Int, Type::Int], vec![Type::Int]);

        // dup then + requires A = Int
        // For now, this will fail (needs unification)
        assert!(Effect::compose(&dup, &add).is_none());

        // But concrete Int versions should compose
        let dup_int = Effect::from_vecs(vec![Type::Int], vec![Type::Int, Type::Int]);
        let composed = Effect::compose(&dup_int, &add);
        assert!(composed.is_some());
        let composed = composed.unwrap();
        assert_eq!(composed.inputs.depth(), Some(1));
        assert_eq!(composed.outputs.depth(), Some(1));
    }

    #[test]
    fn test_copy_types() {
        assert!(Type::Int.is_copy());
        assert!(Type::Bool.is_copy());
        assert!(!Type::String.is_copy());
        assert!(Type::String.is_linear());
    }
}
