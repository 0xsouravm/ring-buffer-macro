use proc_macro2::Span;
use syn::Error as SynError;

/// Custom error type for ring buffer macro
#[derive(Debug)]
pub enum Error {
    NotAStruct(Span),
    NotNamedFields(Span),
    MissingDataField(Span),
    InvalidDataFieldType(Span),
    Syn(SynError),
}

impl Error {
    pub fn not_a_struct(span: Span) -> Self {
        Error::NotAStruct(span)
    }

    pub fn not_named_fields(span: Span) -> Self {
        Error::NotNamedFields(span)
    }

    pub fn missing_data_field(span: Span) -> Self {
        Error::MissingDataField(span)
    }

    pub fn invalid_data_field_type(span: Span) -> Self {
        Error::InvalidDataFieldType(span)
    }

    pub fn to_compile_error(&self) -> proc_macro2::TokenStream {
        let error = match self {
            Error::NotAStruct(span) => {
                SynError::new(*span, "ring_buffer can only be applied to structs")
            }
            Error::NotNamedFields(span) => SynError::new(
                *span,
                "ring_buffer only works with structs with named fields",
            ),
            Error::MissingDataField(span) => SynError::new(
                *span,
                "ring_buffer requires a field named 'data' of type Vec<T>",
            ),
            Error::InvalidDataFieldType(span) => {
                SynError::new(*span, "data field must be of type Vec<T>")
            }
            Error::Syn(err) => return err.to_compile_error(),
        };
        error.to_compile_error()
    }
}

impl From<SynError> for Error {
    fn from(err: SynError) -> Self {
        Error::Syn(err)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
