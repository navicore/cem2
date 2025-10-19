/**
Type unification for Cem

Implements unification of types and stack types for polymorphic type checking.
*/
use crate::ast::types::{StackType, Type};
use crate::typechecker::errors::{TypeError, TypeResult};
use std::collections::HashMap;

/// Substitution map: type variable -> concrete type
pub type Substitution = HashMap<String, Type>;

/// Substitution for stack row variables
pub type StackSubstitution = HashMap<String, StackType>;

/// Unify two types, returning a substitution or error
pub fn unify_types(ty1: &Type, ty2: &Type) -> TypeResult<Substitution> {
    let mut subst = HashMap::new();
    unify_types_with_subst(ty1, ty2, &mut subst)?;
    Ok(subst)
}

fn unify_types_with_subst(ty1: &Type, ty2: &Type, subst: &mut Substitution) -> TypeResult<()> {
    match (ty1, ty2) {
        // Same primitive types unify
        (Type::Int, Type::Int) => Ok(()),
        (Type::Bool, Type::Bool) => Ok(()),
        (Type::String, Type::String) => Ok(()),

        // Type variables
        (Type::Var(name), ty) | (ty, Type::Var(name)) => {
            if let Some(existing) = subst.get(name).cloned() {
                // Variable already bound, check consistency
                unify_types_with_subst(&existing, ty, subst)
            } else {
                // Bind variable
                subst.insert(name.clone(), ty.clone());
                Ok(())
            }
        }

        // Named types (ADTs) must have same name and compatible args
        (Type::Named { name: n1, args: a1 }, Type::Named { name: n2, args: a2 }) => {
            if n1 != n2 {
                return Err(Box::new(TypeError::UnificationError {
                    ty1: ty1.clone(),
                    ty2: ty2.clone(),
                    reason: format!("Type names don't match: {} vs {}", n1, n2),
                }));
            }

            if a1.len() != a2.len() {
                return Err(Box::new(TypeError::UnificationError {
                    ty1: ty1.clone(),
                    ty2: ty2.clone(),
                    reason: "Different number of type arguments".to_string(),
                }));
            }

            // Unify all type arguments
            for (arg1, arg2) in a1.iter().zip(a2.iter()) {
                unify_types_with_subst(arg1, arg2, subst)?;
            }

            Ok(())
        }

        // Quotations: unify their effects (would need effect unification)
        (Type::Quotation(_eff1), Type::Quotation(_eff2)) => {
            // TODO(#10): Implement effect unification
            //
            // KNOWN LIMITATION: Any two quotation types unify successfully, even if
            // they have incompatible effects. This compounds the soundness hole from
            // quotation type inference.
            //
            // Example that should fail but won't:
            //   : takes-int-to-int ( [Int -- Int] -- ) drop ;
            //   : main ( -- ) [ "hello" write_line ] takes-int-to-int ;
            //
            // To fix: recursively unify the input and output stack types of the effects
            // For now, just succeed
            Ok(())
        }

        // Mismatched types
        _ => Err(Box::new(TypeError::UnificationError {
            ty1: ty1.clone(),
            ty2: ty2.clone(),
            reason: "Types are incompatible".to_string(),
        })),
    }
}

/// Unify two stack types
pub fn unify_stack_types(
    stack1: &StackType,
    stack2: &StackType,
) -> TypeResult<(Substitution, StackSubstitution)> {
    let mut type_subst = HashMap::new();
    let mut stack_subst = HashMap::new();

    unify_stack_types_with_subst(stack1, stack2, &mut type_subst, &mut stack_subst)?;

    Ok((type_subst, stack_subst))
}

fn unify_stack_types_with_subst(
    stack1: &StackType,
    stack2: &StackType,
    type_subst: &mut Substitution,
    stack_subst: &mut StackSubstitution,
) -> TypeResult<()> {
    match (stack1, stack2) {
        // Empty stacks unify
        (StackType::Empty, StackType::Empty) => Ok(()),

        // Cons cells: unify tops and rests
        (StackType::Cons { rest: r1, top: t1 }, StackType::Cons { rest: r2, top: t2 }) => {
            // Unify the top types
            unify_types_with_subst(t1, t2, type_subst)?;

            // Unify the rest stacks
            unify_stack_types_with_subst(r1, r2, type_subst, stack_subst)?;

            Ok(())
        }

        // Row variable can unify with anything
        (StackType::RowVar(name), stack) | (stack, StackType::RowVar(name)) => {
            if let Some(existing) = stack_subst.get(name).cloned() {
                // Variable already bound, check consistency
                unify_stack_types_with_subst(&existing, stack, type_subst, stack_subst)
            } else {
                // Bind variable
                stack_subst.insert(name.clone(), stack.clone());
                Ok(())
            }
        }

        // Mismatched stacks
        _ => Err(Box::new(TypeError::StackUnificationError {
            stack1: stack1.clone(),
            stack2: stack2.clone(),
            reason: "Stack shapes are incompatible".to_string(),
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unify_primitives() {
        assert!(unify_types(&Type::Int, &Type::Int).is_ok());
        assert!(unify_types(&Type::Bool, &Type::Bool).is_ok());
        assert!(unify_types(&Type::Int, &Type::Bool).is_err());
    }

    #[test]
    fn test_unify_type_variables() {
        let a = Type::Var("A".to_string());
        let int = Type::Int;

        let subst = unify_types(&a, &int).unwrap();
        assert_eq!(subst.get("A"), Some(&Type::Int));
    }

    #[test]
    fn test_unify_named_types() {
        let opt_int1 = Type::Named {
            name: "Option".to_string(),
            args: vec![Type::Int],
        };

        let opt_int2 = Type::Named {
            name: "Option".to_string(),
            args: vec![Type::Int],
        };

        assert!(unify_types(&opt_int1, &opt_int2).is_ok());

        let opt_bool = Type::Named {
            name: "Option".to_string(),
            args: vec![Type::Bool],
        };

        assert!(unify_types(&opt_int1, &opt_bool).is_err());
    }

    #[test]
    fn test_unify_stack_types() {
        let stack1 = StackType::empty().push(Type::Int);
        let stack2 = StackType::empty().push(Type::Int);

        assert!(unify_stack_types(&stack1, &stack2).is_ok());

        let stack3 = StackType::empty().push(Type::Bool);
        assert!(unify_stack_types(&stack1, &stack3).is_err());
    }
}
