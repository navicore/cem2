/**
Type checking environment for Cem

Maintains symbol tables for words, types, and type variables during type checking.
*/
use crate::ast::types::{Effect, StackType, Type};
use crate::ast::{TypeDef, Variant};
use std::collections::HashMap;

/// Type checking environment
///
/// Contains:
/// - Word definitions (name -> effect signature)
/// - Type definitions (name -> ADT definition)
/// - Built-in primitives
#[derive(Debug, Clone)]
pub struct Environment {
    /// Word definitions: name -> effect
    words: HashMap<String, Effect>,

    /// Type definitions: name -> TypeDef
    types: HashMap<String, TypeDef>,
}

impl Environment {
    /// Create a new environment with built-in primitives
    pub fn new() -> Self {
        let mut env = Environment {
            words: HashMap::new(),
            types: HashMap::new(),
        };

        // Add built-in stack operations
        env.add_builtin_words();
        env.add_builtin_types();

        env
    }

    /// Add a word definition
    pub fn add_word(&mut self, name: String, effect: Effect) {
        self.words.insert(name, effect);
    }

    /// Look up a word's effect signature
    pub fn lookup_word(&self, name: &str) -> Option<&Effect> {
        self.words.get(name)
    }

    /// Add a type definition and automatically create variant constructor words
    pub fn add_type(&mut self, typedef: TypeDef) {
        // Note: Validation of variant features (multi-field, nested) happens at codegen time
        // This allows defining types that aren't fully supported yet, as long as they're not used

        // For each variant, create a constructor word
        // Example: type Option<T> = Some(T) | None
        // Creates:
        //   Some : ( T -- Option(T) )
        //   None : ( -- Option(T) )

        for variant in &typedef.variants {
            // Build the result type (the ADT with type parameters)
            let result_type = if typedef.type_params.is_empty() {
                // Non-generic type: Option => Option
                Type::Named {
                    name: typedef.name.clone(),
                    args: vec![],
                }
            } else {
                // Generic type: Option<T> => Option(T)
                Type::Named {
                    name: typedef.name.clone(),
                    args: typedef
                        .type_params
                        .iter()
                        .map(|p| Type::Var(p.clone()))
                        .collect(),
                }
            };

            // Build the effect signature
            // Input stack: variant fields (if any)
            // Output stack: the ADT type
            //
            // Note: .rev() is used because stack types are built right-to-left
            // For Some(T), we want: ( T -- Option(T) )
            // Without .rev(), we'd get the fields in wrong order for multi-field variants
            let effect = Effect {
                inputs: variant
                    .fields
                    .iter()
                    .rev()
                    .fold(StackType::Empty, |stack, field| stack.push(field.clone())),
                outputs: StackType::Empty.push(result_type),
            };

            // Register the variant constructor as a word
            self.add_word(variant.name.clone(), effect);
        }

        // Store the type definition
        self.types.insert(typedef.name.clone(), typedef);
    }

    /// Look up a type definition
    pub fn lookup_type(&self, name: &str) -> Option<&TypeDef> {
        self.types.get(name)
    }

    /// Get all variants for a sum type (for exhaustiveness checking)
    pub fn get_variants(&self, type_name: &str) -> Option<&[Variant]> {
        self.types.get(type_name).map(|td| td.variants.as_slice())
    }

