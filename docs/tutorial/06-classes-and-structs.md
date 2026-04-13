# Classes and Structs

## Classes

Classes are heap-allocated, support inheritance, and have reference semantics for method dispatch.

```riven
class User
  name: String
  pub age: Int

  def init(@name: String, @age: Int)
  end

  pub def display -> String
    "#{self.name} (age #{self.age})"
  end
end

let user = User.new("Alice", 30)
puts user.display
```

### Field Visibility

| Modifier | Access |
|----------|--------|
| (none) | Private — only accessible within the class |
| `pub` | Public — accessible from anywhere |
| `protected` | Accessible from the class and its subclasses |

### Constructor Auto-Assign

The `@` prefix in constructor parameters automatically assigns to the field:

```riven
def init(@name: String, @age: Int)
end
# Equivalent to:
def init(name: String, age: Int)
  self.name = name
  self.age = age
end
```

### Method Self-Modes

```riven
class Account
  balance: Int

  def init(@balance: Int) end

  # &self — immutable borrow (default)
  pub def get_balance -> Int
    self.balance
  end

  # &mut self — mutable borrow
  pub def mut deposit(amount: Int)
    self.balance += amount
  end

  # self — takes ownership, consumes the instance
  pub def consume close -> Int
    puts "Account closed"
    self.balance
  end

  # No self — class method
  def self.create(initial: Int) -> Account
    Account.new(initial)
  end
end
```

### Inheritance

Classes can inherit from one parent with `<`:

```riven
class Animal
  name: String
  def init(@name: String) end
  pub def speak -> String { "..." }
end

class Dog < Animal
  pub def speak -> String
    "Woof! I'm #{self.name}"
  end
end

class Cat < Animal
  pub def speak -> String
    "Meow! I'm #{self.name}"
  end
end
```

### Inline Trait Implementation

Implement traits directly inside the class body:

```riven
class User
  name: String
  def init(@name: String) end

  impl Displayable
    def to_display -> String
      self.name.clone
    end
  end
end
```

## Structs

Structs are lightweight value types. No inheritance, no heap allocation by default.

```riven
struct Point
  x: Float
  y: Float
end

let p = Point.new(3.0, 4.0)
```

### Deriving Traits

Structs can derive `Copy` if all fields are Copy:

```riven
struct Color
  r: UInt8
  g: UInt8
  b: UInt8
  derive Copy, Clone
end

let red = Color.new(255, 0, 0)
let also_red = red               # copy, both valid
```

### Structs vs Classes

| Feature | Class | Struct |
|---------|-------|--------|
| Allocation | Heap | Stack (by default) |
| Inheritance | Yes (single) | No |
| Copy | No (unless all fields Copy) | Yes (with `derive Copy`) |
| Default semantics | Move | Move (Copy if derived) |
| Methods | Yes | Yes |
| Trait impl | Yes | Yes |

## Newtypes

Zero-cost wrapper types that create a distinct type from an existing one:

```riven
newtype UserId(Int)
newtype Email(String)

let id = UserId(42)
let email = Email(String.new("user@example.com"))

# UserId and Int are different types — can't mix them accidentally
```

## Generic Classes and Structs

```riven
class Container[T]
  items: Vec[T]

  def init
    self.items = Vec.new
  end

  pub def mut add(item: T)
    self.items.push(item)
  end

  pub def count -> Int
    self.items.len
  end
end

let mut box = Container[String].new
box.add(String.new("hello"))
```
