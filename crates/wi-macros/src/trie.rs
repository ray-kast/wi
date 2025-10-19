use std::collections::{HashMap, HashSet};

use indexmap::IndexMap;
use syn::{token, Block, Expr, Ident, Label, Lifetime, LitStr, Pat, Path, Token, Type, Visibility};

use crate::prelude::*;

mod kw {
    #![expect(clippy::expl_impl_clone_on_copy)]

    syn::custom_keyword!(acceptor);
    syn::custom_keyword!(advance);
    syn::custom_keyword!(extend);
    syn::custom_keyword!(goto);
    syn::custom_keyword!(grammar);
    syn::custom_keyword!(input);
    syn::custom_keyword!(token);
}

pub struct Input {
    #[expect(unused)]
    pream_input_token: kw::input,
    #[expect(unused)]
    pream_eq_token: Token![=],
    pream_ty: Type,
    #[expect(unused)]
    pream_semi_token: Token![;],

    #[expect(unused)]
    acceptor_token: kw::acceptor,
    #[expect(unused)]
    acceptor_eq_token: Token![=],
    acceptor_path: Path,
    #[expect(unused)]
    acceptor_semi_token: Token![;],

    tokens: Vec<TokenDef>,
    grammars: Vec<GrammarDef>,
}

struct TokenDef {
    #[expect(unused)]
    token: kw::token,
    ident: Ident,
    #[expect(unused)]
    eq_token: Token![=],
    pat: Pat,
    op: Option<TokenOp>,
    #[expect(unused)]
    semi_token: Token![;],
}

struct TokenOp {
    #[expect(unused)]
    arrow_token: Token![=>],
    lit: LitStr,
}

struct GrammarDef {
    vis: Visibility,
    #[expect(unused)]
    grammar_token: kw::grammar,
    ident: Ident,
    #[expect(unused)]
    colon_token: Token![:],
    output: Type,

    #[expect(unused)]
    body_brace: token::Brace,

    advance: Option<GrammarAdvance>,
    nodes: Vec<GrammarNode>,
    extends: Vec<GrammarExtend>,
}

struct GrammarAdvance {
    #[expect(unused)]
    advance_token: kw::advance,
    #[expect(unused)]
    eq_token: Token![=],
    expr: Expr,
    #[expect(unused)]
    semi_token: Token![;],
}

struct GrammarNode {
    label: Option<Label>,
    tok_ident: Ident,
    kind: NodeKind,
}

#[expect(clippy::large_enum_variant)]
enum NodeKind {
    Leaf(NodeLeaf),
    Branch(NodeBranch),
}

struct NodeLeaf {
    #[expect(unused)]
    arrow_token: Token![=>],
    out: Output,
    #[expect(unused)]
    semi_token: Token![;],
}

enum Output {
    Expr(#[expect(unused)] Token![yield], Expr),
    Goto(kw::goto, Lifetime, Option<(Token![,], Expr)>),
    Continue(Token![continue]),
}

struct NodeBranch {
    #[expect(unused)]
    brace: token::Brace,
    out: Option<BranchYield>,
    nodes: Vec<GrammarNode>,
    default: Option<BranchDefault>,
}

struct BranchYield {
    #[expect(unused)]
    yield_token: Token![yield],
    expr: Expr,
    #[expect(unused)]
    semi_token: Token![;],
}

struct BranchDefault {
    #[expect(unused)]
    ddot_token: Token![..],
    out: Output,
}

struct GrammarExtend {
    #[expect(unused)]
    extend_token: kw::extend,
    ident: Ident,
    kind: ExtendKind,
}

#[allow(clippy::large_enum_variant)]
enum ExtendKind {
    From(#[expect(unused)] Token![;]),
    With(ExtendWith),
}

struct ExtendWith {
    #[expect(unused)]
    paren: token::Paren,
    pat: Pat,
    block: Block,
}

impl Parse for Input {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let pream_input_token = input.parse()?;
        let pream_eq_token = input.parse()?;
        let pream_ty = input.parse()?;
        let pream_semi_token = input.parse()?;

