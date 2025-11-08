use std::collections::HashMap;

use indexmap::IndexMap;
use spin::{mutex::SpinMutex, once::Once};
use syn::{
    fold::{self, Fold},
    punctuated::Punctuated,
    token::{Brace, Paren},
    AngleBracketedGenericArguments, Arm, Attribute, Expr, ExprCall, ExprMacro, ExprMatch, ExprPath,
    FieldPat, Fields, GenericArgument, Ident, ImplItemFn, Index, Item, ItemEnum, ItemImpl, LitBool,
    Macro, Member, Pat, PatIdent, PatStruct, PatTupleStruct, Path, PathArguments, PathSegment,
    QSelf, Signature, Stmt, StmtMacro, Token, Type, TypePath, Variant,
};

use crate::prelude::*;

#[derive(Default, Clone, Copy)]
pub struct Args {
    from: Option<(Span, bool)>,
}

impl Args {
    pub fn parser(&mut self) -> impl syn::parse::Parser<Output = ()> {
        syn::meta::parser(|meta| {
            if meta.path.is_ident("from") {
                self.from = Some((
                    meta.path.span(),
                    if meta.input.peek(Token![=]) {
                        meta.value()?.parse::<LitBool>()?.value
                    } else {
                        true
                    },
                ));

                Ok(())
            } else {
                Err(meta.error("Unknown property"))
            }
        })
    }
}

type Input = Item;

pub fn run(args: Args, input: Input) -> TokenStream {
    let mut diag = TokenStream::new();

    let ret = match input {
        Item::Enum(e) => run_enum(args, &e, &mut diag),
        Item::Impl(
            i @ ItemImpl {
                trait_: Some(_), ..
            },
        ) => run_impl(args, i, &mut diag),
        _ => input
            .span()
            .error("#[impl_enum] may only be used on enum or impl items")
            .into_compile_error(),
    };

    [diag, ret].into_iter().collect()
}

struct EnumInfo(String);

impl EnumInfo {
    #[inline]
    fn new(item: &ItemEnum) -> Self { Self(item.to_token_stream().to_string()) }

    #[inline]
    fn reparse(&self) -> ItemEnum {
        syn::parse_str(&self.0).unwrap_or_else(|_| unreachable!("Couldn't reparse enum"))
    }
}

struct ImplInfo(String);

impl ImplInfo {
    #[inline]
    fn new(item: &ItemImpl) -> Self { Self(item.to_token_stream().to_string()) }

    #[inline]
    fn reparse(&self) -> ItemImpl {
        syn::parse_str(&self.0).unwrap_or_else(|_| unreachable!("Couldn't reparse impl"))
    }
}

fn ty_ident<'ty>(ty: &'ty Type, diag: &mut TokenStream) -> Option<&'ty Ident> {
    Some(match ty {
        Type::Group(g) => ty_ident(&g.elem, diag)?,
        Type::Paren(p) => ty_ident(&p.elem, diag)?,
        Type::Path(TypePath { qself: None, path }) => &path.segments.last()?.ident,
        Type::Ptr(p) => ty_ident(&p.elem, diag)?,
        Type::Reference(r) => ty_ident(&r.elem, diag)?,
        Type::Slice(s) => ty_ident(&s.elem, diag)?,
        _ => {
            diag.extend(
                ty.span()
                    .error("Unable to determine type name")
                    .into_compile_error(),
            );
            return None;
        },
    })
}

// Can strengthen if needed, but honestly given the limitations this seems good enough
type EnumKey = String;
type EnumMap = HashMap<EnumKey, EnumInfo>;
type ImplMap = HashMap<EnumKey, Vec<ImplInfo>>;

fn shared() -> impl std::ops::DerefMut<Target = (EnumMap, ImplMap)> {
    static SHARED: Once<SpinMutex<(EnumMap, ImplMap)>> = Once::new();

    SHARED
        .call_once(|| (HashMap::new(), HashMap::new()).into())
        .lock()
}

