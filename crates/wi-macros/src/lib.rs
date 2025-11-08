use proc_macro::TokenStream as TokenStream1;

mod impl_enum;
mod kind;

pub(crate) mod prelude {
    pub use proc_macro2::{Span, TokenStream};
    pub use quote::{quote_spanned, ToTokens};
    pub use syn::{
        parse::{Parse, ParseStream},
        spanned::Spanned,
    };

    pub trait SpanExt {
        fn error(self, msg: impl std::fmt::Display) -> syn::Error;
    }

    impl<T: Into<Span>> SpanExt for T {
        fn error(self, msg: impl std::fmt::Display) -> syn::Error {
            syn::Error::new(self.into(), msg)
        }
    }
}

#[proc_macro_attribute]
pub fn impl_enum(attr: TokenStream1, input: TokenStream1) -> TokenStream1 {
    #![expect(clippy::let_and_return)]

    let mut args = impl_enum::Args::default();
    let parser = args.parser();
    syn::parse_macro_input!(attr with parser);

    let out = impl_enum::run(args, syn::parse_macro_input!(input)).into();
    // eprintln!("{out}");
    out
}

#[proc_macro_derive(Kind, attributes(kind))]
pub fn kind(input: TokenStream1) -> TokenStream1 {
    #![expect(clippy::let_and_return)]
    let out = kind::run(syn::parse_macro_input!(input)).into();
    // eprintln!("{out}");
    out
}
