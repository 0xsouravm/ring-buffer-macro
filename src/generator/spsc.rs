use crate::parser::RingBufferArgs;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields, Type};

// Emit spsc impl + producer/consumer handles
pub fn generate_spsc_impl(
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
    let blocking = args.blocking;

    let producer_name = format_ident!("{}Producer", struct_name);
    let consumer_name = format_ident!("{}Consumer", struct_name);
    let cache_padded_atomic_name = format_ident!("{}CachePaddedAtomic", struct_name);

    let next_index_impl = if args.power_of_two {
        quote! {
            #[inline]
            fn next_index(index: usize) -> usize { (index + 1) & #mask }
        }
    } else {
        quote! {
            #[inline]
            fn next_index(index: usize) -> usize {
                let next = index + 1;
                if next >= #capacity { 0 } else { next }
            }
        }
    };

    let len_calc = quote! {
        if tail >= head { tail - head } else { #capacity - head + tail }
    };

    let (cache_padded_type, head_init, tail_init) = if cache_padded {
        (
            quote! {
                #[repr(C, align(64))]
                #vis struct #cache_padded_atomic_name(std::sync::atomic::AtomicUsize);

                impl #cache_padded_atomic_name {
                    fn new(val: usize) -> Self { Self(std::sync::atomic::AtomicUsize::new(val)) }
                    #[inline]
                    fn load(&self, order: std::sync::atomic::Ordering) -> usize { self.0.load(order) }
                    #[inline]
                    fn store(&self, val: usize, order: std::sync::atomic::Ordering) { self.0.store(val, order) }
                }
            },
            quote! { #cache_padded_atomic_name::new(0) },
            quote! { #cache_padded_atomic_name::new(0) },
        )
    } else {
        (
            quote! {},
            quote! { AtomicUsize::new(0) },
            quote! { AtomicUsize::new(0) },
        )
    };

    let blocking_init = if blocking {
        quote! {
            mutex: std::sync::Mutex::new(()),
            not_empty: std::sync::Condvar::new(),
            not_full: std::sync::Condvar::new(),
        }
    } else {
        quote! {}
    };

    let producer_blocking_methods = if blocking {
        quote! {
            /// Blocks until an item can be enqueued.
            pub fn enqueue_blocking(&self, item: #element_type) {
                use std::sync::atomic::Ordering;
                use std::mem::MaybeUninit;

                let mut guard = self.buffer.mutex.lock().unwrap();
                loop {
                    let tail = self.buffer.tail.load(Ordering::Relaxed);
                    let next_tail = #struct_name::next_index(tail);
                    let head = self.buffer.head.load(Ordering::Acquire);

                    if next_tail != head {
                        unsafe {
                            let data = &mut *self.buffer.data.get();
                            data[tail] = MaybeUninit::new(item);
                        }
                        self.buffer.tail.store(next_tail, Ordering::Release);
                        drop(guard);
                        self.buffer.not_empty.notify_one();
                        return;
                    }

                    guard = self.buffer.not_full.wait(guard).unwrap();
                }
            }
        }
    } else {
        quote! {}
    };

    let consumer_blocking_methods = if blocking {
        quote! {
            /// Blocks until an item is available.
            pub fn dequeue_blocking(&self) -> #element_type {
                use std::sync::atomic::Ordering;

                let mut guard = self.buffer.mutex.lock().unwrap();
                loop {
                    let head = self.buffer.head.load(Ordering::Relaxed);
                    let tail = self.buffer.tail.load(Ordering::Acquire);

                    if head != tail {
                        let item = unsafe {
                            let data = &*self.buffer.data.get();
                            data[head].assume_init_read()
                        };
                        let next_head = #struct_name::next_index(head);
                        self.buffer.head.store(next_head, Ordering::Release);
                        drop(guard);
                        self.buffer.not_full.notify_one();
                        return item;
                    }

                    guard = self.buffer.not_empty.wait(guard).unwrap();
                }
            }
        }
    } else {
        quote! {}
    };

    quote! {
        #cache_padded_type

        impl #impl_generics #struct_name #ty_generics #where_clause {
            /// Creates a new empty SPSC ring buffer.
            #vis fn new() -> Self {
                use std::sync::atomic::AtomicUsize;
                use std::cell::UnsafeCell;
                use std::mem::MaybeUninit;

                let mut data = Vec::with_capacity(#capacity);
                for _ in 0..#capacity {
                    data.push(MaybeUninit::uninit());
                }

                Self {
                    data: UnsafeCell::new(data),
                    head: #head_init,
                    tail: #tail_init,
                    _marker: std::marker::PhantomData,
                    #blocking_init
                }
            }

            #[inline]
            #vis fn capacity(&self) -> usize { #capacity }

            #[inline]
            #vis fn is_empty(&self) -> bool {
                use std::sync::atomic::Ordering;
                self.head.load(Ordering::Relaxed) == self.tail.load(Ordering::Relaxed)
            }

            #[inline]
            #vis fn is_full(&self) -> bool {
                use std::sync::atomic::Ordering;
                let tail = self.tail.load(Ordering::Relaxed);
                let head = self.head.load(Ordering::Relaxed);
                Self::next_index(tail) == head
            }

            #[inline]
            #vis fn len(&self) -> usize {
                use std::sync::atomic::Ordering;
                let tail = self.tail.load(Ordering::Relaxed);
                let head = self.head.load(Ordering::Relaxed);
                #len_calc
            }

            #next_index_impl

            #vis fn split(&self) -> (#producer_name<'_>, #consumer_name<'_>) {
                (#producer_name { buffer: self }, #consumer_name { buffer: self })
            }
        }

        #vis struct #producer_name<'a> {
            buffer: &'a #struct_name #ty_generics,
        }

        impl<'a> #producer_name<'a> #where_clause {
            #[inline]
            pub fn try_enqueue(&self, item: #element_type) -> Result<(), #element_type> {
                use std::sync::atomic::Ordering;
                use std::mem::MaybeUninit;

                let tail = self.buffer.tail.load(Ordering::Relaxed);
                let next_tail = #struct_name::next_index(tail);
                let head = self.buffer.head.load(Ordering::Acquire);

                if next_tail == head { return Err(item); }

                unsafe {
                    let data = &mut *self.buffer.data.get();
                    data[tail] = MaybeUninit::new(item);
                }

                self.buffer.tail.store(next_tail, Ordering::Release);
                Ok(())
            }

            #[inline]
            pub fn is_full(&self) -> bool { self.buffer.is_full() }
            #[inline]
            pub fn len(&self) -> usize { self.buffer.len() }
            #[inline]
            pub fn is_empty(&self) -> bool { self.buffer.is_empty() }

            #producer_blocking_methods
        }

        #vis struct #consumer_name<'a> {
            buffer: &'a #struct_name #ty_generics,
        }

        impl<'a> #consumer_name<'a> #where_clause {
            #[inline]
            pub fn try_dequeue(&self) -> Option<#element_type> {
                use std::sync::atomic::Ordering;

                let head = self.buffer.head.load(Ordering::Relaxed);
                let tail = self.buffer.tail.load(Ordering::Acquire);

                if head == tail { return None; }

                let item = unsafe {
                    let data = &*self.buffer.data.get();
                    data[head].assume_init_read()
                };

                let next_head = #struct_name::next_index(head);
                self.buffer.head.store(next_head, Ordering::Release);

                Some(item)
            }

            #[inline]
            pub fn peek(&self) -> Option<&#element_type> {
                use std::sync::atomic::Ordering;

                let head = self.buffer.head.load(Ordering::Relaxed);
                let tail = self.buffer.tail.load(Ordering::Acquire);

                if head == tail { return None; }

                unsafe {
                    let data = &*self.buffer.data.get();
                    Some(data[head].assume_init_ref())
                }
            }

            #[inline]
            pub fn is_empty(&self) -> bool { self.buffer.is_empty() }
            #[inline]
            pub fn len(&self) -> usize { self.buffer.len() }
            #[inline]
            pub fn is_full(&self) -> bool { self.buffer.is_full() }

            #consumer_blocking_methods
        }

        unsafe impl #impl_generics Send for #struct_name #ty_generics where #element_type: Send {}
        unsafe impl #impl_generics Sync for #struct_name #ty_generics where #element_type: Send {}
        unsafe impl<'a> Send for #producer_name<'a> where #element_type: Send {}
        unsafe impl<'a> Send for #consumer_name<'a> where #element_type: Send {}
    }
}

