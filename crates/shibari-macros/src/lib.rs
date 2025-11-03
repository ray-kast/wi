use proc_macro::TokenStream as TokenStream1;

mod static_acceptors;

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

#[proc_macro]
pub fn static_acceptors(input: TokenStream1) -> TokenStream1 {
    #![expect(clippy::let_and_return)]
    let out = static_acceptors::run(syn::parse_macro_input!(input)).into();
    // eprintln!("{out}");
    out
}
