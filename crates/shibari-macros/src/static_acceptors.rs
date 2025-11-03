use hashbrown::HashMap;
use shibari_grammar::{
    table::{ConversionPath, Delta, DeltaKind, SparseTable, StateId, TreeOutput},
    tree::{RootId, TokenId},
};
use syn::Ident;

use crate::prelude::*;

mod parse;
mod reparse;

pub(crate) use parse::Input;

pub(super) fn run(input: Input) -> TokenStream {
    let mut diag = TokenStream::new();

    let Input {
        alias,
        pream_input_token: _,
        pream_eq_token: _,
        pream_ty: input_ty,
        pream_semi_token: _,
        tokens,
        grammars,
    } = input;

    let trees = reparse::parse_grammars(tokens, grammars, &mut diag);
    let tables = trees.flatten(reparse::StateExtra::from_path);

    let shibari = alias.map_or_else(
        || syn::parse_quote! { ::shibari },
        |parse::CrateAlias {
             use_token: _,
             path,
             as_token: _,
             shibari_token: _,
             semi_token: _,
         }| path,
    );

    let cx = BaseCx {
        acceptor_path: quote_spanned! { shibari.span() => #shibari::Acceptor },
        accept_state_path: quote_spanned! { shibari.span() => #shibari::AcceptState },
        input_ty,
    };

    let items = tables
        .into_iter()
        .map(|r| emit_acceptor(r, &trees, &cx))
        .collect();

    [diag, items].into_iter().collect()
}

fn make_accept_ty(ident: &Ident, span: Span) -> Ident {
    Ident::new(&format!("{ident}Accept"), span)
}
#[inline]
fn make_state_name(id: StateId, span: Span) -> Ident {
    Ident::new(&format!("S{}", id.as_u32()), span)
}

struct BaseCx {
    acceptor_path: TokenStream,
    accept_state_path: TokenStream,
    input_ty: syn::Type,
}

struct AcceptorCx<'a> {
    base: &'a BaseCx,
    accept_ty: Ident,
    state_ty: Ident,
}

fn emit_acceptor(
    (root, table): (
        RootId,
        SparseTable<
            TokenId,
            TreeOutput<reparse::Output, reparse::ExtendConversion>,
            reparse::StateExtra,
        >,
    ),
    trees: &reparse::TreeMap,
    cx: &BaseCx,
) -> TokenStream {
    let mut accept_attrs = TokenStream::new();
    let mut state_attrs = TokenStream::new();
    let mut states = TokenStream::new();
    let mut op_arms = TokenStream::new();
    let mut accept_arms = TokenStream::new();

    let root = trees.root(root).unwrap_or_else(|| unreachable!());
    let reparse::RootExtra { vis, ident, output } = root.extra();
    let span = ident.span();

    let cx = AcceptorCx {
        base: cx,
        accept_ty: make_accept_ty(ident, span),
        state_ty: Ident::new(&format!("{ident}State"), span),
    };
    let mut conv_state = ConvertState {
        items: TokenStream::new(),
        names: HashMap::new(),
    };

    for (id, state) in table.states() {
        let reparse::StateExtra { op } = state.extra();

        if id == StateId::START {
            accept_attrs.extend(quote_spanned! { span => #[derive(Default)] });
            state_attrs.extend(quote_spanned! { span => #[derive(Default)] });
            states.extend(quote_spanned! { span => #[default] });
        }

        let state_ty = &cx.state_ty;
        let id = make_state_name(id, span);
        states.extend(quote_spanned! { span => #id, });

        op_arms.extend(quote_spanned! { span =>
            #state_ty::#id => #op,
        });

        for (&input, output) in state.delta() {
            let reparse::TokenDef { pat, .. } =
                trees.token(input).unwrap_or_else(|| unreachable!());
            let delta = emit_delta(output, span, &cx, &mut conv_state);

            accept_arms.extend(quote_spanned! { span => (#state_ty::#id, #pat) => #delta });
        }

        let delta = state.default_delta();
        let delta = emit_delta(delta, span, &cx, &mut conv_state);

        accept_arms.extend(quote_spanned! { span => (#state_ty::#id, _) => #delta });
    }

    let mut accept_items = TokenStream::new();

    if states.is_empty() {
        accept_items.extend(quote_spanned! { span =>
            #![expect(unreachable_code)]
        });
    }

    accept_items.extend(conv_state.items);

    let AcceptorCx {
        base: BaseCx {
            acceptor_path,
            input_ty,
            ..
        },
        accept_ty,
        state_ty,
    } = cx;

    quote_spanned! { span =>
        #[derive(Debug, Clone, Copy, PartialEq)]
        #[repr(transparent)]
        #accept_attrs
        #vis struct #accept_ty(#state_ty);

        #[derive(Debug, Clone, Copy, PartialEq)]
        #state_attrs
        enum #state_ty { #states }

        impl #acceptor_path<#input_ty> for #accept_ty {
            type Output = #output;

            fn pending_op(&self) -> &'static str {
                match self.0 { #op_arms }
            }

            fn accept(&mut self, __input: #input_ty) -> Self::Output {
                #![allow(
                    clippy::never_loop,
                    clippy::useless_conversion,
                    reason = "Generated code"
                )]

                #accept_items

                let Self(__state) = self;
                loop {
                    let __out;
                    (*__state, __out) = match (*__state, __input) { #accept_arms };
                    break __out;
                }
            }
        }
    }
}

fn emit_delta(
    delta: &Delta<TreeOutput<reparse::Output, reparse::ExtendConversion>>,
    span: Span,
    cx: &AcceptorCx,
    conv_state: &mut ConvertState,
) -> TokenStream {
    let &Delta { next, ref kind } = delta;
    let span = if let DeltaKind::Yield(TreeOutput {
        output: reparse::Output::Expr(e),
        ..
    }) = kind
    {
        e.span()
    } else {
        span
    };
    let next = make_state_name(next, span);

    let state_ty = &cx.state_ty;

    match kind {
        DeltaKind::Yield(TreeOutput { output, conversion }) => {
            let out = emit_conversion(conversion, output, cx, conv_state);
            quote_spanned! { span => (#state_ty::#next, #out), }
        },
        DeltaKind::Continue => quote_spanned! { span =>
            { *__state = #state_ty::#next; continue; },
        },
    }
}

struct ConvertState {
    items: TokenStream,
    /// Maps (conversion, target) to emitted function name
    names: HashMap<(reparse::ExtendConversion, Ident), Ident>,
}

fn emit_conversion(
    conversion: &ConversionPath<reparse::ExtendConversion>,
    output: &reparse::Output,
    cx: &AcceptorCx,
    state: &mut ConvertState,
) -> TokenStream {
    let span = match output {
        reparse::Output::Expr(e) => e.span(),
        reparse::Output::Trap | reparse::Output::Advance => cx.base.accept_state_path.span(),
    };

    let AcceptorCx {
        base:
            BaseCx {
                acceptor_path,
                accept_state_path,
                input_ty,
                ..
            },
        accept_ty,
        ..
    } = cx;

    let mut it = conversion.iter().copied().peekable();

    let mut source_accept_opt = it.peek().map(|c| make_accept_ty(&c.source_grammar, span));
    let source_accept = source_accept_opt.as_ref().unwrap_or(accept_ty);

    let mut converted = match output {
        reparse::Output::Expr(e) => quote_spanned! { span =>
            ::core::convert::Into::<
                <#source_accept as #acceptor_path<#input_ty>>::Output
            >::into(#e)
        },
        reparse::Output::Trap => {
            return quote_spanned! { span =>
                <Self::Output as #accept_state_path>::TRAP
            }
        },
        reparse::Output::Advance => {
            return quote_spanned! { span =>
                <Self::Output as #accept_state_path>::ADVANCE
            }
        },
    };

    while let Some(conv) = it.next() {
        let source_accept = source_accept_opt.unwrap_or_else(|| unreachable!());

        let target_accept_opt = it.peek().map(|c| make_accept_ty(&c.source_grammar, span));
        let target_accept = target_accept_opt.as_ref().unwrap_or(accept_ty);

        let free = state.names.len();
        let ident = state
            .names
            .entry((conv.clone(), target_accept.clone()))
            .or_insert_with(|| {
                let name = Ident::new(&format!("__conv{free}"), span);

                let (param, body) = match &conv.kind {
                    reparse::ExtendConversionKind::From => (
                        &syn::parse_quote! { __value },
                        &syn::parse_quote! { { ::core::convert::Into::into(__value) } },
                    ),
                    reparse::ExtendConversionKind::Explicit { param, body } => (param, body),
                };

                state.items.extend(quote_spanned! { span =>
                    fn #name(
                        #param: <#source_accept as #acceptor_path<#input_ty>>::Output,
                    ) -> <#target_accept as #acceptor_path<#input_ty>>::Output #body
                });

                name
            });

        converted = quote_spanned! { span => #ident(#converted) };
        source_accept_opt = target_accept_opt;
    }

    converted
}
