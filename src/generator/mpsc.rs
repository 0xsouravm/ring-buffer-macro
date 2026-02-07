use crate::parser::RingBufferArgs;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{DeriveInput, Type};

// Emit mpsc impl + CAS producer/consumer handles
pub fn generate_mpsc_impl(
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
    let blocking = args.blocking;

    let producer_name = format_ident!("{}Producer", struct_name);
    let consumer_name = format_ident!("{}Consumer", struct_name);

    let next_index = if args.power_of_two {
        quote! { (index + 1) & #mask }
    } else {
        quote! { if index + 1 >= #capacity { 0 } else { index + 1 } }
    };

    let len_calc = quote! {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Relaxed);
        if tail >= head { tail - head } else { #capacity - head + tail }
    };

    let blocking_methods = if blocking {
        quote! {
            /// Blocks until an item can be enqueued.
            pub fn enqueue_blocking(&self, mut item: #element_type) {
                let mut guard = self.buffer.mutex.lock().unwrap();
                loop {
                    match self.try_enqueue(item) {
                        Ok(()) => {
                            drop(guard);
                            self.buffer.not_empty.notify_one();
                            return;
                        }
                        Err(returned) => {
                            item = returned;
                            guard = self.buffer.not_full.wait(guard).unwrap();
                        }
                    }
                }
            }
        }
    } else {
        quote! {}
    };

    let blocking_consumer_methods = if blocking {
        quote! {
            /// Blocks until an item is available.
            pub fn dequeue_blocking(&self) -> #element_type {
                let mut guard = self.buffer.mutex.lock().unwrap();
                loop {
                    if let Some(item) = self.try_dequeue() {
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

    let blocking_init = if blocking {
        quote! {
            mutex: std::sync::Mutex::new(()),
            not_empty: std::sync::Condvar::new(),
            not_full: std::sync::Condvar::new(),
        }
    } else {
        quote! {}
    };

    quote! {
        impl #impl_generics #struct_name #ty_generics #where_clause {
            /// Creates a new empty MPSC ring buffer.
            #vis fn new() -> Self {
                use std::sync::atomic::{AtomicUsize, AtomicBool};
                use std::cell::UnsafeCell;
                use std::mem::MaybeUninit;

                let mut data = Vec::with_capacity(#capacity);
                let mut written = Vec::with_capacity(#capacity);
                for _ in 0..#capacity {
                    data.push(MaybeUninit::uninit());
                    written.push(AtomicBool::new(false));
                }

                Self {
                    data: UnsafeCell::new(data),
                    written: written.into_boxed_slice(),
                    head: AtomicUsize::new(0),
                    tail: AtomicUsize::new(0),
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
            #vis fn len(&self) -> usize {
                use std::sync::atomic::Ordering;
                #len_calc
            }

            /// Creates a new producer handle. Can be cloned for multiple producers.
            #vis fn producer(&self) -> #producer_name<'_> {
                #producer_name { buffer: self }
            }

            /// Creates the consumer handle. Only one consumer should exist.
            #vis fn consumer(&self) -> #consumer_name<'_> {
                #consumer_name { buffer: self }
            }
        }

        /// Producer handle for MPSC buffer. Can be cloned for multiple producers.
        #[derive(Clone)]
        #vis struct #producer_name<'a> {
            buffer: &'a #struct_name #ty_generics,
        }

        impl<'a> #producer_name<'a> #where_clause {
            /// Attempts to enqueue an item using CAS.
            #[inline]
            pub fn try_enqueue(&self, item: #element_type) -> Result<(), #element_type> {
                use std::sync::atomic::Ordering;
                use std::mem::MaybeUninit;

                loop {
                    let tail = self.buffer.tail.load(Ordering::Relaxed);
                    let next_tail = { let index = tail; #next_index };
                    let head = self.buffer.head.load(Ordering::Acquire);

                    if next_tail == head {
                        return Err(item); // Full
                    }

                    match self.buffer.tail.compare_exchange_weak(
                        tail, next_tail, Ordering::AcqRel, Ordering::Relaxed
                    ) {
                        Ok(_) => {
                            unsafe {
                                let data = &mut *self.buffer.data.get();
                                data[tail] = MaybeUninit::new(item);
                            }
                            self.buffer.written[tail].store(true, Ordering::Release);
                            return Ok(());
                        }
                        Err(_) => continue, // CAS failed, retry
                    }
                }
            }

            #[inline]
            pub fn is_full(&self) -> bool {
                use std::sync::atomic::Ordering;
                let tail = self.buffer.tail.load(Ordering::Relaxed);
                let next_tail = { let index = tail; #next_index };
                let head = self.buffer.head.load(Ordering::Acquire);
                next_tail == head
            }

            #blocking_methods
        }

        /// Consumer handle for MPSC buffer.
        #vis struct #consumer_name<'a> {
            buffer: &'a #struct_name #ty_generics,
        }

        impl<'a> #consumer_name<'a> #where_clause {
            /// Attempts to dequeue an item.
            #[inline]
            pub fn try_dequeue(&self) -> Option<#element_type> {
                use std::sync::atomic::Ordering;

                let head = self.buffer.head.load(Ordering::Relaxed);
                let tail = self.buffer.tail.load(Ordering::Acquire);

                if head == tail {
                    return None; // Empty
                }

                // Wait for the slot to be written
                while !self.buffer.written[head].load(Ordering::Acquire) {
                    std::hint::spin_loop();
                }

                let item = unsafe {
                    let data = &*self.buffer.data.get();
                    data[head].assume_init_read()
                };

                self.buffer.written[head].store(false, Ordering::Release);
                let next_head = { let index = head; #next_index };
                self.buffer.head.store(next_head, Ordering::Release);

                Some(item)
            }

            /// Peeks at the front item.
            #[inline]
            pub fn peek(&self) -> Option<&#element_type> {
                use std::sync::atomic::Ordering;

                let head = self.buffer.head.load(Ordering::Relaxed);
                let tail = self.buffer.tail.load(Ordering::Acquire);

                if head == tail { return None; }

                while !self.buffer.written[head].load(Ordering::Acquire) {
                    std::hint::spin_loop();
                }

                unsafe {
                    let data = &*self.buffer.data.get();
                    Some(data[head].assume_init_ref())
                }
            }

            #[inline]
            pub fn is_empty(&self) -> bool { self.buffer.is_empty() }

            #[inline]
            pub fn len(&self) -> usize { self.buffer.len() }

            #blocking_consumer_methods
        }

        unsafe impl #impl_generics Send for #struct_name #ty_generics where #element_type: Send {}
        unsafe impl #impl_generics Sync for #struct_name #ty_generics where #element_type: Send {}
        unsafe impl<'a> Send for #producer_name<'a> where #element_type: Send {}
        unsafe impl<'a> Sync for #producer_name<'a> where #element_type: Send {}
        unsafe impl<'a> Send for #consumer_name<'a> where #element_type: Send {}
    }
}

// Replace tuple fields with atomic + written flags
pub fn add_mpsc_fields(
    input: &mut syn::DeriveInput,
    element_type: &Type,
    blocking: bool,
) -> crate::error::Result<()> {
    use syn::{Data, Fields};

    if let Data::Struct(data_struct) = &mut input.data {
        let data_field: syn::Field = syn::parse_quote! {
            data: std::cell::UnsafeCell<Vec<std::mem::MaybeUninit<#element_type>>>
        };
        let written_field: syn::Field = syn::parse_quote! {
            written: Box<[std::sync::atomic::AtomicBool]>
        };
        let head_field: syn::Field = syn::parse_quote! {
            head: std::sync::atomic::AtomicUsize
        };
        let tail_field: syn::Field = syn::parse_quote! {
            tail: std::sync::atomic::AtomicUsize
        };
        let marker_field: syn::Field = syn::parse_quote! {
            _marker: std::marker::PhantomData<#element_type>
        };

        let named_fields: syn::FieldsNamed = syn::parse_quote! { { } };
        let mut fields = named_fields;
        fields.named.push(data_field);
        fields.named.push(written_field);
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
