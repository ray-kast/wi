use syn::{
    Arm, Attribute, Data, DataEnum, DataStruct, DeriveInput, Expr, ExprLit, Fields, Generics,
    Ident, Member, Meta, Token, Type, TypePath, Variant,
};

use crate::prelude::*;

struct ExprTypeAttr {
    #[expect(unused)]
    const_token: Token![const],
    expr: Expr,
    kind_ty: Option<(Token![:], Type)>,
}

impl Parse for ExprTypeAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let const_token = input.parse()?;
        let expr = input.parse()?;
        let kind_ty = if input.lookahead1().peek(Token![:]) {
            Some((input.parse()?, input.parse()?))
        } else {
            None
        };

        Ok(Self {
            const_token,
            expr,
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

enum OuterAttr {
    ExprType(ExprTypeAttr),
    Type(TypeAttr),
}

impl Parse for OuterAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(if input.lookahead1().peek(Token![const]) {
            Self::ExprType(input.parse()?)
        } else {
            Self::Type(input.parse()?)
        })
    }
}

struct FieldAttr {
    arms: Option<Vec<Arm>>,
}

impl FieldAttr {
    fn parse_attribute(attr: &Attribute) -> syn::Result<Self> {
        if matches!(attr.meta, Meta::Path(_)) {
            Ok(Self { arms: None })
        } else {
            attr.parse_args_with(|i: ParseStream| {
                let mut arms = vec![];
                while !i.is_empty() {
                    arms.push(i.parse()?);
                }

                Ok(Self { arms: Some(arms) })
            })
        }
    }
}

struct VariantAttr {
    expr: Expr,
}

impl Parse for VariantAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let expr = input.parse()?;

        Ok(Self { expr })
    }
}

pub fn run(input: DeriveInput) -> TokenStream {
    let span = input.span();
    let mut diag = TokenStream::new();
    let attr = get_attr(input.attrs, &mut diag);
    let attr = match attr.map(|a| a.parse_args::<OuterAttr>()).transpose() {
        Ok(a) => a,
        Err(e) => return e.into_compile_error(),
    };

    let item = match input.data {
        Data::Struct(data) => {
            run_struct(data, span, attr, &input.ident, &input.generics, &mut diag)
        },
        Data::Enum(data) => run_enum(data, span, attr, &input.ident, &input.generics, &mut diag),
        Data::Union(_) => span
            .error("Cannot derive Kind on unions")
            .into_compile_error(),
    };

    [diag, item].into_iter().collect()
}

