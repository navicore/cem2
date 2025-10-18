/// Integration tests for the parser
use super::*;
use crate::ast::Expr;

#[test]
fn test_parse_complete_program() {
    let input = r#"
        type Option (T)
          | Some(T)
          | None

        : unwrap ( Option(Int) Int -- Int )
          swap match
            Some => [ swap drop ]
            None => [ ]
          end ;
    "#;

    let mut parser = Parser::new(input);
    let result = parser.parse();

    assert!(result.is_ok(), "Parse failed: {:?}", result.err());
    let program = result.unwrap();

    assert_eq!(program.type_defs.len(), 1);
    assert_eq!(program.word_defs.len(), 1);

    // Check type definition
    assert_eq!(program.type_defs[0].name, "Option");
    assert_eq!(program.type_defs[0].variants.len(), 2);

    // Check word definition
    assert_eq!(program.word_defs[0].name, "unwrap");
}

#[test]
fn test_parse_pattern_match() {
    let input = r#"
        : handle ( Option(Int) -- Int )
          match
            Some => [ ]
            None => [ 0 ]
          end ;
    "#;

    let mut parser = Parser::new(input);
    let result = parser.parse();

    assert!(result.is_ok());
    let program = result.unwrap();

    assert_eq!(program.word_defs.len(), 1);

    // Check that body contains a match expression
    assert_eq!(program.word_defs[0].body.len(), 1);
    match &program.word_defs[0].body[0] {
        Expr::Match { branches, loc: _ } => {
            assert_eq!(branches.len(), 2);
        }
        _ => panic!("Expected Match expression"),
    }
}

#[test]
fn test_parse_if_expression() {
    let input = r#"
        : abs ( Int -- Int )
          dup 0 < if [ 0 swap - ] [ ] ;
    "#;

    let mut parser = Parser::new(input);
    let result = parser.parse();

    if let Err(e) = &result {
        eprintln!("Parse error: {}", e);
    }
    assert!(result.is_ok());
    let program = result.unwrap();

    // Body should be: dup, 0, <, if-expr
    assert_eq!(program.word_defs[0].body.len(), 4);
}

#[test]
fn test_parse_comments() {
    let input = r#"
        # This is a comment
        : square ( Int -- Int )
          # Duplicate and multiply
          dup * ;  # End of word
    "#;

    let mut parser = Parser::new(input);
    let result = parser.parse();

    assert!(result.is_ok());
    let program = result.unwrap();
    assert_eq!(program.word_defs.len(), 1);
}

#[test]
fn test_parse_multiple_words() {
    let input = r#"
        : double ( Int -- Int )
          2 * ;

        : quadruple ( Int -- Int )
          double double ;
    "#;

    let mut parser = Parser::new(input);
    let result = parser.parse();

    assert!(result.is_ok());
    let program = result.unwrap();
    assert_eq!(program.word_defs.len(), 2);
    assert_eq!(program.word_defs[0].name, "double");
    assert_eq!(program.word_defs[1].name, "quadruple");
}

#[test]
fn test_parse_polymorphic_effect() {
    let input = r#"
        : dup ( A -- A A )
          dup ;
    "#;

    let mut parser = Parser::new(input);
    let result = parser.parse();

    assert!(result.is_ok());
    let program = result.unwrap();

    // Check that effect signature parsed correctly
    let effect = &program.word_defs[0].effect;
    assert_eq!(effect.inputs.depth(), Some(1));
    assert_eq!(effect.outputs.depth(), Some(2));
}
