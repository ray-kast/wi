use syn::{meta::ParseNestedMeta, Attribute, LitBool, Path, Token};

use crate::prelude::*;

pub const ATTRIBUTE_NAME: &str = "shibari";

#[derive(Default)]
#[must_use = "Call .finish() to finish error checking"]
pub struct AttributeArgs {
    crate_path: Option<(Span, Path)>,
    trap: Option<(Span, bool)>,
    advance: Option<(Span, bool)>,
    from: Option<(Span, bool)>,
}

impl AttributeArgs {
    fn parse_bool(meta: &ParseNestedMeta<'_>, default: bool) -> syn::Result<bool> {
        Ok(if meta.input.peek(Token![=]) {
            meta.value()?.parse::<LitBool>()?.value
        } else {
            default
        })
    }

    pub fn from_attrs(attrs: &[Attribute]) -> syn::Result<Self> {
        let mut this = Self::default();
        let mut found = false;

        let mut it = attrs.iter();
        for attr in &mut it {
            if this.parse_if_matches(attr)? {
                found = true;
                break;
            }
        }

        if found {
            for attr in &mut it {
                if attr.path().is_ident(ATTRIBUTE_NAME) {
                    return Err(attr.span().error("Duplicate attribute"));
                }
            }
        }

        Ok(this)
    }

    pub fn parse_if_matches(&mut self, attr: &Attribute) -> syn::Result<bool> {
        if !attr.path().is_ident(ATTRIBUTE_NAME) {
            return Ok(false);
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("advance") {
                self.advance = Some((meta.path.span(), Self::parse_bool(&meta, true)?));
            } else if meta.path.is_ident("crate") {
                self.crate_path = Some((meta.path.span(), meta.value()?.parse()?));
            } else if meta.path.is_ident("from") {
                self.from = Some((meta.path.span(), Self::parse_bool(&meta, true)?));
            } else if meta.path.is_ident("trap") {
                self.trap = Some((meta.path.span(), Self::parse_bool(&meta, true)?));
            } else {
                return Err(meta.error("Invalid argument"));
            }

            Ok(())
        })?;

        Ok(true)
    }

    #[inline]
    pub fn crate_path(&mut self) -> Option<Path> { self.crate_path.take().map(|(_, p)| p) }

    #[inline]
    pub fn trap(&mut self) -> Option<bool> { self.trap.take().map(|(_, b)| b) }

    #[inline]
    pub fn advance(&mut self) -> Option<bool> { self.advance.take().map(|(_, b)| b) }

    #[inline]
    pub fn from(&mut self) -> Option<bool> { self.from.take().map(|(_, b)| b) }

    pub fn finish(self, diag: &mut TokenStream) {
        let Self {
            crate_path,
            trap,
            advance,
            from,
        } = self;

        for span in [
            crate_path.map(|(s, _)| s),
            trap.map(|(s, _)| s),
            advance.map(|(s, _)| s),
            from.map(|(s, _)| s),
        ]
        .into_iter()
        .flatten()
        {
            diag.extend(
                span.error("Argument is not valid here")
                    .into_compile_error(),
            );
        }
    }
}
