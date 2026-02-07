// Add_*_fields: mutate struct, generate_*_impl: emit impl block
mod mpsc;
mod spsc;
mod standard;

pub use mpsc::{add_mpsc_fields, generate_mpsc_impl};
pub use spsc::{add_spsc_fields, generate_spsc_impl};
pub use standard::{add_fields, generate_impl};