fn run_enum(args: Args, item: &ItemEnum, diag: &mut TokenStream) -> TokenStream {
    use std::collections::hash_map::Entry;

    let Args { from } = args;
    let from = from.is_none_or(|(_, f)| f);

    let mut valid = true;
    for var in &item.variants {
        match var.fields {
            Fields::Named(ref n) if n.named.len() == 1 => (),
            Fields::Unnamed(ref u) if u.unnamed.len() == 1 => (),
            _ => {
                diag.extend(
                    var.span()
                        .error("Enum variants must contain exactly one field")
                        .into_compile_error(),
                );
                valid = false;
            },
        }
    }

    let mut items = TokenStream::new();
    if valid {
        let key = item.ident.to_string();
        let (ref mut enums, ref mut impls) = *shared();
        match enums.entry(key.clone()) {
            Entry::Occupied(_) => diag.extend(
                item.ident
                    .span()
                    .error(format!("Duplicate enum definition for {key}"))
                    .into_compile_error(),
            ),
            Entry::Vacant(v) => {
                let def = v.insert(EnumInfo::new(item));

                for imp in impls
                    .get_mut(&item.ident.to_string())
                    .into_iter()
                    .flat_map(|v| v.drain(..))
                {
                    link(def, Some(item.span()), imp.reparse(), diag);
                }
            },
        }

        if from {
            let ty = &item.ident;
            let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();
            vars(&item.variants)
                .iter()
                .map(|(v, (m, t))| {
                    let body = match m {
                        Member::Named(i) => quote_spanned! { i.span() => { #i: __value } },
                        Member::Unnamed(_) => quote_spanned! { v.span() => (__value) },
                    };

                    quote_spanned! { v.span() =>
                        impl #impl_generics
                            ::core::convert::From<#t>
                        for #ty #ty_generics
                            #where_clause
                        {
                            #[inline]
                            fn from(__value: #t) -> Self { Self::#v #body }
                        }
                    }
                })
                .for_each(|v| items.extend(v));
        }
    }

    quote_spanned! { item.span() => #item #items }
}

fn run_impl(args: Args, item: ItemImpl, diag: &mut TokenStream) -> TokenStream {
    let Args { from } = args;

    if let Some((span, _)) = from {
        diag.extend(
            span.error("Argument 'from' is not valid here")
                .into_compile_error(),
        );
    }

    let Some(ident) = ty_ident(&item.self_ty, diag) else {
        return TokenStream::new();
    };
    let key = ident.to_string();

    let (ref mut enums, ref mut impls) = *shared();

    if let Some(def) = enums.get(&key) {
        link(def, None, item, diag)
    } else {
        impls.entry(key).or_default().push(ImplInfo::new(&item));
        TokenStream::new()
    }
}

fn vars<'i>(variants: impl IntoIterator<Item = &'i Variant>) -> IndexMap<Ident, (Member, Type)> {
    variants
        .into_iter()
        .map(|v| {
            let field = match &v.fields {
                Fields::Named(n) => {
                    let field = n
                        .named
                        .iter()
                        .next()
                        .unwrap_or_else(|| unreachable!("Missing named field"))
                        .clone();
                    (
                        Member::Named(
                            field
                                .ident
                                .unwrap_or_else(|| unreachable!("Missing name on named field")),
                        ),
                        field.ty,
                    )
                },
                Fields::Unnamed(u) => (
                    Member::Unnamed(Index {
                        index: 0,
                        span: u.span(),
                    }),
                    u.unnamed
                        .iter()
                        .next()
                        .unwrap_or_else(|| unreachable!("Missing numbered field"))
                        .ty
                        .clone(),
                ),
                Fields::Unit => unreachable!("Missing fields"),
            };

            (v.ident.clone(), field)
        })
        .collect()
}

fn link(def: &EnumInfo, span: Option<Span>, imp: ItemImpl, diag: &mut TokenStream) -> TokenStream {
    let def = def.reparse();
    let vars = vars(&def.variants);

    DispatchFolder {
        current_fn: None,
        span,
        vars,
        trait_path: imp.trait_.as_ref().map(|(_, p, _)| p.clone()),
        diag,
    }
    .fold_item_impl(imp)
    .into_token_stream()
}

struct MacroArgs {
    args: Punctuated<Expr, Token![,]>,
}

impl Parse for MacroArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let args = Punctuated::parse_terminated(input)?;

        Ok(Self { args })
    }
}

struct DispatchFolder<'diag> {
    current_fn: Option<Signature>,

    span: Option<Span>,
    vars: IndexMap<Ident, (Member, Type)>,
    trait_path: Option<Path>,

    diag: &'diag mut TokenStream,
}

