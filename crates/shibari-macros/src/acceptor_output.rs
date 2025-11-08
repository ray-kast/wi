use syn::{Data, Field, Fields, GenericParam, Ident, Variant};

use crate::{derive_attr::AttributeArgs, prelude::*};

pub fn run(input: syn::DeriveInput) -> syn::Result<TokenStream> {
    let mut diag = TokenStream::new();
    let mut args = AttributeArgs::from_attrs(&input.attrs)?;
    let span = input.span();

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let Data::Enum(data) = input.data else {
        return Err(span.error("derive(AcceptorOutput) is only valid for enums"));
    };

    let mut trap = None;
    let mut advance = None;

    let mut from = vec![];

    for var in &data.variants {
        parse_variant(var, &mut trap, &mut advance, &mut from, &mut diag)?;
    }

    let trap = trap.ok_or_else(|| {
        span.error("Exactly one variant must have #[shibari(trap)] when deriving AcceptorOutput")
    })?;
    let advance = advance.ok_or_else(|| {
        span.error("Exactly one variant must have #[shibari(advance)] when deriving AcceptorOutput")
    })?;

    let shibari = args.crate_path().unwrap_or(syn::parse_quote! { ::shibari });
    let ty = input.ident;

    args.finish(&mut diag);

    let mut ret = quote_spanned! { span =>
        #[automatically_derived]
        impl #impl_generics #shibari::AcceptorOutput for #ty #ty_generics #where_clause {
            const TRAP: Self = Self::#trap;
            const ADVANCE: Self = Self::#advance;
        }
    };

    for (var, field) in from {
        let field_ty = field.ty;
        let value = Ident::new("__shibari_from_value", var.span());
        let fields = if let Some(ident) = field.ident {
            quote_spanned! { var.span() => { #ident: #value.into() } }
        } else {
            quote_spanned! { var.span() => (#value.into()) }
        };

        let param = Ident::new("__shibari_from_T", var.span());
        let mut generics = input.generics.clone();
        let index = generics
            .params
            .iter()
            .enumerate()
            .filter_map(|(i, p)| {
                matches!(p, GenericParam::Type(t) if t.default.is_none()).then_some(i)
            })
            .last()
            .unwrap_or(generics.params.len());
        generics.params.insert(
            index,
            GenericParam::Type(syn::parse_quote! {
                #param: ::core::convert::Into<#field_ty>
            }),
        );

        let (impl_generics, _, where_clause) = generics.split_for_impl();

        ret.extend(quote_spanned! { var.span() =>
            #[allow(non_camel_case_types, reason = "Generated code")]
            #[automatically_derived]
            impl #impl_generics ::core::convert::From<#param> for #ty #ty_generics #where_clause {
                #[inline]
                fn from(#value: #param) -> Self {
                    Self::#var #fields
                }
            }
        });
    }

    Ok([diag, ret].into_iter().collect())
}

fn parse_variant(
    var: &Variant,
    trap: &mut Option<Ident>,
    advance: &mut Option<Ident>,
    from: &mut Vec<(Ident, Field)>,
    diag: &mut TokenStream,
) -> syn::Result<()> {
    let mut args = AttributeArgs::from_attrs(&var.attrs)?;

    let mut found_unit = false;

    if args.trap().unwrap_or(false) {
        if trap.is_some() {
            diag.extend(
                var.span()
                    .error("Only one variant may have #[shibari(trap)]")
                    .into_compile_error(),
            );
        } else {
            *trap = Some(var.ident.clone());
        }

        found_unit = true;
    }

    if args.advance().unwrap_or(false) {
        if advance.is_some() {
            diag.extend(
                var.span()
                    .error("Only one variant may have #[shibari(advance)]")
                    .into_compile_error(),
            );
        } else {
            *advance = Some(var.ident.clone());
        }

        found_unit = true;
    }

    if args.from().unwrap_or(false) {
        let field = match &var.fields {
            Fields::Named(n) => {
                let mut it = n.named.iter();
                it.next()
                    .and_then(|v| it.next().is_none().then(|| v.clone()))
            },
            Fields::Unnamed(u) => {
                let mut it = u.unnamed.iter();
                it.next()
                    .and_then(|v| it.next().is_none().then(|| v.clone()))
            },
            Fields::Unit => None,
        };

        let Some(field) = field else {
            diag.extend(
                var.span()
                    .error("Variants with #[shibari(from)] must have exactly one field")
                    .into_compile_error(),
            );
            return Ok(());
        };

        from.push((var.ident.clone(), field));
    }

    if found_unit && !matches!(var.fields, Fields::Unit) {
        diag.extend(
            var.fields
                .span()
                .error("Variants with #[shibari(trap)] or #[shibar(advance)] cannot have fields")
                .into_compile_error(),
        );
    }

    args.finish(diag);
    Ok(())
}
