# Pattern Matching Examples

Examples demonstrating pattern matching on algebraic data types.

## Pattern Matching Syntax

```cem
value match
  VariantName => [ # code when variant matches ]
  OtherVariant => [ # code for other variant ]
end
```

## Examples

### simple-variant.cem
Basic pattern matching on a simple variant type.

**Demonstrates**:
- Defining custom variant types
- Matching on unit variants (no data)
- Matching on variants with data

### nested-matching.cem
Matching on nested structures like `Cons(1, Cons(2, Nil))`.

**Demonstrates**:
- Matching multiple times on the same structure
- Extracting fields from multi-field variants
- Working with the tail of a list

### custom-pair-type.cem
Creating and using a custom Pair type.

**Demonstrates**:
- Defining custom multi-field types
- Pattern matching to extract both fields
- Using stack operations with extracted fields

## Multi-Field Variants

Variants can have multiple fields:

```cem
type Pair
  | P(Int, Int)

: add-pair ( Pair -- Int )
  match
    P => [  # Stack: ( first second )
      +   # Add them
    ]
  end ;
```

**Field extraction**:
When you match on a multi-field variant, the fields are pushed onto the stack in declaration order.

For `Cons(T, List(T))`:
- First field: `T` (the head)
- Second field: `List(T)` (the tail)

```cem
Cons => [  # Stack: ( head tail )
  swap drop  # Keep only head
]
```

## Pattern Matching is Exhaustive

You must handle all variants:

```cem
list match
  Cons => [ # handle non-empty list ]
  Nil  => [ # handle empty list ]
end
```

Missing a variant case will cause a runtime error.

## Running Examples

```bash
./target/release/cem compile examples/pattern-matching/simple-variant.cem -o simple
./simple
```