fn run_struct(
    data: DataStruct,
    span: Span,
    attr: Option<OuterAttr>,
    ty: &Ident,
    generics: &Generics,
    diag: &mut TokenStream,
) -> TokenStream {
    let (kind_ty, expr) = 'found: {
        let fields = match data.fields {
            Fields::Named(n) => n.named,
            Fields::Unnamed(u) => u.unnamed,
            Fields::Unit => match run_unit(attr, span) {
                Ok(f) => break 'found f,
                Err(e) => return e.into_compile_error(),
            },
        };

        let kind_ty = match attr {
            Some(OuterAttr::ExprType(ExprTypeAttr {
                const_token: _,
                expr,
                kind_ty,
            })) => {
                for field in &fields {
                    if let Some(attr) = field.attrs.iter().find(|a| attr_matches(a)) {
                        diag.extend(
                            attr.span()
                                .error(
                                    "Cannot specify struct-level and field-level #[kind(...)] \
                                     expressions simultaneously",
                                )
                                .into_compile_error(),
                        );
                    }
                }

                let Some(kind_ty) = kind_ty.map_or_else(|| guess_type(&expr), |(_, t)| Some(t))
                else {
                    return expr
                        .span()
                        .error("Type annotations needed")
                        .into_compile_error();
                };

                break 'found (kind_ty, Some(expr.into_token_stream()));
            },
            Some(OuterAttr::Type(TypeAttr { kind_ty })) => Some(kind_ty),
            None => None,
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
            let Some(kind_ty) = kind_ty else {
                return span
                    .error("Missing #[kind(...)] attribute for struct")
                    .into_compile_error();
            };

            break 'found (kind_ty, None);
        };
        let span = field.span();

        let memb = field.ident.map_or(Member::Unnamed(i.into()), Member::Named);

        let FieldAttr { arms } = match get_attr(field.attrs, diag)
            .as_ref()
            .map(FieldAttr::parse_attribute)
        {
            Some(Ok(a)) => a,
            Some(Err(e)) => return e.into_compile_error(),
            None => unreachable!(),
        };

        let Some(arms) = arms else {
            let Some(kind_ty) = kind_ty else {
                return span
                    .error("Missing #[kind(...)] attribute for struct")
                    .into_compile_error();
            };

            break 'found (
                kind_ty,
                Some(quote_spanned! { span => crate::actions::Kind::kind(&self.#memb) }),
            );
        };

        let Some(kind_ty) = kind_ty.or_else(|| arms.iter().find_map(|a| guess_type(&a.body)))
        else {
            return span.error("Type annotations needed").into_compile_error();
        };

        (
            kind_ty,
            Some(quote_spanned! { span => match self.#memb { #(#arms)* } }),
        )
    };

    let expr = expr.unwrap_or_else(|| {
        quote_spanned! { kind_ty.span() => #kind_ty::#ty }
    });

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

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
}

fn run_unit(attr: Option<OuterAttr>, span: Span) -> syn::Result<(Type, Option<TokenStream>)> {
    match attr {
        Some(OuterAttr::ExprType(ExprTypeAttr {
            const_token: _,
            expr,
            kind_ty,
        })) => {
            let Some(kind_ty) = kind_ty.map_or_else(|| guess_type(&expr), |(_, t)| Some(t)) else {
                return Err(expr.span().error("Type annotations needed"));
            };

            Ok((kind_ty, Some(expr.into_token_stream())))
        },
        Some(OuterAttr::Type(TypeAttr { kind_ty })) => Ok((kind_ty, None)),
        None => Err(span.error("Missing #[kind(...)] attribute for unit struct")),
    }
}

fn run_enum(
    data: DataEnum,
    span: Span,
    attr: Option<OuterAttr>,
    ty: &Ident,
    generics: &Generics,
    diag: &mut TokenStream,
) -> TokenStream {
    let (kind_ty, expr) = 'found: {
        let mut kind_ty = match attr {
            Some(OuterAttr::ExprType(ExprTypeAttr {
                const_token: _,
                expr,
                kind_ty,
            })) => {
                for attr in data
                    .variants
                    .iter()
                    .flat_map(|v| v.attrs.iter().chain(v.fields.iter().flat_map(|f| &f.attrs)))
                {
                    diag.extend(
                        attr.span()
                            .error(
                                "Cannot specify enum-level and variant-level #[kind(...)] \
                                 expressions simultaneously",
                            )
                            .into_compile_error(),
                    );
                }

                let Some(kind_ty) = kind_ty.map_or_else(|| guess_type(&expr), |(_, t)| Some(t))
                else {
                    return expr
                        .span()
                        .error("Type annotations needed")
                        .into_compile_error();
                };

                break 'found (kind_ty, expr.into_token_stream());
            },
            Some(OuterAttr::Type(TypeAttr { kind_ty })) => Some(kind_ty),
            None => None,
        };

        if data.variants.is_empty()
            && let Some(kind_ty) = kind_ty
        {
            break 'found (kind_ty, quote_spanned! { span => match *self {} });
        }

        let arms: Vec<_> = data
            .variants
            .into_iter()
            .filter_map(|var| {
                let arm = run_variant(var, kind_ty.as_ref(), diag)
                    .map_err(|e| diag.extend(e.into_compile_error()))
                    .ok()?;

                if kind_ty.is_none() {
                    kind_ty = guess_type(&arm.body);
                }

                Some(arm)
            })
            .collect();

        let Some(kind_ty) = kind_ty else {
            return span.error("Type annotations needed").into_compile_error();
        };

        (kind_ty, quote_spanned! { span => match self { #(#arms)* } })
    };

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

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
}

fn run_variant(var: Variant, kind_ty: Option<&Type>, diag: &mut TokenStream) -> syn::Result<Arm> {
    let span = var.span();
    let ident = var.ident;
    let capture_none = match var.fields {
        Fields::Named(_) => quote_spanned! { span => { .. } },
        Fields::Unnamed(_) => quote_spanned! { span => (..) },
        Fields::Unit => TokenStream::new(),
    };

    let (capture, expr) = 'found: {
        if let Some(attr) = get_attr(var.attrs, diag) {
            let VariantAttr { expr } = attr.parse_args()?;

            (None, expr.into_token_stream())
        } else {
            let Some((i, field)) = get_one(
                var.fields
                    .into_iter()
                    .enumerate()
                    .filter(|(_, f)| f.attrs.iter().any(attr_matches)),
                |(_, f)| {
                    diag.extend(
                        f.span()
                            .error("Only one field per variant may have a #[kind] attribute")
                            .into_compile_error(),
                    );
                },
            ) else {
                let Some(kind_ty) = kind_ty else {
                    return Err(span.error("Missing #[kind(...)] attribute for variant"));
                };

                break 'found (None, quote_spanned! { kind_ty.span() => #kind_ty::#ident });
            };

            let span = field.span();

            let FieldAttr { arms } = FieldAttr::parse_attribute(
                &get_attr(field.attrs, diag).unwrap_or_else(|| unreachable!()),
            )?;

            (
                Some(if let Some(ident) = field.ident {
                    quote_spanned! { span => { #ident: __kind_arg, .. } }
                } else {
                    let blanks = std::iter::repeat_n(quote_spanned! { span => _ }, i);
                    quote_spanned! { span => (#(#blanks,)* __kind_arg, ..) }
                }),
                if let Some(arms) = arms {
                    quote_spanned! { span => match __kind_arg { #(#arms)* } }
                } else {
                    quote_spanned! { span => crate::actions::Kind::kind(__kind_arg) }
                },
            )
        }
    };

    let pat = capture.unwrap_or(capture_none);

    Ok(syn::parse_quote_spanned! { span => Self::#ident #pat => #expr, })
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

fn guess_type(expr: &Expr) -> Option<Type> {
    Some(match expr {
        Expr::Call(c) => guess_type(&c.func)?,
        Expr::Group(e) => guess_type(&e.expr)?,
        Expr::Lit(ExprLit { lit, .. }) => match lit {
            syn::Lit::Str(_) => syn::parse_quote! { &'static str },
            syn::Lit::CStr(_) => syn::parse_quote! { &'static ::core::ffi::CStr },
            syn::Lit::Byte(_) => syn::parse_quote! { u8 },
            syn::Lit::Char(_) => syn::parse_quote! { char },
            syn::Lit::Int(i) => syn::parse_str(i.suffix().trim_start_matches('_')).ok()?,
            syn::Lit::Float(f) => syn::parse_str(f.suffix().trim_start_matches('_')).ok()?,
            syn::Lit::Bool(_) => syn::parse_quote! { bool },
            _ => return None,
        },
        Expr::Match(m) => m.arms.iter().find_map(|a| guess_type(&a.body))?,
        Expr::Path(p) => {
            if let Some(ref qself) = p.qself
                && qself.position >= p.path.segments.len().saturating_sub(1)
            {
                (*qself.ty).clone()
            } else {
                let mut path = p.path.clone();
                path.segments.pop();
                path.segments.pop_punct();

                if path.segments.is_empty() {
                    return None;
                }

                TypePath {
                    qself: p.qself.clone(),
                    path,
                }
                .into()
            }
        },
        _ => return None,
    })
}
