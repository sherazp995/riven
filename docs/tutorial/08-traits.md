# Traits

Traits define shared behavior — a contract that types can implement.

## Defining a Trait

```riven
trait Displayable
  def to_display -> String
end
```

### Default Methods

Traits can provide default implementations:

```riven
trait Greetable
  def name -> String                        # required
  def greet -> String                       # default
    "Hello, #{self.name}!"
  end
end
```

### Associated Types

```riven
trait Iterator
  type Item
  def mut next -> Option[Self.Item]
end
```

### Trait Inheritance

```riven
trait Serializable: Displayable
  def serialize -> String
  def self.deserialize(data: &str) -> Result[Self, Error]
end
```

## Implementing Traits

### Standalone `impl` Block

```riven
impl Displayable for User
  def to_display -> String
    "#{self.name} <#{self.email}>"
  end
end
```

### Inline in Class Body

```riven
class User
  name: String
  email: String

  def init(@name: String, @email: String) end

  impl Displayable
    def to_display -> String
      "#{self.name} <#{self.email}>"
    end
  end
end
```

## Using Traits as Bounds

### Static Dispatch (`impl Trait`)

The compiler generates specialized code for each concrete type. No runtime overhead.

```riven
pub def print_it(item: &impl Displayable)
  puts item.to_display
end
```

### Generic Functions with Bounds

```riven
pub def largest[T: Comparable](list: &Vec[T]) -> &T
  # ...
end

# Multiple bounds with +
pub def process[T: Displayable + Serializable](item: &T)
  # ...
end
```

### Dynamic Dispatch (`dyn Trait`)

Uses a vtable at runtime. Required when the concrete type isn't known at compile time.

```riven
pub def print_dyn(item: &dyn Displayable)
  puts item.to_display
end
```

**Key difference**: `impl Trait` allows structural satisfaction (type just needs matching methods). `dyn Trait` requires an explicit `impl ... for ...` block.

## Built-in Traits

| Trait | Purpose |
|-------|---------|
| `Displayable` | Convert to display string |
| `Debug` | Debug representation |
| `Comparable` | Ordering (`<`, `>`, `<=`, `>=`) |
| `Hashable` | Hash computation (for `Hash` keys) |
| `Iterable` | Can produce an iterator |
| `Iterator` | Can yield successive items |
| `Copy` | Assignment duplicates the value |
| `Clone` | Explicit `.clone` deep copy |
| `Drop` | Custom destructor logic |
| `Error` | Error type with `.message` |

## Conditional Implementation

Implement traits only when type parameters satisfy certain bounds:

```riven
impl[T: Displayable] Container[T]
  pub def print_all
    for item in self.items
      puts item.to_display
    end
  end
end
```

## Deriving Traits

Some traits can be auto-derived:

```riven
struct Point
  x: Float
  y: Float
  derive Copy, Clone, Debug
end
```