const DISPATCH_MACRO: &str = "dispatch";

impl Fold for DispatchFolder<'_> {
    fn fold_impl_item_fn(&mut self, i: ImplItemFn) -> ImplItemFn {
        let prev = self.current_fn.replace(i.sig.clone());
        let ret = fold::fold_impl_item_fn(self, i);
        self.current_fn = prev;
        ret
    }

    fn fold_stmt(&mut self, i: Stmt) -> Stmt {
        match i {
            Stmt::Macro(StmtMacro {
                attrs,
                mac,
                semi_token,
            }) if mac.path.is_ident(DISPATCH_MACRO) => Stmt::Expr(
                self.fold_expr(Expr::Macro(ExprMacro { attrs, mac })),
                semi_token,
            ),
            s => fold::fold_stmt(self, s),
        }
    }

    fn fold_expr(&mut self, i: Expr) -> Expr {
        match i {
            Expr::Macro(ExprMacro { attrs, mac }) if mac.path.is_ident(DISPATCH_MACRO) => {
                let MacroArgs { args } = match syn::parse2(mac.tokens.clone()) {
                    Ok(a) => a,
                    Err(e) => {
                        self.diag.extend(e.into_compile_error());
                        return ExprMacro { attrs, mac }.into();
                    },
                };

                let mut args = args.into_iter();
                let Some(this) = args.next() else {
                    self.diag.extend(
                        mac.span()
                            .error("Missing first argument for self")
                            .into_compile_error(),
                    );
                    return ExprMacro { attrs, mac }.into();
                };

                let Some((fn_name, fn_generics)) = self.current_fn_info() else {
                    self.diag.extend(
                        mac.span()
                            .error("Cannot infer function name")
                            .into_compile_error(),
                    );
                    return ExprMacro { attrs, mac }.into();
                };

                self.make_match(attrs, this, &args, fn_name, &fn_generics)
                    .into()
            },
            e => fold::fold_expr(self, e),
        }
    }

    fn fold_macro(&mut self, i: Macro) -> Macro {
        if i.path.is_ident("dispatch") {
            self.diag.extend(
                self.span
                    .unwrap_or_else(|| i.span())
                    .error("Invalid dispatch! macro found (this may be a bug)")
                    .into_compile_error(),
            );
        }

        i
    }
}

