# Functions

## Defining Functions

Functions are defined with `def` and terminated with `end`:

```riven
def greet
  puts "Hello!"
end
```

The last expression is the implicit return value (like Ruby):

```riven
def double(x)
  x * 2
end
```

## Parameters and Return Types

Private functions can have fully inferred types:

```riven
def add(a, b)
  a + b
end
```

Public functions **must** have explicit type annotations (design principle P5 — Clarity At The Boundaries):

```riven
pub def add(a: Int, b: Int) -> Int
  a + b
end
```

## Early Return

Use `return` for early exit:

```riven
pub def find_positive(nums: &Vec[Int]) -> Option[Int]
  for n in nums
    if n > 0
      return Some(n)
    end
  end
  None
end
```

## Single-Expression Functions

Short functions can use brace syntax:

```riven
def double(x: Int) -> Int { x * 2 }
def is_even(n: Int) -> Bool { n % 2 == 0 }
```

## Visibility

| Modifier | Scope |
|----------|-------|
| (none) | Private to the current module |
| `pub` | Public — accessible from anywhere |
| `protected` | Accessible from subclasses |

```riven
def private_helper(x) ... end
pub def public_api(x: Int) -> Int ... end
protected def for_subclasses(x: Int) ... end
```

## Generic Functions

Use square brackets for type parameters:

```riven
pub def identity[T](x: T) -> T
  x
end

pub def largest[T: Comparable](list: &Vec[T]) -> &T
  let mut best = &list[0]
  for item in list
    if item > best
      best = item
    end
  end
  best
end
```

## Where Clauses

For complex generic bounds:

```riven
pub def merge[A, B, C](left: &A, right: &B) -> C
  where A: Iterable[Item = Int],
        B: Iterable[Item = Int],
        C: FromIterator[Int]
  # ...
end
```

## Class Methods vs Instance Methods

```riven
class User
  name: String

  def init(@name: String) end

  # Instance method — implicitly borrows &self
  pub def display -> String
    "User: #{self.name}"
  end

  # Mutable method — borrows &mut self
  pub def mut rename(name: String)
    self.name = name
  end

  # Consuming method — takes ownership of self
  pub def consume into_name -> String
    self.name
  end

  # Class method — no self
  def self.anonymous -> User
    User.new("Anonymous")
  end
end
```

### Self-Mode Summary

| Declaration | Self mode | Meaning |
|-------------|-----------|---------|
| `def method` | `&self` | Borrows self immutably |
| `def mut method` | `&mut self` | Borrows self mutably |
| `def consume method` | `self` | Takes ownership of self |
| `def self.method` | (none) | Class method, no self |
