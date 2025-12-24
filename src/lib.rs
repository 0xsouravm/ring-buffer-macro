//! A procedural macro for creating compile-time ring buffers (circular buffers).
//!
//! ## Usage
//!
//! ```ignore
//! use ring_buffer_macro::ring_buffer;
//!
//! #[ring_buffer(5)]
//! struct IntBuffer {
//!     data: Vec<i32>,
//! }
//!
//! let mut buf = IntBuffer::new();
//! buf.enqueue(1).unwrap();
//! buf.enqueue(2).unwrap();
//! assert_eq!(buf.dequeue(), Some(1));
//! ```
//!
//! ## Generated Methods
//!
//! - `new()` - Create empty buffer
//! - `enqueue(item: T) -> Result<(), T>` - Add item (returns `Err(item)` if full)
//! - `dequeue() -> Option<T>` - Remove oldest item (requires `T: Clone`)
//! - `is_full()`, `is_empty()`, `len()`, `capacity()`, `clear()`
//!
//! ## Requirements
//!
//! - Struct must have a field named `data` of type `Vec<T>`
//! - Element type `T` must implement `Clone`

mod error;
mod generator;
mod parser;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

use error::Result;
use generator::{add_fields, generate_impl};
use parser::{find_data_field, RingBufferArgs};

/// Transforms a struct with a `Vec<T>` field into a fixed-size FIFO ring buffer.
///
/// # Example
///
/// ```ignore
/// #[ring_buffer(10)]
/// struct MyBuffer {
///     data: Vec<String>,
/// }
/// ```
///
/// Adds fields: `capacity`, `head`, `tail`, `size`
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
    let capacity = args.capacity;

    // Find and validate the data field
    let element_type = find_data_field(input)?;

    // Add the additional fields
    add_fields(input)?;

    // Generate the implementation
    let implementation = generate_impl(input, &element_type, capacity);

    let expanded = quote! {
        #input

        #implementation
    };

    Ok(expanded.into())
}