impl DispatchFolder<'_> {
    fn current_fn_info(&self) -> Option<(&Ident, Punctuated<GenericArgument, Token![,]>)> {
        self.current_fn.as_ref().map(|s| {
            (
                &s.ident,
                s.generics
                    .params
                    .iter()
                    .filter_map(|p| match p {
                        syn::GenericParam::Lifetime(_) => None,
                        syn::GenericParam::Type(t) => Some(GenericArgument::Type(
                            TypePath {
                                qself: None,
                                path: t.ident.clone().into(),
                            }
                            .into(),
                        )),
                        syn::GenericParam::Const(c) => Some(GenericArgument::Const(
                            ExprPath {
                                attrs: vec![],
                                qself: None,
                                path: c.ident.clone().into(),
                            }
                            .into(),
                        )),
                    })
                    .collect(),
            )
        })
    }

    fn make_match(
        &self,
        attrs: Vec<Attribute>,
        this: Expr,
        args: &(impl Iterator<Item = Expr> + Clone),
        fn_name: &Ident,
        fn_generics: &Punctuated<GenericArgument, Token![,]>,
    ) -> ExprMatch {
        let this_ident = Ident::new("__impl_enum_this", Span::call_site());
        let this_pat = Pat::Ident(PatIdent {
            attrs: vec![],
            by_ref: None,
            mutability: None,
            ident: this_ident.clone(),
            subpat: None,
        });

        let trait_colon = self.trait_path.as_ref().and_then(|p| p.leading_colon);
        let trait_segs = self.trait_path.as_ref().map(|p| p.segments.iter());

        ExprMatch {
            attrs,
            match_token: <Token![match]>::default(),
            expr: this.into(),
            brace_token: Brace::default(),
            arms: self
                .vars
                .iter()
                .map(|(v, (m, t))| {
                    let self_ident = Ident::new("Self", v.span());
                    let var_path = Path {
                        leading_colon: None,
                        segments: [&self_ident, v]
                            .into_iter()
                            .cloned()
                            .map(PathSegment::from)
                            .collect(),
                    };

                    Arm {
                        attrs: vec![],
                        pat: Self::memb_pat(m, var_path, this_pat.clone()),
                        guard: None,
                        fat_arrow_token: <Token![=>]>::default(),
                        body: Expr::Call(ExprCall {
                            attrs: vec![],
                            func: Expr::Path(ExprPath {
                                attrs: vec![],
                                qself: Some(QSelf {
                                    lt_token: <Token![<]>::default(),
                                    ty: t.clone().into(),
                                    position: trait_segs
                                        .as_ref()
                                        .map_or(0, std::iter::ExactSizeIterator::len),
                                    as_token: trait_segs.as_ref().map(|_| <Token![as]>::default()),
                                    gt_token: <Token![>]>::default(),
                                }),
                                path: Path {
                                    leading_colon: trait_colon,
                                    segments: trait_segs
                                        .iter()
                                        .cloned()
                                        .flatten()
                                        .cloned()
                                        .chain([PathSegment {
                                            ident: fn_name.clone(),
                                            arguments: PathArguments::AngleBracketed(
                                                AngleBracketedGenericArguments {
                                                    colon2_token: Some(<Token![::]>::default()),
                                                    lt_token: <Token![<]>::default(),
                                                    args: fn_generics.clone(),
                                                    gt_token: <Token![>]>::default(),
                                                },
                                            ),
                                        }])
                                        .collect(),
                                },
                            })
                            .into(),
                            paren_token: Paren::default(),
                            args: [ExprPath {
                                attrs: vec![],
                                qself: None,
                                path: this_ident.clone().into(),
                            }
                            .into()]
                            .into_iter()
                            .chain(args.clone())
                            .collect(),
                        })
                        .into(),
                        comma: Some(<Token![,]>::default()),
                    }
                })
                .collect(),
        }
    }

    fn memb_pat(memb: &Member, path: Path, this_pat: Pat) -> Pat {
        match memb {
            m @ Member::Named(_) => PatStruct {
                attrs: vec![],
                qself: None,
                path,
                brace_token: Brace::default(),
                fields: [FieldPat {
                    attrs: vec![],
                    member: m.clone(),
                    colon_token: Some(<Token![:]>::default()),
                    pat: this_pat.into(),
                }]
                .into_iter()
                .collect(),
                rest: None,
            }
            .into(),
            Member::Unnamed(_) => PatTupleStruct {
                attrs: vec![],
                qself: None,
                path,
                paren_token: Paren::default(),
                elems: [this_pat].into_iter().collect(),
            }
            .into(),
        }
    }
}
