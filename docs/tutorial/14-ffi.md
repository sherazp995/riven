# FFI (Foreign Function Interface)

Riven can call C libraries directly using `lib` blocks and `extern` blocks.

## Declaring External Libraries

Use `lib` to declare functions from a C library:

```riven
@[link("m")]
lib LibM
  def sin(x: Float) -> Float
  def cos(x: Float) -> Float
  def sqrt(x: Float) -> Float
end

let result = LibM.sin(3.14159 / 2.0)
puts result    # ~1.0
```

### Link Attributes

The `@[link(...)]` attribute tells the linker which library to link:

```riven
@[link("sqlite3")]
lib Sqlite
  def sqlite3_open(filename: *UInt8, db: *mut *Void) -> Int32
  def sqlite3_close(db: *Void) -> Int32
end
```

## Extern Blocks

For lower-level C interop:

```riven
extern "C"
  def puts(s: *UInt8) -> Int32
  def malloc(size: USize) -> *Void
  def free(ptr: *Void)
end
```

## Raw Pointers

FFI uses raw pointers (`*T`, `*mut T`) which are `unsafe`:

```riven
unsafe
  let ptr = malloc(1024) as *mut UInt8
  # ... use ptr ...
  free(ptr as *Void)
end
```

## Variadic Functions

C functions with `...` in the parameter list:

```riven
extern "C"
  def printf(fmt: *UInt8, ...) -> Int32
end
```

## Safety

All FFI calls are inherently unsafe — the compiler cannot verify memory safety across the language boundary. Wrap FFI calls in safe Riven APIs:

```riven
@[link("m")]
lib LibM
  def sqrt(x: Float) -> Float
end

# Safe wrapper
pub def sqrt(x: Float) -> Result[Float, String]
  if x < 0.0
    Err(String.new("cannot take sqrt of negative number"))
  else
    Ok(LibM.sqrt(x))
  end
end
```
