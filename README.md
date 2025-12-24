# ring-buffer-macro

<div align="center">

[![Crates.io](https://img.shields.io/crates/v/ring-buffer-macro?style=flat-square)](https://crates.io/crates/ring-buffer-macro)
[![docs.rs](https://img.shields.io/docsrs/ring-buffer-macro?style=flat-square)](https://docs.rs/ring-buffer-macro)
[![License](https://img.shields.io/crates/l/ring-buffer-macro?style=flat-square)](LICENSE)
[![Build Status](https://img.shields.io/github/actions/workflow/status/0xsouravm/ring-buffer-macro/ci.yml?style=flat-square)](https://github.com/0xsouravm/ring-buffer-macro/actions)
[![Downloads](https://img.shields.io/crates/d/ring-buffer-macro?style=flat-square)](https://crates.io/crates/ring-buffer-macro)

**A procedural macro for creating ring buffer (circular buffer) data structures at compile time**

[Documentation](https://docs.rs/ring-buffer-macro) | [Crates.io](https://crates.io/crates/ring-buffer-macro) | [Repository](https://github.com/0xsouravm/ring-buffer-macro)

</div>

---

## Overview

A ring buffer is a fixed-size FIFO (First-In-First-Out) data structure that efficiently reuses memory by wrapping around when it reaches the end. This macro generates all necessary fields and methods at compile time with zero runtime overhead.

## Features

- **Zero runtime overhead** - All code generation happens at compile time
- **Type safe** - Works with any type implementing `Clone`
- **Generic support** - Preserves type parameters and constraints
- **Visibility preservation** - Maintains your struct's visibility modifiers
- **Comprehensive API** - All standard ring buffer operations

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
ring-buffer-macro = "0.1.0"
```

## Quick Start

```rust
use ring_buffer_macro::ring_buffer;

#[ring_buffer(5)]
struct IntBuffer {
    data: Vec<i32>,
}

fn main() {
    let mut buf = IntBuffer::new();

    buf.enqueue(1).unwrap();
    buf.enqueue(2).unwrap();
    buf.enqueue(3).unwrap();

    assert_eq!(buf.dequeue(), Some(1));
    assert_eq!(buf.dequeue(), Some(2));
    assert_eq!(buf.len(), 1);
}
```

## How It Works

The `#[ring_buffer(capacity)]` attribute macro transforms your struct:

### Input
```rust
#[ring_buffer(5)]
struct Buffer {
    data: Vec<i32>,
}
```

### Output
Adds fields:
- `capacity: usize` - Maximum elements
- `head: usize` - Read position
- `tail: usize` - Write position
- `size: usize` - Current count

Generates methods:
- `new()` - Creates empty buffer
- `enqueue(item)` - Adds item, returns `Err(item)` if full
- `dequeue()` - Removes oldest item (requires `T: Clone`)
- `is_full()` - Checks if at capacity
- `is_empty()` - Checks if empty
- `len()` - Current element count
- `capacity()` - Maximum capacity
- `clear()` - Removes all elements

## Requirements

- Struct with named fields
- Field named `data` of type `Vec<T>`
- Element type `T` must implement `Clone`
- Capacity must be positive integer literal

## Performance

- **O(1)** enqueue and dequeue operations
- **Zero allocations** after initialization
- **No runtime overhead** - everything generated at compile time
- **Cache-friendly** - contiguous memory access

## Use Cases

- Logging systems with fixed-size buffers
- Audio/video sample buffers
- Network packet queues
- Embedded systems with constrained resources
- Rate limiting request queues
- Producer-consumer patterns

## Contributing

Contributions are welcome! Please open an issue or submit a pull request.

## License

Licensed under the MIT License. See [LICENSE](LICENSE) for details.

---

<div align="center">
Built with Rust
</div>
