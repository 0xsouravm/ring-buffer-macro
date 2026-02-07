use proc_macro2::Span;
use syn::Error as SynError;

// Spans point errors at user code
#[derive(Debug)]
pub enum Error {
    NotAStruct(Span),
    NotTupleStruct(Span),
    InvalidTupleStruct(Span),
    Syn(SynError),
}

impl Error {
    pub fn not_a_struct(span: Span) -> Self {
        Error::NotAStruct(span)
    }

    pub fn not_tuple_struct(span: Span) -> Self {
        Error::NotTupleStruct(span)
    }

    pub fn invalid_tuple_struct(span: Span) -> Self {
        Error::InvalidTupleStruct(span)
    }

    pub fn to_compile_error(&self) -> proc_macro2::TokenStream {
        let error = match self {
            Error::NotAStruct(span) => {
                SynError::new(*span, "ring_buffer can only be applied to structs")
            }
            Error::NotTupleStruct(span) => SynError::new(
                *span,
                "ring_buffer requires a tuple struct with one type, e.g., struct Buffer(i32);",
            ),
            Error::InvalidTupleStruct(span) => SynError::new(
                *span,
                "ring_buffer tuple struct must have exactly one field, e.g., struct Buffer(i32);",
            ),
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
