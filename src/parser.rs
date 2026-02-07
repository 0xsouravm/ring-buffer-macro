use crate::error::{Error, Result};
use syn::{
    parse::Parse, parse::ParseStream, Data, DeriveInput, Fields, Ident, LitBool, LitInt, LitStr,
    Token, Type,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BufferMode {
    #[default]
    Standard,
    Spsc,
    Mpsc,
}

// Parsed macro attribute arguments
pub struct RingBufferArgs {
    pub capacity: usize,
    pub mode: BufferMode,
    pub power_of_two: bool,
    pub cache_padded: bool,
    pub blocking: bool,
}

impl RingBufferArgs {
    pub fn index_mask(&self) -> usize {
        self.capacity - 1
    }

    fn is_power_of_two(n: usize) -> bool {
        n > 0 && (n & (n - 1)) == 0
    }
}

impl Parse for RingBufferArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Try to parse as named parameters first: #[ring_buffer(capacity = 5, mode = "spsc")]
        // Or as simple integer: #[ring_buffer(5)]

        let lookahead = input.lookahead1();

        if lookahead.peek(Ident) {
            // Named parameter style
            let mut capacity: Option<usize> = None;
            let mut capacity_span = input.span();
            let mut mode = BufferMode::Standard;
            let mut power_of_two = false;
            let mut cache_padded = false;
            let mut blocking = false;

            while !input.is_empty() {
                let key: Ident = input.parse()?;
                input.parse::<Token![=]>()?;

                match key.to_string().as_str() {
                    "capacity" => {
                        let lit: LitInt = input.parse()?;
                        capacity_span = lit.span();
                        let cap = lit.base10_parse::<usize>().map_err(|_| {
                            syn::Error::new(lit.span(), "capacity must be a valid usize")
                        })?;
                        if cap == 0 {
                            return Err(syn::Error::new(
                                lit.span(),
                                "capacity must be greater than 0",
                            ));
                        }
                        capacity = Some(cap);
                    }
                    "mode" => {
                        let lit: LitStr = input.parse()?;
                        mode = match lit.value().as_str() {
                            "standard" => BufferMode::Standard,
                            "spsc" => BufferMode::Spsc,
                            "mpsc" => BufferMode::Mpsc,
                            _ => {
                                return Err(syn::Error::new(
                                    lit.span(),
                                    "mode must be \"standard\", \"spsc\", or \"mpsc\"",
                                ))
                            }
                        };
                    }
                    "power_of_two" => {
                        let lit: LitBool = input.parse()?;
                        power_of_two = lit.value();
                    }
                    "cache_padded" => {
                        let lit: LitBool = input.parse()?;
                        cache_padded = lit.value();
                    }
                    "blocking" => {
                        let lit: LitBool = input.parse()?;
                        blocking = lit.value();
                    }
                    _ => {
                        return Err(syn::Error::new(
                            key.span(),
                            format!("unknown parameter: {}", key),
                        ))
                    }
                }

                // Parse optional comma
                if input.peek(Token![,]) {
                    input.parse::<Token![,]>()?;
                }
            }

            let capacity = capacity
                .ok_or_else(|| syn::Error::new(input.span(), "capacity parameter is required"))?;

            // Validate power_of_two constraint
            if power_of_two && !Self::is_power_of_two(capacity) {
                return Err(syn::Error::new(
                    capacity_span,
                    format!(
                        "capacity must be a power of two when power_of_two = true (got {})",
                        capacity
                    ),
                ));
            }

            Ok(RingBufferArgs {
                capacity,
                mode,
                power_of_two,
                cache_padded,
                blocking,
            })
        } else if lookahead.peek(LitInt) {
            // Simple integer style: #[ring_buffer(5)]
            let capacity_lit: LitInt = input.parse()?;
            let capacity = capacity_lit.base10_parse::<usize>().map_err(|_| {
                syn::Error::new(capacity_lit.span(), "capacity must be a valid usize")
            })?;

            if capacity == 0 {
                return Err(syn::Error::new(
                    capacity_lit.span(),
                    "capacity must be greater than 0",
                ));
            }

            Ok(RingBufferArgs {
                capacity,
                mode: BufferMode::Standard,
                power_of_two: false,
                cache_padded: false,
                blocking: false,
            })
        } else {
            Err(lookahead.error())
        }
    }
}

// Extract T from struct Foo(T)
pub fn find_element_type(input: &DeriveInput) -> Result<Type> {
    match &input.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                Ok(fields.unnamed.first().unwrap().ty.clone())
            }
            Fields::Unnamed(_) => Err(Error::invalid_tuple_struct(input.ident.span())),
            Fields::Named(_) => Err(Error::not_tuple_struct(input.ident.span())),
            Fields::Unit => Err(Error::not_tuple_struct(input.ident.span())),
        },
        _ => Err(Error::not_a_struct(input.ident.span())),
    }
}