        let acceptor_token = input.parse()?;
        let acceptor_eq_token = input.parse()?;
        let acceptor_path = input.parse()?;
        let acceptor_semi_token = input.parse()?;

        let mut tokens = vec![];
        let mut grammars = vec![];

        while !input.is_empty() {
            if input.lookahead1().peek(kw::token) {
                tokens.push(input.parse()?);
            } else {
                grammars.push(input.parse()?);
            }
        }

        Ok(Self {
            pream_input_token,
            pream_eq_token,
            pream_ty,
            pream_semi_token,
            acceptor_token,
            acceptor_eq_token,
            acceptor_path,
            acceptor_semi_token,
            tokens,
            grammars,
        })
    }
}

impl Parse for TokenDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let token = input.parse()?;
        let ident = input.parse()?;
        let eq_token = input.parse()?;
        let pat = Pat::parse_multi_with_leading_vert(input)?;

        let op = if input.lookahead1().peek(Token![=>]) {
            Some(input.parse()?)
        } else {
            None
        };

        let semi_token = input.parse()?;

        Ok(Self {
            token,
            ident,
            eq_token,
            pat,
            op,
            semi_token,
        })
    }
}

impl Parse for TokenOp {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let arrow_token = input.parse()?;
        let lit = input.parse()?;

        Ok(Self { arrow_token, lit })
    }
}

impl Parse for GrammarDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let vis = input.parse()?;
        let grammar_token = input.parse()?;
        let ident = input.parse()?;
        let colon_token = input.parse()?;
        let output = input.parse()?;

        let body;
        let body_brace = syn::braced!(body in input);

        let advance = if body.peek(kw::advance) {
            Some(body.parse()?)
        } else {
            None
        };

        let mut nodes = vec![];
        let mut extends = vec![];

        while !(body.is_empty() || body.peek(kw::extend)) {
            nodes.push(body.parse()?);
        }

        while !body.is_empty() {
            extends.push(body.parse()?);
        }

        Ok(Self {
            vis,
            grammar_token,
            ident,
            colon_token,
            output,
            body_brace,
            advance,
            nodes,
            extends,
        })
    }
}

impl Parse for GrammarAdvance {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let advance_token = input.parse()?;
        let eq_token = input.parse()?;
        let expr = input.parse()?;
        let semi_token = input.parse()?;

        Ok(Self {
            advance_token,
            eq_token,
            expr,
            semi_token,
        })
    }
}

impl Parse for GrammarNode {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let label = if input.lookahead1().peek(Lifetime) {
            Some(input.parse()?)
        } else {
            None
        };

        let tok_ident = input.parse()?;

        let kind = if input.lookahead1().peek(Token![=>]) {
            NodeKind::Leaf(input.parse()?)
        } else {
            NodeKind::Branch(input.parse()?)
        };

        Ok(Self {
            label,
            tok_ident,
            kind,
        })
    }
}

impl Parse for NodeLeaf {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let arrow_token = input.parse()?;
        let out = input.parse()?;
        let semi_token = input.parse()?;

        Ok(Self {
            arrow_token,
            out,
            semi_token,
        })
    }
}

impl Parse for Output {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(Token![yield]) {
            Ok(Self::Expr(input.parse()?, input.parse()?))
        } else if lookahead.peek(kw::goto) {
            let goto_token = input.parse()?;
            let label = input.parse()?;
            let out = if input.lookahead1().peek(Token![,]) {
                Some((input.parse()?, input.parse()?))
            } else {
                None
            };

            Ok(Self::Goto(goto_token, label, out))
        } else {
            Ok(Self::Continue(input.parse()?))
        }
    }
}

impl Parse for NodeBranch {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        let brace = syn::braced!(content in input);

        let out = if content.peek(Token![yield]) {
            Some(content.parse()?)
        } else {
            None
        };

        let mut nodes = vec![];

        while !(content.is_empty() || content.peek(Token![..])) {
            nodes.push(content.parse()?);
        }

        let default = if content.peek(Token![..]) {
            Some(content.parse()?)
        } else {
            None
        };

