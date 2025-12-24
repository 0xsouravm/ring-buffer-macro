use crate::error::Result;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Type};

/// Add required fields to the struct
pub fn add_fields(input: &mut DeriveInput) -> Result<()> {
    if let Data::Struct(data_struct) = &mut input.data {
        if let Fields::Named(fields) = &mut data_struct.fields {
            let capacity_field: syn::Field = syn::parse_quote! { capacity: usize };
            let head_field: syn::Field = syn::parse_quote! { head: usize };
            let tail_field: syn::Field = syn::parse_quote! { tail: usize };
            let size_field: syn::Field = syn::parse_quote! { size: usize };

            fields.named.push(capacity_field);
            fields.named.push(head_field);
            fields.named.push(tail_field);
            fields.named.push(size_field);
        }
    }

    Ok(())
}

/// Generate the implementation block for the ring buffer
pub fn generate_impl(input: &DeriveInput, element_type: &Type, capacity: usize) -> TokenStream {
    let struct_name = &input.ident;
    let vis = &input.vis;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Build where clause for Clone bound on element type
    let clone_bound = quote! { where #element_type: Clone };

    quote! {
        impl #impl_generics #struct_name #ty_generics #where_clause {
            #vis fn new() -> Self {
                Self {
                    data: Vec::with_capacity(#capacity),
                    capacity: #capacity,
                    head: 0,
                    tail: 0,
                    size: 0,
                }
            }

            #vis fn enqueue(&mut self, item: #element_type) -> Result<(), #element_type> {
                if self.is_full() {
                    return Err(item);
                }

                if self.data.len() <= self.tail {
                    self.data.push(item);
                } else {
                    self.data[self.tail] = item;
                }

                self.tail = (self.tail + 1) % self.capacity;
                self.size += 1;
                Ok(())
            }

            #vis fn dequeue(&mut self) -> Option<#element_type>
                #clone_bound
            {
                if self.is_empty() {
                    return None;
                }

                let item = self.data[self.head].clone();
                self.head = (self.head + 1) % self.capacity;
                self.size -= 1;

                Some(item)
            }

            #vis fn is_full(&self) -> bool {
                self.size == self.capacity
            }

            #vis fn is_empty(&self) -> bool {
                self.size == 0
            }

            #vis fn len(&self) -> usize {
                self.size
            }

            #vis fn capacity(&self) -> usize {
                self.capacity
            }

            #vis fn clear(&mut self) {
                self.head = 0;
                self.tail = 0;
                self.size = 0;
            }
        }
    }
}
