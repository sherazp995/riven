# Unsafe Code

Riven's safety guarantees are enforced by default (P1 — Implicit Safety, Explicit Danger). The `unsafe` keyword opts into operations that the compiler cannot verify.

## What Requires Unsafe

- Dereferencing raw pointers (`*T`, `*mut T`)
- Calling FFI functions
- Accessing mutable global state
- Performing unchecked type casts

## Unsafe Blocks

```riven
let ptr: *Int = get_raw_pointer()

# Must wrap pointer operations in unsafe
unsafe
  let value = *ptr              # dereference
end
```

## The `!` Convention

Methods that can panic use `!` suffix — they're safe but signal danger:

```riven
let value = option.unwrap!           # panics on None
let value = result.expect!("oops")   # panics on Err
```

This is a naming convention, not a language-level unsafe mechanism. `unwrap!` is valid safe code — it just might crash at runtime.

## Keeping Unsafe Minimal

The idiomatic approach is to create safe abstractions over unsafe code:

```riven
# Unsafe implementation detail
class SafeBuffer
  ptr: *mut UInt8
  len: Int

  def init(size: Int)
    unsafe
      self.ptr = malloc(size) as *mut UInt8
      self.len = size
    end
  end

  # Safe public API
  pub def get(index: Int) -> Option[UInt8]
    if index < 0 || index >= self.len
      None
    else
      unsafe
        Some(*(self.ptr + index))
      end
    end
  end

  impl Drop
    def drop
      unsafe
        free(self.ptr as *Void)
      end
    end
  end
end
```

Users of `SafeBuffer` never need to write `unsafe` — the type's API is fully safe.