        if !content.is_empty() {
            return Err(content.span().error("Unexpected tokens"));
        }

        Ok(Self {
            brace,
            out,
            nodes,
            default,
        })
    }
}

impl Parse for BranchYield {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let yield_token = input.parse()?;
        let expr = input.parse()?;
        let semi_token = input.parse()?;

        Ok(Self {
            yield_token,
            expr,
            semi_token,
        })
    }
}

impl Parse for BranchDefault {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ddot_token = input.parse()?;
        let out = input.parse()?;

        Ok(Self { ddot_token, out })
    }
}

impl Parse for GrammarExtend {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let extend_token = input.parse()?;
        let ident = input.parse()?;

        let kind = if input.lookahead1().peek(Token![;]) {
            ExtendKind::From(input.parse()?)
        } else {
            ExtendKind::With(input.parse()?)
        };

        Ok(Self {
            extend_token,
            ident,
            kind,
        })
    }
}

impl Parse for ExtendWith {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let args;
        let paren = syn::parenthesized!(args in input);
        let pat = Pat::parse_single(&args)?;

        if !args.is_empty() {
            return Err(args.span().error("Unexpected tokens"));
        }

        let block = input.parse()?;

        Ok(Self { paren, pat, block })
    }
}

struct ParsedGrammar {
    vis: Visibility,
    output: Type,
    advance: Option<Expr>,
    root: ParsedBranch,
    extends: Vec<(Ident, Option<ParsedExtendConv>)>,
}

struct ParsedBranch {
    label: Option<Lifetime>,
    out: Option<Expr>,
    nodes: IndexMap<Ident, ParsedNode>,
    default: Option<Output>,
}

type ParsedExtendConv = (Pat, Block);

#[expect(clippy::large_enum_variant)]
enum ParsedNode {
    Leaf(Output),
    Branch(ParsedBranch),
}

fn parse_tokens(
    tokens: Vec<TokenDef>,
    diag: &mut TokenStream,
) -> HashMap<Ident, (Pat, Option<LitStr>)> {
    let mut pats = HashMap::new();

    for TokenDef {
        token: _,
        ident,
        eq_token: _,
        pat,
        op,
        semi_token: _,
    } in tokens
    {
        use std::collections::hash_map::Entry;

        let ident_span = ident.span();
        match pats.entry(ident) {
            Entry::Vacant(v) => {
                v.insert((
                    pat,
                    op.map(
                        |TokenOp {
                             arrow_token: _,
                             lit,
                         }| lit,
                    ),
                ));
            },
            Entry::Occupied(_) => diag.extend(
                ident_span
                    .error("Duplicate token name")
                    .into_compile_error(),
            ),
        }
    }

    pats
}

fn parse_grammars(
    grammars: Vec<GrammarDef>,
    diag: &mut TokenStream,
) -> IndexMap<Ident, ParsedGrammar> {
    let mut parsed = IndexMap::new();
    for GrammarDef {
        vis,
        grammar_token: _,
        ident,
        colon_token: _,
        output,
        body_brace: _,
        advance,
        nodes,
        extends,
    } in grammars
    {
        use indexmap::map::Entry;

        let ident_span = ident.span();
        match parsed.entry(ident) {
            Entry::Vacant(v) => {
                v.insert(ParsedGrammar {
                    vis,
                    output,
                    advance: advance.map(
                        |GrammarAdvance {
                             advance_token: _,
                             eq_token: _,
                             expr,
                             semi_token: _,
                         }| expr,
                    ),
                    root: parse_branch(None, None, nodes, None, diag),
                    extends: extends
                        .into_iter()
                        .map(
                            |GrammarExtend {
                                 extend_token: _,
                                 ident,
                                 kind,
                             }| {
                                (ident, match kind {
                                    ExtendKind::From(_) => None,
                                    ExtendKind::With(ExtendWith {
                                        paren: _,
                                        pat,
                                        block,
                                    }) => Some((pat, block)),
                                })
                            },
                        )
                        .collect(),
                });
            },
            Entry::Occupied(_) => diag.extend(
                ident_span
                    .error("Duplicate grammar name")
                    .into_compile_error(),
            ),
        }
    }

    parsed
}