    /// Add built-in word definitions
    fn add_builtin_words(&mut self) {
        use crate::ast::types::StackType;

        // dup: ( A -- A A )
        self.add_word(
            "dup".to_string(),
            Effect {
                inputs: StackType::empty().push(Type::Var("A".to_string())),
                outputs: StackType::empty()
                    .push(Type::Var("A".to_string()))
                    .push(Type::Var("A".to_string())),
            },
        );

        // drop: ( A -- )
        self.add_word(
            "drop".to_string(),
            Effect {
                inputs: StackType::empty().push(Type::Var("A".to_string())),
                outputs: StackType::empty(),
            },
        );

        // swap: ( A B -- B A )
        self.add_word(
            "swap".to_string(),
            Effect {
                inputs: StackType::empty()
                    .push(Type::Var("A".to_string()))
                    .push(Type::Var("B".to_string())),
                outputs: StackType::empty()
                    .push(Type::Var("B".to_string()))
                    .push(Type::Var("A".to_string())),
            },
        );

        // over: ( A B -- A B A )
        self.add_word(
            "over".to_string(),
            Effect {
                inputs: StackType::empty()
                    .push(Type::Var("A".to_string()))
                    .push(Type::Var("B".to_string())),
                outputs: StackType::empty()
                    .push(Type::Var("A".to_string()))
                    .push(Type::Var("B".to_string()))
                    .push(Type::Var("A".to_string())),
            },
        );

        // rot: ( A B C -- B C A )
        self.add_word(
            "rot".to_string(),
            Effect {
                inputs: StackType::empty()
                    .push(Type::Var("A".to_string()))
                    .push(Type::Var("B".to_string()))
                    .push(Type::Var("C".to_string())),
                outputs: StackType::empty()
                    .push(Type::Var("B".to_string()))
                    .push(Type::Var("C".to_string()))
                    .push(Type::Var("A".to_string())),
            },
        );

        // nip: ( A B -- B )
        self.add_word(
            "nip".to_string(),
            Effect {
                inputs: StackType::empty()
                    .push(Type::Var("A".to_string()))
                    .push(Type::Var("B".to_string())),
                outputs: StackType::empty().push(Type::Var("B".to_string())),
            },
        );

        // tuck: ( A B -- B A B )
        self.add_word(
            "tuck".to_string(),
            Effect {
                inputs: StackType::empty()
                    .push(Type::Var("A".to_string()))
                    .push(Type::Var("B".to_string())),
                outputs: StackType::empty()
                    .push(Type::Var("B".to_string()))
                    .push(Type::Var("A".to_string()))
                    .push(Type::Var("B".to_string())),
            },
        );

        // Arithmetic operations
        // +: ( Int Int -- Int )
        self.add_word(
            "+".to_string(),
            Effect::from_vecs(vec![Type::Int, Type::Int], vec![Type::Int]),
        );

        // -: ( Int Int -- Int )
        self.add_word(
            "-".to_string(),
            Effect::from_vecs(vec![Type::Int, Type::Int], vec![Type::Int]),
        );

        // *: ( Int Int -- Int )
        self.add_word(
            "*".to_string(),
            Effect::from_vecs(vec![Type::Int, Type::Int], vec![Type::Int]),
        );

        // /: ( Int Int -- Int )
        self.add_word(
            "/".to_string(),
            Effect::from_vecs(vec![Type::Int, Type::Int], vec![Type::Int]),
        );

        // Comparison operations
        // =: ( Int Int -- Bool )
        self.add_word(
            "=".to_string(),
            Effect::from_vecs(vec![Type::Int, Type::Int], vec![Type::Bool]),
        );

        // <: ( Int Int -- Bool )
        self.add_word(
            "<".to_string(),
            Effect::from_vecs(vec![Type::Int, Type::Int], vec![Type::Bool]),
        );

        // >: ( Int Int -- Bool )
        self.add_word(
            ">".to_string(),
            Effect::from_vecs(vec![Type::Int, Type::Int], vec![Type::Bool]),
        );

        // <=: ( Int Int -- Bool )
        self.add_word(
            "<=".to_string(),
            Effect::from_vecs(vec![Type::Int, Type::Int], vec![Type::Bool]),
        );

        // >=: ( Int Int -- Bool )
        self.add_word(
            ">=".to_string(),
            Effect::from_vecs(vec![Type::Int, Type::Int], vec![Type::Bool]),
        );

        // !=: ( Int Int -- Bool )
        self.add_word(
            "!=".to_string(),
            Effect::from_vecs(vec![Type::Int, Type::Int], vec![Type::Bool]),
        );

        // clone: ( A -- A A ) for explicit cloning
        self.add_word(
            "clone".to_string(),
            Effect {
                inputs: StackType::empty().push(Type::Var("A".to_string())),
                outputs: StackType::empty()
                    .push(Type::Var("A".to_string()))
                    .push(Type::Var("A".to_string())),
            },
        );

        // Type conversions
        // int-to-string: ( Int -- String )
        self.add_word(
            "int-to-string".to_string(),
            Effect::from_vecs(vec![Type::Int], vec![Type::String]),
        );

        // bool-to-string: ( Bool -- String )
        self.add_word(
            "bool-to-string".to_string(),
            Effect::from_vecs(vec![Type::Bool], vec![Type::String]),
        );

        // exit: ( Int -- )
        // Note: This function never returns, but we model it as consuming Int and producing empty stack
        self.add_word(
            "exit".to_string(),
            Effect::from_vecs(vec![Type::Int], vec![]),
        );
    }

