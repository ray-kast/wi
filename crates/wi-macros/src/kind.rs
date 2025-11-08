use syn::{Arm, Attribute, Data, DeriveInput, Expr, Fields, Member, Token, Type};

use crate::prelude::*;

struct ExprTypeAttr {
    #[expect(unused)]
    const_token: Token![const],
    expr: Expr,
    #[expect(unused)]
    colon_token: Token![:],
    kind_ty: Type,
}

impl Parse for ExprTypeAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let const_token = input.parse()?;
        let expr = input.parse()?;
        let colon_token = input.parse()?;
        let kind_ty = input.parse()?;

        Ok(Self {
            const_token,
            expr,
            colon_token,
            kind_ty,
        })
    }
}

struct TypeAttr {
    kind_ty: Type,
}

impl Parse for TypeAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let kind_ty = input.parse()?;

        Ok(Self { kind_ty })
    }
}

enum StructAttr {
    ExprType(ExprTypeAttr),
    Type(TypeAttr),
}

impl Parse for StructAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(if input.lookahead1().peek(Token![const]) {
            Self::ExprType(input.parse()?)
        } else {
            Self::Type(input.parse()?)
        })
    }
}

struct FieldAttr {
    arms: Vec<Arm>,
}

impl Parse for FieldAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut arms = vec![];
        while !input.is_empty() {
            arms.push(input.parse()?);
        }

        Ok(Self { arms })
    }
}

pub fn run(input: DeriveInput) -> TokenStream {
    let span = input.span();
    let mut diag = TokenStream::new();
    let Some(attr) = get_attr(input.attrs, &mut diag) else {
        return input
            .ident
            .span()
            .error(match input.data {
                Data::Struct(_) => "Missing #[kind] attribute on struct",
                Data::Enum(_) => "Missing #[kind] attribute on enum",
                Data::Union(_) => "Missing #[kind] attribute on union",
            })
            .into_compile_error();
    };

    let ty = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let item = match input.data {
        Data::Struct(s) => 'fail: {
            let attr: StructAttr = match attr.parse_args() {
                Ok(a) => a,
                Err(e) => break 'fail e.into_compile_error(),
            };

            let (kind_ty, expr) = 'found: {
                let fields = match s.fields {
                    Fields::Named(n) => n.named,
                    Fields::Unnamed(u) => u.unnamed,
                    Fields::Unit => match attr {
                        StructAttr::ExprType(ExprTypeAttr {
                            const_token: _,
                            expr,
                            colon_token: _,
                            kind_ty,
                        }) => break 'found (kind_ty, Some(expr.into_token_stream())),
                        StructAttr::Type(TypeAttr { kind_ty }) => break 'found (kind_ty, None),
                    },
                };

                let kind_ty = match attr {
                    StructAttr::ExprType(ExprTypeAttr {
                        const_token: _,
                        expr,
                        colon_token: _,
                        kind_ty,
                    }) => break 'found (kind_ty, Some(expr.into_token_stream())),
                    StructAttr::Type(TypeAttr { kind_ty }) => kind_ty,
                };

                let Some((i, field)) = get_one(
                    fields
                        .into_iter()
                        .enumerate()
                        .filter(|(_, f)| f.attrs.iter().any(attr_matches)),
                    |(_, f)| {
                        diag.extend(
                            f.span()
                                .error("Only one field may have a #[kind] attribute")
                                .into_compile_error(),
                        );
                    },
                ) else {
                    break 'found (kind_ty, None);
                };
                let span = field.span();

                let memb = field.ident.map_or(Member::Unnamed(i.into()), Member::Named);

                let FieldAttr { arms } =
                    match get_attr(field.attrs, &mut diag).map(|a| a.parse_args()) {
                        Some(Ok(a)) => a,
                        Some(Err(e)) => break 'fail e.into_compile_error(),
                        None => unreachable!(),
                    };

                (
                    kind_ty,
                    Some(quote_spanned! { span => match self.#memb { #(#arms)* } }),
                )
            };

            let expr = expr.unwrap_or_else(|| {
                quote_spanned! { kind_ty.span() => #kind_ty::#ty }
            });

            quote_spanned! { span =>
                #[automatically_derived]
                impl
                    #impl_generics crate::actions::Kind<#kind_ty>
                for #ty #ty_generics
                    #where_clause
                {
                    #[inline]
                    fn kind(&self) -> #kind_ty { #expr }
                }
            }
        },
        Data::Enum(_) => todo!(),
        Data::Union(_) => span
            .error("Cannot derive Kind on unions")
            .into_compile_error(),
    };

    [diag, item].into_iter().collect()
}

fn get_one<I: IntoIterator>(it: I, err: impl FnOnce(I::Item)) -> Option<I::Item> {
    let mut it = it.into_iter();
    let i = it.next();
    it.next().map(err);
    i
}

#[inline]
fn attr_matches(attr: &Attribute) -> bool { attr.path().is_ident("kind") }

#[inline]
fn get_attr(
    attrs: impl IntoIterator<Item = Attribute>,
    diag: &mut TokenStream,
) -> Option<Attribute> {
    get_one(attrs.into_iter().filter(attr_matches), |a| {
        diag.extend(
            a.span()
                .error("Duplicate #[kind] attribute")
                .into_compile_error(),
        );
    })
}
