//! A procedural macro for creating compile-time ring buffers (circular buffers).
//!
//! ## Usage
//!
//! ### Standard Mode (default)
//!
//! ```ignore
//! use ring_buffer_macro::ring_buffer;
//!
//! #[ring_buffer(5)]
//! struct IntBuffer(i32);
//!
//! let mut buf = IntBuffer::new();
//! buf.enqueue(1).unwrap();
//! buf.enqueue(2).unwrap();
//! assert_eq!(buf.dequeue(), Some(1));
//! ```
//!
//! ### SPSC Mode (lock-free, thread-safe)
//!
//! ```ignore
//! use ring_buffer_macro::ring_buffer;
//! use std::thread;
//!
//! #[ring_buffer(capacity = 1024, mode = "spsc")]
//! struct SpscBuffer(i32);
//!
//! let buffer = SpscBuffer::new();
//! let (producer, consumer) = buffer.split();
//!
//! // Producer and consumer can be sent to different threads
//! ```
//!
//! ### MPSC Mode (multi-producer, single-consumer)
//!
//! ```ignore
//! use ring_buffer_macro::ring_buffer;
//!
//! #[ring_buffer(capacity = 1024, mode = "mpsc")]
//! struct MpscBuffer(i32);
//!
//! let buffer = MpscBuffer::new();
//! let producer1 = buffer.producer();
//! let producer2 = buffer.producer(); // Can clone for more producers
//! let consumer = buffer.consumer();
//! ```
//!
//! ### Blocking Mode
//!
//! ```ignore
//! #[ring_buffer(capacity = 1024, mode = "mpsc", blocking = true)]
//! struct BlockingQueue(Message);
//!
//! // Use enqueue_blocking() and dequeue_blocking() for blocking operations
//! ```
//!
//! ## Generated Methods
//!
//! ### Standard Mode
//! - `new()` - Create empty buffer
//! - `enqueue(item: T) -> Result<(), T>` - Add item (returns `Err(item)` if full)
//! - `dequeue() -> Option<T>` - Remove oldest item (requires `T: Clone`)
//! - `is_full()`, `is_empty()`, `len()`, `capacity()`, `clear()`
//!
//! ### SPSC Mode
//! - `new()` - Create empty buffer
//! - `split()` - Get producer and consumer handles
//! - `is_full()`, `is_empty()`, `len()`, `capacity()`
//! - Producer: `try_enqueue(item: T) -> Result<(), T>`
//! - Consumer: `try_dequeue() -> Option<T>`
//!
//! ### MPSC Mode
//! - `new()` - Create empty buffer
//! - `producer()` - Get clonable producer handle
//! - `consumer()` - Get consumer handle
//! - Producer: `try_enqueue(item: T) -> Result<(), T>`, `enqueue_blocking(item: T)` (if blocking)
//! - Consumer: `try_dequeue() -> Option<T>`, `dequeue_blocking() -> T` (if blocking)
//!
//! ## Requirements
//!
//! - Tuple struct with one element type, e.g., `struct Buffer(i32);`
//! - Standard mode: Element type `T` must implement `Clone`
//! - SPSC/MPSC modes: Element type `T` must implement `Send`

mod error;
mod generator;
mod parser;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

use error::Result;
use generator::{
    add_fields, add_mpsc_fields, add_spsc_fields, generate_impl, generate_mpsc_impl,
    generate_spsc_impl,
};
use parser::{find_element_type, BufferMode, RingBufferArgs};

/// Transforms a tuple struct into a fixed-size FIFO ring buffer.
///
/// # Example
///
/// ```ignore
/// #[ring_buffer(10)]
/// struct MyBuffer(String);
/// ```
///
/// Generates a struct with fields: `data`, `capacity`, `head`, `tail`, `size`
///
/// Generates methods: `new()`, `enqueue()`, `dequeue()`, `is_full()`, `is_empty()`,
/// `len()`, `capacity()`, `clear()`
#[proc_macro_attribute]
pub fn ring_buffer(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as RingBufferArgs);
    let mut input = parse_macro_input!(input as DeriveInput);

    match expand_ring_buffer(args, &mut input) {
        Ok(tokens) => tokens,
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand_ring_buffer(args: RingBufferArgs, input: &mut DeriveInput) -> Result<TokenStream> {
    let element_type = find_element_type(input)?;

    let expanded = match args.mode {
        BufferMode::Standard => {
            add_fields(input, &element_type, args.cache_padded)?;
            let implementation = generate_impl(input, &element_type, &args);
            quote! {
                #input
                #implementation
            }
        }
        BufferMode::Spsc => {
            add_spsc_fields(input, &element_type, args.cache_padded, args.blocking)?;
            let implementation = generate_spsc_impl(input, &element_type, &args);
            quote! {
                #input
                #implementation
            }
        }
        BufferMode::Mpsc => {
            add_mpsc_fields(input, &element_type, args.blocking)?;
            let implementation = generate_mpsc_impl(input, &element_type, &args);
            quote! {
                #input
                #implementation
            }
        }
    };

    Ok(expanded.into())
}