    /// Add built-in type definitions
    fn add_builtin_types(&mut self) {
        // Option<T>
        self.add_type(TypeDef {
            name: "Option".to_string(),
            type_params: vec!["T".to_string()],
            variants: vec![
                Variant {
                    name: "Some".to_string(),
                    fields: vec![Type::Var("T".to_string())],
                },
                Variant {
                    name: "None".to_string(),
                    fields: vec![],
                },
            ],
        });

        // Result<T, E>
        self.add_type(TypeDef {
            name: "Result".to_string(),
            type_params: vec!["T".to_string(), "E".to_string()],
            variants: vec![
                Variant {
                    name: "Ok".to_string(),
                    fields: vec![Type::Var("T".to_string())],
                },
                Variant {
                    name: "Err".to_string(),
                    fields: vec![Type::Var("E".to_string())],
                },
            ],
        });

        // List<T>
        self.add_type(TypeDef {
            name: "List".to_string(),
            type_params: vec!["T".to_string()],
            variants: vec![
                Variant {
                    name: "Cons".to_string(),
                    fields: vec![
                        Type::Var("T".to_string()),
                        Type::Named {
                            name: "List".to_string(),
                            args: vec![Type::Var("T".to_string())],
                        },
                    ],
                },
                Variant {
                    name: "Nil".to_string(),
                    fields: vec![],
                },
            ],
        });
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_words() {
        let env = Environment::new();

        // Should have basic stack operations
        assert!(env.lookup_word("dup").is_some());
        assert!(env.lookup_word("drop").is_some());
        assert!(env.lookup_word("swap").is_some());

        // Should have arithmetic
        assert!(env.lookup_word("+").is_some());
        assert!(env.lookup_word("*").is_some());

        // Unknown word
        assert!(env.lookup_word("unknown").is_none());
    }

    #[test]
    fn test_builtin_types() {
        let env = Environment::new();

        // Should have Option
        let option_def = env.lookup_type("Option");
        assert!(option_def.is_some());
        assert_eq!(option_def.unwrap().variants.len(), 2);

        // Should have Result
        let result_def = env.lookup_type("Result");
        assert!(result_def.is_some());
        assert_eq!(result_def.unwrap().variants.len(), 2);

        // Should have List
        let list_def = env.lookup_type("List");
        assert!(list_def.is_some());
        assert_eq!(list_def.unwrap().variants.len(), 2);
    }

    #[test]
    fn test_add_word() {
        let mut env = Environment::new();

        let square_effect = Effect::from_vecs(vec![Type::Int], vec![Type::Int]);
        env.add_word("square".to_string(), square_effect.clone());

        let looked_up = env.lookup_word("square");
        assert!(looked_up.is_some());
        assert_eq!(*looked_up.unwrap(), square_effect);
    }
}
