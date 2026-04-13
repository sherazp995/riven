# Generics

Generics let you write code that works with multiple types while maintaining type safety.

## Generic Functions

Type parameters go in square brackets:

```riven
pub def identity[T](x: T) -> T
  x
end

let n = identity(42)          # T = Int
let s = identity("hello")    # T = &str
```

## Trait Bounds

Constrain type parameters with `:`:

```riven
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

### Multiple Bounds

Use `+` for multiple trait requirements:

```riven
pub def log_and_save[T: Displayable + Serializable](item: &T)
  puts item.to_display
  save(item.serialize)
end
```

## Generic Classes

```riven
class Stack[T]
  items: Vec[T]

  def init
    self.items = Vec.new
  end

  pub def mut push(item: T)
    self.items.push(item)
  end

  pub def mut pop -> Option[T]
    self.items.pop
  end

  pub def peek -> Option[&T]
    if self.items.is_empty
      None
    else
      Some(&self.items[self.items.len - 1])
    end
  end

  pub def is_empty -> Bool
    self.items.is_empty
  end
end
```

## Generic Structs

```riven
struct Pair[A, B]
  first: A
  second: B
end

let p = Pair.new(42, "hello")
```

## Generic Enums

```riven
enum Either[L, R]
  Left(L)
  Right(R)
end
```

`Option[T]` and `Result[T, E]` are generic enums built into the language.

## Where Clauses

For complex constraints:

```riven
pub def merge[A, B, C](left: &A, right: &B) -> C
  where A: Iterable[Item = Int],
        B: Iterable[Item = Int],
        C: FromIterator[Int]
  # ...
end
```

## Conditional Implementation

Add methods only when type parameters meet certain bounds:

```riven
# All Containers get these methods
impl[T] Container[T]
  pub def count -> Int
    self.items.len
  end
end

# Only Containers of Displayable types get print_all
impl[T: Displayable] Container[T]
  pub def print_all
    for item in self.items
      puts item.to_display
    end
  end
end
```

## Lifetime Parameters

When returning references, you may need lifetime annotations:

```riven
pub def longest['a](a: &'a String, b: &'a String) -> &'a String
  if a.len > b.len
    a
  else
    b
  end
end
```

Most of the time, lifetime elision rules handle this automatically and you don't need explicit annotations.
