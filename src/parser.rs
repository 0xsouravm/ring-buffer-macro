use crate::error::{Error, Result};
use syn::{
    parse::Parse, parse::ParseStream, spanned::Spanned, Data, DeriveInput, Fields, LitInt, Type,
    TypePath,
};

/// Arguments for the ring_buffer attribute macro
pub struct RingBufferArgs {
    pub capacity: usize,
}

impl Parse for RingBufferArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let capacity_lit: LitInt = input.parse()?;
        let capacity = capacity_lit
            .base10_parse::<usize>()
            .map_err(|_| syn::Error::new(capacity_lit.span(), "capacity must be a valid usize"))?;

        if capacity == 0 {
            return Err(syn::Error::new(
                capacity_lit.span(),
                "capacity must be greater than 0",
            ));
        }

        Ok(RingBufferArgs { capacity })
    }
}

/// Extract the element type T from Vec<T>
pub fn extract_vec_element_type(ty: &Type) -> Result<Type> {
    if let Type::Path(TypePath { path, .. }) = ty {
        if let Some(segment) = path.segments.last() {
            if segment.ident != "Vec" {
                return Err(Error::invalid_data_field_type(segment.ident.span()));
            }

            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                if let Some(syn::GenericArgument::Type(element_type)) = args.args.first() {
                    return Ok(element_type.clone());
                }
            }
        }
    }

    Err(Error::invalid_data_field_type(ty.span()))
}

/// Find and validate the 'data' field in the struct
pub fn find_data_field(input: &DeriveInput) -> Result<Type> {
    let fields = match &input.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(fields) => fields,
            _ => return Err(Error::not_named_fields(input.ident.span())),
        },
        _ => return Err(Error::not_a_struct(input.ident.span())),
    };

    let data_field = fields
        .named
        .iter()
        .find(|f| f.ident.as_ref().map(|i| i == "data").unwrap_or(false));

    if let Some(field) = data_field {
        extract_vec_element_type(&field.ty)
    } else {
        Err(Error::missing_data_field(input.ident.span()))
    }
}