fn parse_branch(
    label: Option<Lifetime>,
    out: Option<BranchYield>,
    nodes: Vec<GrammarNode>,
    default: Option<BranchDefault>,
    diag: &mut TokenStream,
) -> ParsedBranch {
    let mut node_map = IndexMap::new();

    for GrammarNode {
        label,
        tok_ident,
        kind,
    } in nodes
    {
        use indexmap::map::Entry;

        let ident_span = tok_ident.span();
        match node_map.entry(tok_ident) {
            Entry::Vacant(v) => {
                v.insert(match kind {
                    NodeKind::Leaf(NodeLeaf {
                        arrow_token: _,
                        out,
                        semi_token: _,
                    }) => ParsedNode::Leaf(out),
                    NodeKind::Branch(NodeBranch {
                        brace: _,
                        out,
                        nodes,
                        default,
                    }) => ParsedNode::Branch(parse_branch(
                        label.map(|l| l.name),
                        out,
                        nodes,
                        default,
                        diag,
                    )),
                });
            },
            Entry::Occupied(_) => diag.extend(
                ident_span
                    .error("Duplicate token in grammar block")
                    .into_compile_error(),
            ),
        }
    }

    ParsedBranch {
        label,
        out: out.map(
            |BranchYield {
                 yield_token: _,
                 expr,
                 semi_token: _,
             }| expr,
        ),
        nodes: node_map,
        default: default.map(|BranchDefault { ddot_token: _, out }| out),
    }
}

#[inline]
fn default_output(span: Span) -> TokenStream {
    quote_spanned! { span => <Self::Output as ::core::default::Default>::default() }
}

pub(super) fn run(input: Input) -> TokenStream {
    let mut diag = TokenStream::new();

    let Input {
        pream_input_token: _,
        pream_eq_token: _,
        pream_ty,
        pream_semi_token: _,
        acceptor_token: _,
        acceptor_eq_token: _,
        acceptor_path,
        acceptor_semi_token: _,
        tokens,
        grammars,
    } = input;

    let pats = parse_tokens(tokens, &mut diag);
    let grammars = parse_grammars(grammars, &mut diag);

    let referenced: HashSet<_> = grammars
        .values()
        .flat_map(|g| g.extends.iter())
        .map(|(i, _)| i)
        .collect();

    let mut cx = Context {
        input_ty: pream_ty,
        acceptor_path,
        pats,
        diag: &mut diag,
    };

    let items = grammars
        .iter()
        .map(|(i, g)| cx.grammar(i, g, referenced.contains(i), &grammars))
        .collect();

    [diag, items].into_iter().collect()
}

struct Context<'a> {
    input_ty: Type,
    acceptor_path: Path,
    pats: HashMap<Ident, (Pat, Option<LitStr>)>,
    diag: &'a mut TokenStream,
}

enum StateIdent {
    Variant(Ident),
    Const(Ident),
}

struct NodeCx<'a> {
    accept_ty: Ident,
    state_ty: Ident,
    advance: Option<&'a Expr>,
    extend_convs: ExtendConvs<'a>,
    start_state: Ident,

    free_state: u32,
    conv_fn_names: HashMap<&'a Ident, Ident>,
    label_states: HashMap<(Option<&'a Ident>, &'a Lifetime), StateIdent>,

    states: TokenStream,
    op_arms: TokenStream,
    accept_items: TokenStream,
    accept_arms: TokenStream,
}

fn accept_ty(ident: &Ident) -> Ident { Ident::new(&format!("{ident}Accept"), ident.span()) }
fn state_name(id: u32, span: Span) -> Ident { Ident::new(&format!("S{id}"), span) }
fn state_const_name(id: usize, span: Span) -> Ident { Ident::new(&format!("__STATE{id}"), span) }
fn conv_fn_name(id: usize, span: Span) -> Ident { Ident::new(&format!("__conv{id}"), span) }

