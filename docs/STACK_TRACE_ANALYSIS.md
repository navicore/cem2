# Stack Pointer Trace Analysis

## The Failing Pattern

```cem
match
  Cons => [
    # Stack after match extraction: ( acc head tail )
    rot swap rot rot swap Cons
  ]
end
```

## Detailed Trace

### After Match Extraction

Match extracts Cons(10, Nil) and puts on stack with acc:

```
Stack cells (with next pointers):
- head_cell: { data: 10, next: &tail_cell }
- tail_cell: { data: Nil, next: &rest }
- rest (acc): { data: Nil, next: null }

Stack pointer: &head_cell
```

So stack is: head -> tail -> acc (reading next pointers)
Or in terms of values: ( acc head tail ) reading top-to-bottom

### Operation 1: `rot`

```rust
// rot: ( A B C -- B C A )
pop C (head)
pop B (tail)
pop A (acc)
push B (tail)    // tail.next = null
push C (head)    // head.next = &tail
push A (acc)     // acc.next = &head
```

Result stack pointer: &acc
Stack: acc -> head -> tail (next pointers)
Values: ( tail head acc ) top-to-bottom

### Operation 2: `swap`

```rust
// swap: ( ... B A -- ... A B )
pop A (acc)
pop B (head)
push A (acc)     // acc.next = &tail
push B (head)    // head.next = &acc
```

Result stack pointer: &head
Stack: head -> acc -> tail (next pointers)
Values: ( tail acc head ) top-to-bottom

### Operation 3: `rot`

```rust
pop C (head)
pop B (acc)
pop A (tail)
push B (acc)     // acc.next = null
push C (head)    // head.next = &acc
push A (tail)    // tail.next = &head
```

Result stack pointer: &tail
Stack: tail -> head -> acc (next pointers)
Values: ( acc head tail ) top-to-bottom

### Operation 4: `rot`

```rust
pop C (tail)
pop B (head)
pop A (acc)
push B (head)    // head.next = null
push C (tail)    // tail.next = &head
push A (acc)     // acc.next = &tail
```

Result stack pointer: &acc
Stack: acc -> tail -> head (next pointers)
Values: ( head tail acc ) top-to-bottom

### Operation 5: `swap`

```rust
pop A (acc)
pop B (tail)
push A (acc)     // acc.next = &head
push B (tail)    // tail.next = &acc
```

Result stack pointer: &tail
Stack: tail -> acc -> head (next pointers)
Values: ( head acc tail ) top-to-bottom

### Now: `Cons`

We want to construct Cons(head, acc).

Stack is: tail -> acc -> head (next pointers)

Cons construction does:
```llvm
%17 = (stack pointer = &tail)
copy from %17           // Copies TAIL (WRONG! We want HEAD)
%19 = &(%17.next)       // Gets &tail.next
%20 = load %19          // Loads tail.next = &acc (WRONG! We want ACC as second field but HEAD as first)
copy from %20           // Copies acc
```

## THE BUG

After all the shuffling, stack pointer points to `tail`, and:
- `tail.next` = `acc`
- `acc.next` = `head`

So Cons construction copies (tail, acc) when we want (head, acc)!

The bug is that Cons expects the values in stack positions 0 and 1, but they're actually in positions 2 and 1!

The stack is: ( head acc tail ) reading top-to-bottom
But the next pointers go: tail -> acc -> head

**ROOT CAUSE:** After `swap`, the stack pointer and the stack ORDER are now inverted from the cell chain direction!
