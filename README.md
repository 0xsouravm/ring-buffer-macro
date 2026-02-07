# ring-buffer-macro

[![Crates.io](https://img.shields.io/crates/v/ring-buffer-macro?style=flat-square)](https://crates.io/crates/ring-buffer-macro)
[![docs.rs](https://img.shields.io/docsrs/ring-buffer-macro?style=flat-square)](https://docs.rs/ring-buffer-macro)
[![License](https://img.shields.io/crates/l/ring-buffer-macro?style=flat-square)](LICENSE)
[![Website](https://img.shields.io/badge/website-ringbuf.dev-F97316?style=flat-square)](https://ringbuf.dev)

A proc macro that turns a tuple struct into a fixed-size ring buffer. Supports single-threaded, lock-free SPSC, and lock-free MPSC modes.

```toml
[dependencies]
ring-buffer-macro = "0.2.0"
```

## How it works

You write a tuple struct with one field indicating the element type:

```rust
#[ring_buffer(5)]
struct IntBuffer(i32);
```

The macro replaces this with a named struct (`data`, `head`, `tail`, etc.) and generates a full `impl` block with ring buffer methods. The tuple struct is just a way to specify the name and element type — the `(i32)` field gets thrown away.

For concurrent modes (SPSC/MPSC), the generated struct uses `UnsafeCell<Vec<MaybeUninit<T>>>` with atomic indices instead of plain `Vec<T>`, so items are moved rather than cloned. This drops the trait bound from `Clone` to `Send`.

## Usage

### Standard mode

```rust
use ring_buffer_macro::ring_buffer;

#[ring_buffer(5)]
struct IntBuffer(i32);

fn main() {
    let mut buf = IntBuffer::new();

    buf.enqueue(1).unwrap();
    buf.enqueue(2).unwrap();
    buf.enqueue(3).unwrap();

    assert_eq!(buf.peek(), Some(&1));
    assert_eq!(buf.peek_back(), Some(&3));

    for item in buf.iter() {
        println!("{}", item);
    }

    assert_eq!(buf.dequeue(), Some(1));

    // drain() removes items as it iterates
    let rest: Vec<_> = buf.drain().collect();
    assert!(buf.is_empty());
}
```

### Generics

Type parameters are preserved in the generated code:

```rust
#[ring_buffer(10)]
struct GenericBuffer<T: Clone>(T);

let mut buf: GenericBuffer<String> = GenericBuffer::new();
buf.enqueue("hello".to_string()).unwrap();
```

### SPSC (lock-free, single-producer/single-consumer)

Uses `AtomicUsize` head/tail with acquire/release ordering. No locks.

```rust
use ring_buffer_macro::ring_buffer;
use std::sync::Arc;
use std::thread;

#[ring_buffer(capacity = 1024, mode = "spsc")]
struct MessageQueue(String);

fn main() {
    let queue = Arc::new(MessageQueue::new());

    let q1 = Arc::clone(&queue);
    let producer = thread::spawn(move || {
        let (p, _) = q1.split();
        for i in 0..100 {
            while p.try_enqueue(format!("msg {}", i)).is_err() {}
        }
    });

    let q2 = Arc::clone(&queue);
    let consumer = thread::spawn(move || {
        let (_, c) = q2.split();
        for _ in 0..100 {
            while c.try_dequeue().is_none() {}
        }
    });

    producer.join().unwrap();
    consumer.join().unwrap();
}
```

### MPSC (multi-producer, single-consumer)

Producers coordinate via `compare_exchange_weak` on the tail index. Each slot has an `AtomicBool` flag so the consumer knows when a write is complete.

The producer handle is `Clone`; the consumer is not. Only one consumer should exist — this is a protocol constraint, not enforced at runtime.

```rust
use ring_buffer_macro::ring_buffer;
use std::sync::Arc;
use std::thread;

#[ring_buffer(capacity = 1024, mode = "mpsc")]
struct WorkQueue(i32);

fn main() {
    let queue = Arc::new(WorkQueue::new());
    let mut handles = vec![];

    for i in 0..4 {
        let q = Arc::clone(&queue);
        handles.push(thread::spawn(move || {
            let producer = q.producer();
            for j in 0..100 {
                while producer.try_enqueue(i * 100 + j).is_err() {}
            }
        }));
    }

    let q = Arc::clone(&queue);
    handles.push(thread::spawn(move || {
        let consumer = q.consumer();
        let mut count = 0;
        while count < 400 {
            if consumer.try_dequeue().is_some() {
                count += 1;
            }
        }
    }));

    for h in handles {
        h.join().unwrap();
    }
}
```

### Blocking mode

Available for both SPSC and MPSC. Uses a `Mutex<()>` + `Condvar` pair — the mutex doesn't protect data, it just satisfies the condvar API. The actual data path is still lock-free atomics.

```rust
use ring_buffer_macro::ring_buffer;

#[ring_buffer(capacity = 64, mode = "mpsc", blocking = true)]
struct BlockingQueue(String);

// These wait instead of returning Err/None
// producer.enqueue_blocking("message".to_string());
// let msg = consumer.dequeue_blocking();
```

### Power-of-two optimization

If your capacity is a power of two, this swaps modulo for bitwise AND on index wraparound. The macro enforces the constraint at compile time.

```rust
#[ring_buffer(capacity = 1024, power_of_two = true)]
struct FastBuffer(u8);
```

### Cache-line padding

Aligns head and tail to 64-byte boundaries to prevent false sharing when producer and consumer run on different cores. Mostly relevant for SPSC mode.

```rust
#[ring_buffer(capacity = 1024, mode = "spsc", cache_padded = true)]
struct PaddedQueue(u8);
```

## Configuration

| Option | Values | Default | Description |
|--------|--------|---------|-------------|
| `capacity` | positive integer | required | Maximum number of elements |
| `mode` | `"standard"`, `"spsc"`, `"mpsc"` | `"standard"` | Buffer mode |
| `power_of_two` | `true`, `false` | `false` | Bitwise indexing (capacity must be 2^n) |
| `cache_padded` | `true`, `false` | `false` | 64-byte align head/tail to avoid false sharing |
| `blocking` | `true`, `false` | `false` | Blocking enqueue/dequeue (concurrent modes only) |

```rust
// simple
#[ring_buffer(10)]

// named
#[ring_buffer(capacity = 1024, mode = "spsc", power_of_two = true, cache_padded = true)]
```

## Generated API

### Standard mode

- `new()` / `enqueue(item)` / `dequeue()` / `clear()`
- `peek()` / `peek_mut()` / `peek_back()`
- `iter()` / `drain()`
- `is_full()` / `is_empty()` / `len()` / `capacity()`

`dequeue()` and `drain()` require `T: Clone` (bound is on the method, not the struct, so you can create a buffer of non-Clone types — you just can't dequeue from it).

### SPSC mode

Buffer: `new()`, `split() -> (Producer, Consumer)`, `is_full()`, `is_empty()`, `len()`, `capacity()`

Producer: `try_enqueue(item)`, `enqueue_blocking(item)` (if blocking)

Consumer: `try_dequeue()`, `dequeue_blocking()` (if blocking), `peek()`

### MPSC mode

Buffer: `new()`, `producer()`, `consumer()`, `is_empty()`, `len()`, `capacity()`

Producer (clonable): `try_enqueue(item)`, `enqueue_blocking(item)` (if blocking), `is_full()`

Consumer: `try_dequeue()`, `dequeue_blocking()` (if blocking), `peek()`, `is_empty()`, `len()`

## Requirements

- Input must be a tuple struct with one field: `struct Buffer(i32)`
- Standard mode requires `T: Clone` (for dequeue/drain only)
- SPSC/MPSC modes require `T: Send`
- Capacity must be a positive integer

## License

MIT
