use crate::error::Result;
use crate::parser::RingBufferArgs;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields, FieldsNamed, Type};

// Replace tuple fields with named fields
pub fn add_fields(input: &mut DeriveInput, element_type: &Type, cache_padded: bool) -> Result<()> {
    let struct_name = &input.ident;
    let cache_padded_type_name = format_ident!("{}CachePadded", struct_name);

    if let Data::Struct(data_struct) = &mut input.data {
        let data_field: syn::Field = syn::parse_quote! { data: Vec<#element_type> };
        let capacity_field: syn::Field = syn::parse_quote! { capacity: usize };
        let size_field: syn::Field = syn::parse_quote! { size: usize };

        let (head_field, tail_field): (syn::Field, syn::Field) = if cache_padded {
            (
                syn::parse_quote! { head: #cache_padded_type_name },
                syn::parse_quote! { tail: #cache_padded_type_name },
            )
        } else {
            (
                syn::parse_quote! { head: usize },
                syn::parse_quote! { tail: usize },
            )
        };

        let named_fields: FieldsNamed = syn::parse_quote! { { } };
        let mut fields = named_fields;
        fields.named.push(data_field);
        fields.named.push(capacity_field);
        fields.named.push(head_field);
        fields.named.push(tail_field);
        fields.named.push(size_field);

        data_struct.fields = Fields::Named(fields);
    }

    Ok(())
}

// Emit standard mode impl + drain iterator
pub fn generate_impl(
    input: &DeriveInput,
    element_type: &Type,
    args: &RingBufferArgs,
) -> TokenStream {
    let struct_name = &input.ident;
    let vis = &input.vis;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let capacity = args.capacity;
    let mask = args.index_mask();
    let cache_padded = args.cache_padded;

    let clone_bound = quote! { where #element_type: Clone };
    let drain_name = format_ident!("{}Drain", struct_name);
    let cache_padded_type_name = format_ident!("{}CachePadded", struct_name);

    let (head_get, tail_get, head_set, tail_set, head_init, tail_init) = if cache_padded {
        (
            quote! { self.head.0 },
            quote! { self.tail.0 },
            quote! { self.head.0 = },
            quote! { self.tail.0 = },
            quote! { #cache_padded_type_name(0) },
            quote! { #cache_padded_type_name(0) },
        )
    } else {
        (
            quote! { self.head },
            quote! { self.tail },
            quote! { self.head = },
            quote! { self.tail = },
            quote! { 0 },
            quote! { 0 },
        )
    };

    let next_head = if args.power_of_two {
        quote! { (#head_get + 1) & #mask }
    } else {
        quote! { (#head_get + 1) % self.capacity }
    };

    let next_tail = if args.power_of_two {
        quote! { (#tail_get + 1) & #mask }
    } else {
        quote! { (#tail_get + 1) % self.capacity }
    };

    let cache_padded_type = if cache_padded {
        quote! {
            #[repr(C, align(64))]
            #[derive(Debug, Clone, Copy, Default)]
            #vis struct #cache_padded_type_name(usize);
        }
    } else {
        quote! {}
    };

    quote! {
        #cache_padded_type

        impl #impl_generics #struct_name #ty_generics #where_clause {
            /// Creates a new empty ring buffer.
            #vis fn new() -> Self {
                Self {
                    data: Vec::with_capacity(#capacity),
                    capacity: #capacity,
                    head: #head_init,
                    tail: #tail_init,
                    size: 0,
                }
            }

            /// Adds an item to the back of the buffer.
            #vis fn enqueue(&mut self, item: #element_type) -> Result<(), #element_type> {
                if self.is_full() {
                    return Err(item);
                }

                let tail = #tail_get;
                if self.data.len() <= tail {
                    self.data.push(item);
                } else {
                    self.data[tail] = item;
                }

                #tail_set #next_tail;
                self.size += 1;
                Ok(())
            }

            /// Removes and returns the item at the front of the buffer.
            #vis fn dequeue(&mut self) -> Option<#element_type>
                #clone_bound
            {
                if self.is_empty() {
                    return None;
                }

                let head = #head_get;
                let item = self.data[head].clone();
                #head_set #next_head;
                self.size -= 1;

                Some(item)
            }

            /// Returns a reference to the front item without removing it.
            #[inline]
            #vis fn peek(&self) -> Option<&#element_type> {
                if self.is_empty() { None } else { Some(&self.data[#head_get]) }
            }

            /// Returns a mutable reference to the front item without removing it.
            #[inline]
            #vis fn peek_mut(&mut self) -> Option<&mut #element_type> {
                if self.is_empty() { None } else { let h = #head_get; Some(&mut self.data[h]) }
            }

            /// Returns a reference to the back item (most recently added).
            #[inline]
            #vis fn peek_back(&self) -> Option<&#element_type> {
                if self.is_empty() {
                    None
                } else {
                    let tail = #tail_get;
                    let idx = if tail == 0 { self.capacity - 1 } else { tail - 1 };
                    if idx < self.data.len() { Some(&self.data[idx]) } else { None }
                }
            }

            /// Returns `true` if the buffer is full.
            #[inline]
            #vis fn is_full(&self) -> bool { self.size == self.capacity }

            /// Returns `true` if the buffer is empty.
            #[inline]
            #vis fn is_empty(&self) -> bool { self.size == 0 }

            /// Returns the number of items in the buffer.
            #[inline]
            #vis fn len(&self) -> usize { self.size }

            /// Returns the maximum capacity of the buffer.
            #[inline]
            #vis fn capacity(&self) -> usize { self.capacity }

            /// Removes all items from the buffer.
            #vis fn clear(&mut self) {
                #head_set 0;
                #tail_set 0;
                self.size = 0;
            }

            /// Returns an iterator over references to the items in the buffer.
            #vis fn iter(&self) -> impl Iterator<Item = &#element_type> {
                let head = #head_get;
                let size = self.size;
                let cap = self.capacity;
                (0..size).map(move |i| &self.data[(head + i) % cap])
            }

            /// Returns an iterator that removes and yields all items from the buffer.
            #vis fn drain(&mut self) -> #drain_name #ty_generics
                #clone_bound
            {
                #drain_name { buffer: self }
            }
        }

        /// Draining iterator that removes and yields all items.
        #vis struct #drain_name #generics {
            buffer: *mut #struct_name #ty_generics,
        }

        impl #impl_generics Iterator for #drain_name #ty_generics
            #clone_bound
        {
            type Item = #element_type;

            fn next(&mut self) -> Option<Self::Item> {
                unsafe { (*self.buffer).dequeue() }
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                let len = unsafe { (*self.buffer).len() };
                (len, Some(len))
            }
        }

        impl #impl_generics ExactSizeIterator for #drain_name #ty_generics
            #clone_bound
        {}

        impl #impl_generics Drop for #drain_name #ty_generics
            #clone_bound
        {
            fn drop(&mut self) {
                unsafe { while (*self.buffer).dequeue().is_some() {} }
            }
        }
    }
}