type ExtendRefs<'a> = Vec<(&'a ParsedBranch, Option<&'a Ident>)>;
type ExtendConvs<'a> = HashMap<&'a Ident, Option<&'a ParsedExtendConv>>;

impl Context<'_> {
    fn collect_extends<'a>(
        &mut self,
        root: &'a ParsedBranch,
        extends: &'a [(Ident, Option<ParsedExtendConv>)],
        grammars: &'a IndexMap<Ident, ParsedGrammar>,
    ) -> (ExtendRefs<'a>, ExtendConvs<'a>) {
        let mut branches = vec![(root, None)];
        let mut extend_convs = HashMap::new();

        for (ident, with) in extends {
            use std::collections::hash_map::Entry;

            let grammar = grammars.get(ident);

            if let Some(grammar) = grammar {
                branches.push((&grammar.root, Some(ident)));
            } else {
                self.diag.extend(
                    ident
                        .span()
                        .error("Unrecognized grammar name")
                        .to_compile_error(),
                );
            }

            let ident_span = ident.span();
            match extend_convs.entry(ident) {
                Entry::Vacant(v) => {
                    v.insert(with.as_ref());
                },
                Entry::Occupied(_) => self.diag.extend(
                    ident_span
                        .error("Duplicate extend declaration")
                        .to_compile_error(),
                ),
            }
        }

        (branches, extend_convs)
    }

    fn grammar(
        &mut self,
        ident: &Ident,
        grammar: &ParsedGrammar,
        referenced: bool,
        grammars: &IndexMap<Ident, ParsedGrammar>,
    ) -> TokenStream {
        let ParsedGrammar {
            vis,
            output,
            advance,
            root,
            extends,
        } = grammar;

        let start_state = state_name(0, ident.span());

        let (branches, extend_convs) = self.collect_extends(root, extends, grammars);

        let mut node_cx = NodeCx {
            accept_ty: accept_ty(ident),
            state_ty: Ident::new(&format!("{ident}State"), ident.span()),
            advance: advance.as_ref(),
            extend_convs,
            start_state: start_state.clone(),

            free_state: 1,
            conv_fn_names: HashMap::new(),
            label_states: HashMap::new(),

            states: TokenStream::new(),
            op_arms: TokenStream::new(),
            accept_items: TokenStream::new(),
            accept_arms: TokenStream::new(),
        };

        self.visit_branch(
            &start_state,
            &LitStr::new("", ident.span()),
            &branches,
            &mut node_cx,
        );

        let Context {
            input_ty,
            acceptor_path,
            ..
        } = self;
        let NodeCx {
            accept_ty,
            state_ty,
            start_state,
            states,
            op_arms,
            accept_items: conv_fns,
            accept_arms,
            ..
        } = node_cx;

        let allow_unused = referenced.then(|| {
            quote_spanned! { ident.span() =>
                #[allow(dead_code, reason = "Generated code, referenced by another grammar")]
            }
        });
        let default = default_output(ident.span());

        quote_spanned! { ident.span() =>
            #[derive(Debug, Default, Clone, Copy, PartialEq)]
            #[repr(transparent)]
            #allow_unused
            #vis struct #accept_ty(#state_ty);

            #[derive(Debug, Default, Clone, Copy, PartialEq)]
            #allow_unused
            enum #state_ty {
                #[default] #start_state,
                #states
            }

            impl #acceptor_path<#input_ty> for #accept_ty {
                type Output = #output;

                fn pending_op(&self) -> &'static str {
                    match self.0 { #op_arms }
                }

                fn accept(&mut self, __input: #input_ty) -> Self::Output {
                    #![allow(clippy::never_loop, clippy::useless_conversion, reason = "Generated code")]

                    #conv_fns

                    let Self(__state) = self;
                    loop {
                        let __out;
                        (*__state, __out) = match (*__state, __input) {
                            #accept_arms
                            _ => (#state_ty::#start_state, #default),
                        };
                        break __out;
                    }
                }
            }
        }
    }

    fn visit_branch<'a>(
        &mut self,
        state: &Ident,
        op: &LitStr,
        branches: &[(&'a ParsedBranch, Option<&'a Ident>)],
        node_cx: &mut NodeCx<'a>,
    ) {
        let state_ty = &node_cx.state_ty;
        node_cx
            .op_arms
            .extend(quote_spanned! { state.span() => #state_ty::#state => #op, });

        let mut collected = IndexMap::new();
        let mut first_default = None;

        for &(branch, extend_ident) in branches {
            let ParsedBranch {
                label,
                out: _,
                nodes,
                default,
            } = branch;

            if let Some(label) = label {
                use std::collections::hash_map::Entry;

                match node_cx.label_states.entry((extend_ident, label)) {
                    Entry::Occupied(o) => match o.get() {
                        StateIdent::Variant(_) => self.diag.extend(
                            label
                                .span()
                                .error("Duplicate node label")
                                .to_compile_error(),
                        ),
                        StateIdent::Const(c) => {
                            let state_ty = &node_cx.state_ty;

                            node_cx.accept_items.extend(quote_spanned! { c.span() =>
                                const #c: #state_ty = #state_ty::#state;
                            });

                            *o.into_mut() = StateIdent::Variant(state.clone());
                        },
                    },
                    Entry::Vacant(v) => {
                        v.insert(StateIdent::Variant(state.clone()));
                    },
                }
            }

            for (tok_ident, node) in nodes {
                collected
                    .entry(tok_ident)
                    .or_insert(vec![])
                    .push((node, extend_ident));
            }

            first_default = first_default.or(default.as_ref().map(|d| (d, extend_ident)));
        }

        for (tok_ident, nodes) in collected {
            self.visit_node(state, op, tok_ident, &nodes, node_cx);
        }

        if let Some((default, extend_ident)) = first_default {
            let next = self.output(default, extend_ident, node_cx);
            let state_ty = &node_cx.state_ty;

            node_cx.accept_arms.extend(quote_spanned! { next.span() =>
                (#state_ty::#state, _) => #next,
            });
        }
    }

    fn visit_node<'a>(
        &mut self,
        state: &Ident,
        op: &LitStr,
        tok_ident: &Ident,
        nodes: &[(&'a ParsedNode, Option<&'a Ident>)],
        node_cx: &mut NodeCx<'a>,
    ) {
        let Some((pat, tok_op)) = self.pats.get(tok_ident) else {
            self.diag.extend(
                tok_ident
                    .span()
                    .error("Unknown token name")
                    .to_compile_error(),
            );
            return;
        };

        let mut branches = vec![];
        let mut advance = None;

        for &(node, extend_ident) in nodes {
            match node {
                ParsedNode::Leaf(out) => {
                    let next = self.output(out, extend_ident, node_cx);
                    let state_ty = &node_cx.state_ty;

                    node_cx
                        .accept_arms
                        .extend(quote_spanned! { tok_ident.span() =>
                            (#state_ty::#state, #pat) => #next,
                        });

                    return;
                },
                ParsedNode::Branch(b @ ParsedBranch { out: Some(o), .. }) => {
                    branches = vec![(b, extend_ident)];
                    advance = Some(self.convert_output_expr(
                        o,
                        extend_ident,
                        &node_cx.accept_ty,
                        &node_cx.extend_convs,
                        &mut node_cx.conv_fn_names,
                        &mut node_cx.accept_items,
                    ));
                    break;
                },
                ParsedNode::Branch(b @ ParsedBranch { out: None, .. }) => {
                    branches.push((b, extend_ident));
                },
            }
        }

        let free = node_cx.free_state;
        let next = state_name(free, tok_ident.span());

        node_cx.free_state += 1;
        node_cx
            .states
            .extend(quote_spanned! { next.span() => #next, });

        let advance = advance.unwrap_or_else(|| {
            node_cx
                .advance
                .map_or(default_output(tok_ident.span()), |a| {
                    quote_spanned! { a.span() =>
                        <Self::Output as ::core::convert::From<_>>::from(#a)
                    }
                })
        });
        let state_ty = &node_cx.state_ty;

        node_cx
            .accept_arms
            .extend(quote_spanned! { tok_ident.span() =>
                (#state_ty::#state, #pat) => (#state_ty::#next, #advance),
            });

        let Some(tok_op) = tok_op else {
            self.diag.extend(
                pat.span()
                    .error("Missing operator string for branch token")
                    .to_compile_error(),
            );
            return;
        };

        let mut op = op.value();
        op.push_str(&tok_op.value());
        self.visit_branch(&next, &LitStr::new(&op, tok_op.span()), &branches, node_cx);
    }

    fn output<'a>(
        &self,
        out: &'a Output,
        extend_ident: Option<&'a Ident>,
        node_cx: &mut NodeCx<'a>,
    ) -> TokenStream {
        let state_ty = &node_cx.state_ty;
        let start_state = &node_cx.start_state;

        match out {
            Output::Expr(_, e) => {
                let out = self.convert_output_expr(
                    e,
                    extend_ident,
                    &node_cx.accept_ty,
                    &node_cx.extend_convs,
                    &mut node_cx.conv_fn_names,
                    &mut node_cx.accept_items,
                );

                quote_spanned! { e.span() => (#state_ty::#start_state, #out) }
            },
            Output::Goto(g, l, o) => {
                let free = node_cx.label_states.len();
                let next = node_cx
                    .label_states
                    .entry((extend_ident, l))
                    .or_insert_with(|| StateIdent::Const(state_const_name(free, l.span())));

                let out = o.as_ref().map_or_else(
                    || default_output(g.span()),
                    |o| {
                        self.convert_output_expr(
                            &o.1,
                            extend_ident,
                            &node_cx.accept_ty,
                            &node_cx.extend_convs,
                            &mut node_cx.conv_fn_names,
                            &mut node_cx.accept_items,
                        )
                    },
                );

                match next {
                    StateIdent::Variant(i) => {
                        quote_spanned! { g.span() => (#state_ty::#i, #out) }
                    },
                    StateIdent::Const(i) => {
                        quote_spanned! { g.span() => (#i, #out) }
                    },
                }
            },
            Output::Continue(c) => quote_spanned! { c.span() =>
                { *__state = #state_ty::#start_state; continue; }
            },
        }
    }

    fn convert_output_expr<'a>(
        &self,
        out: &Expr,
        extend_ident: Option<&'a Ident>,
        self_ty: &Ident,
        extend_convs: &ExtendConvs,
        fn_names: &mut HashMap<&'a Ident, Ident>,
        items: &mut TokenStream,
    ) -> TokenStream {
        if let Some(i) = extend_ident {
            use std::collections::hash_map::Entry;

            let free = fn_names.len();
            let conv_fn = match fn_names.entry(i) {
                Entry::Occupied(o) => &*o.into_mut(),
                Entry::Vacant(v) => {
                    let (pat, block) = extend_convs
                        .get(i)
                        .unwrap_or_else(|| unreachable!())
                        .map_or_else(
                            || {
                                (
                                    quote_spanned! { i.span() => __value },
                                    quote_spanned! { i.span() =>
                                        { ::core::convert::From::from(__value) }
                                    },
                                )
                            },
                            |(p, e)| (p.to_token_stream(), e.to_token_stream()),
                        );

                    let fn_name = conv_fn_name(free, i.span());
                    let extend_ty = accept_ty(i);
                    let input_ty = &self.input_ty;
                    let acceptor_path = &self.acceptor_path;

                    items.extend(quote_spanned! { i.span() =>
                        fn #fn_name(
                            #pat: <#extend_ty as #acceptor_path<#input_ty>>::Output,
                        ) -> <#self_ty as #acceptor_path<#input_ty>>::Output #block
                    });

                    v.insert(fn_name)
                },
            };

            quote_spanned! { i.span() => #conv_fn(::core::convert::Into::into(#out)) }
        } else {
            quote_spanned! { out.span() =>
                <Self::Output as ::core::convert::From<_>>::from(#out)
            }
        }
    }
}