// Replace tuple fields with atomic + UnsafeCell fields
pub fn add_spsc_fields(
    input: &mut DeriveInput,
    element_type: &Type,
    cache_padded: bool,
    blocking: bool,
) -> crate::error::Result<()> {
    let struct_name = &input.ident;
    let cache_padded_atomic_name = format_ident!("{}CachePaddedAtomic", struct_name);

    if let Data::Struct(data_struct) = &mut input.data {
        let data_field: syn::Field = syn::parse_quote! {
            data: std::cell::UnsafeCell<Vec<std::mem::MaybeUninit<#element_type>>>
        };

        let (head_field, tail_field): (syn::Field, syn::Field) = if cache_padded {
            (
                syn::parse_quote! { head: #cache_padded_atomic_name },
                syn::parse_quote! { tail: #cache_padded_atomic_name },
            )
        } else {
            (
                syn::parse_quote! { head: std::sync::atomic::AtomicUsize },
                syn::parse_quote! { tail: std::sync::atomic::AtomicUsize },
            )
        };

        let marker_field: syn::Field = syn::parse_quote! {
            _marker: std::marker::PhantomData<#element_type>
        };

        let named_fields: syn::FieldsNamed = syn::parse_quote! { { } };
        let mut fields = named_fields;
        fields.named.push(data_field);
        fields.named.push(head_field);
        fields.named.push(tail_field);
        fields.named.push(marker_field);

        if blocking {
            let mutex_field: syn::Field = syn::parse_quote! {
                mutex: std::sync::Mutex<()>
            };
            let not_empty_field: syn::Field = syn::parse_quote! {
                not_empty: std::sync::Condvar
            };
            let not_full_field: syn::Field = syn::parse_quote! {
                not_full: std::sync::Condvar
            };
            fields.named.push(mutex_field);
            fields.named.push(not_empty_field);
            fields.named.push(not_full_field);
        }

        data_struct.fields = Fields::Named(fields);
    }

    Ok(())
}
