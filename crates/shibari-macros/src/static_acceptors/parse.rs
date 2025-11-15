use syn::{
    punctuated::Punctuated, token, Block, Expr, Ident, Label, Lifetime, LitStr, Pat, Path, Token,
    Type, Visibility,
};

use crate::prelude::*;

mod kw {
    #![expect(clippy::expl_impl_clone_on_copy)]

    syn::custom_keyword!(extend);
    syn::custom_keyword!(goto);
    syn::custom_keyword!(grammar);
    syn::custom_keyword!(input);
    syn::custom_keyword!(shibari);
    syn::custom_keyword!(token);
}

pub(crate) struct Input {
    pub(super) alias: Option<CrateAlias>,

    pub(super) pream_input_token: kw::input,

    pub(super) pream_eq_token: Token![=],
    pub(super) pream_ty: Type,

    pub(super) pream_semi_token: Token![;],

    pub(super) token_defs: Vec<TokenDef>,
    pub(super) grammars: Vec<GrammarDef>,
}

pub(super) struct CrateAlias {
    pub use_token: Token![use],
    pub path: Path,

    pub as_token: Token![as],

    pub shibari_token: kw::shibari,

    pub semi_token: Token![;],
}

pub(super) struct TokenDef {
    pub token: kw::token,
    pub ident: Ident,

    pub eq_token: Token![=],
    pub body: TokenBody,

    pub semi_token: Token![;],
}

pub(super) struct TokenBody {
    pub pat: Pat,
    pub op: TokenOp,
}

pub(super) struct TokenOp {
    pub arrow_token: Token![=>],
    pub lit: LitStr,
}

pub(super) struct GrammarDef {
    pub vis: Visibility,

    pub grammar_token: kw::grammar,
    pub ident: Ident,

    pub colon_token: Token![:],
    pub output: Type,
    pub op: Option<TokenOp>,

    pub root: NodeBranch,
}

pub(super) struct GrammarNode {
    pub label: Option<Label>,

    pub leading_vert: Option<Token![|]>,
    pub tok_refs: Punctuated<TokenRef, Token![|]>,
    pub kind: NodeKind,
}

pub(super) enum TokenRef {
    Named(Ident),
    Anon(kw::token, token::Brace, TokenBody),
}

#[expect(clippy::large_enum_variant)]
pub(super) enum NodeKind {
    Leaf(NodeLeaf),
    Branch(NodeBranch),
}

pub(super) struct NodeLeaf {
    pub arrow_token: Token![=>],
    pub out: Output,

    pub semi_token: Token![;],
}

#[derive(Clone)]
pub(super) enum Output {
    Expr(Token![yield], Expr),
    Goto(kw::goto, Lifetime, Option<(Token![,], Expr)>),
    Continue(Token![continue]),
}

pub(super) struct NodeBranch {
    pub brace: token::Brace,
    pub out: Option<BranchYield>,
    pub nodes: Vec<GrammarNode>,
    pub extends: Vec<GrammarExtend>,
    pub default: Option<BranchDefault>,
}

pub struct BranchYield {
    pub yield_token: Token![yield],
    pub expr: Expr,

    pub semi_token: Token![;],
}

pub struct BranchDefault {
    pub ddot_token: Token![..],
    pub out: Output,
}

pub struct GrammarExtend {
    pub extend_token: kw::extend,
    pub ident: Ident,
    pub kind: ExtendKind,
}

#[allow(clippy::large_enum_variant)]
pub(super) enum ExtendKind {
    From(Token![;]),
    With(ExtendWith),
}

pub(super) struct ExtendWith {
    pub paren: token::Paren,
    pub pat: Pat,
    pub block: Block,
}

impl Parse for Input {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let alias = if input.lookahead1().peek(Token![use]) {
            Some(input.parse()?)
        } else {
            None
        };

        let pream_input_token = input.parse()?;
        let pream_eq_token = input.parse()?;
        let pream_ty = input.parse()?;
        let pream_semi_token = input.parse()?;

        let mut token_defs = vec![];
        let mut grammars = vec![];

        while !input.is_empty() {
            if input.lookahead1().peek(kw::token) {
                token_defs.push(input.parse()?);
            } else {
                grammars.push(input.parse()?);
            }
        }

        Ok(Self {
            alias,
            pream_input_token,
            pream_eq_token,
            pream_ty,
            pream_semi_token,
            token_defs,
            grammars,
        })
    }
}

impl Parse for CrateAlias {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let use_token = input.parse()?;
        let path = input.parse()?;
        let as_token = input.parse()?;
        let shibari_token = input.parse()?;
        let semi_token = input.parse()?;

        Ok(Self {
            use_token,
            path,
            as_token,
            shibari_token,
            semi_token,
        })
    }
}

impl Parse for TokenDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let token = input.parse()?;
        let ident = input.parse()?;
        let eq_token = input.parse()?;
        let body = input.parse()?;
        let semi_token = input.parse()?;

        Ok(Self {
            token,
            ident,
            eq_token,
            body,
            semi_token,
        })
    }
}

impl Parse for TokenBody {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let pat = Pat::parse_multi_with_leading_vert(input)?;
        let op = input.parse()?;

        Ok(Self { pat, op })
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

        let op = if input.lookahead1().peek(Token![=>]) {
            Some(input.parse()?)
        } else {
            None
        };

        let root = input.parse()?;

        Ok(Self {
            vis,
            grammar_token,
            ident,
            colon_token,
            output,
            op,
            root,
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

        let leading_vert = if input.lookahead1().peek(Token![|]) {
            Some(input.parse()?)
        } else {
            None
        };

        let tok_refs = Punctuated::parse_separated_nonempty(input)?;

        let kind = if input.lookahead1().peek(Token![=>]) {
            NodeKind::Leaf(input.parse()?)
        } else {
            NodeKind::Branch(input.parse()?)
        };

        Ok(Self {
            label,
            leading_vert,
            tok_refs,
            kind,
        })
    }
}

impl Parse for TokenRef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(if input.lookahead1().peek(kw::token) {
            let token = input.parse()?;
            let content;
            let brace = syn::braced!(content in input);
            let body = content.parse()?;

            if !content.is_empty() {
                return Err(content.span().error("Unexpected tokens"));
            }

            Self::Anon(token, brace, body)
        } else {
            Self::Named(input.parse()?)
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
        let mut extends = vec![];

        while !(content.is_empty() || content.peek(kw::extend) || content.peek(Token![..])) {
            nodes.push(content.parse()?);
        }

        while !(content.is_empty() || content.peek(Token![..])) {
            extends.push(content.parse()?);
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
            extends,
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

impl ToTokens for Input {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            alias,
            pream_input_token,
            pream_eq_token,
            pream_ty,
            pream_semi_token,
            token_defs,
            grammars,
        } = self;

        alias.to_tokens(tokens);
        pream_input_token.to_tokens(tokens);
        pream_eq_token.to_tokens(tokens);
        pream_ty.to_tokens(tokens);
        pream_semi_token.to_tokens(tokens);

        for d in token_defs {
            d.to_tokens(tokens);
        }

        for g in grammars {
            g.to_tokens(tokens);
        }
    }
}

impl ToTokens for CrateAlias {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            use_token,
            path,
            as_token,
            shibari_token,
            semi_token,
        } = self;

        use_token.to_tokens(tokens);
        path.to_tokens(tokens);
        as_token.to_tokens(tokens);
        shibari_token.to_tokens(tokens);
        semi_token.to_tokens(tokens);
    }
}

impl ToTokens for TokenDef {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            token,
            ident,
            eq_token,
            body,
            semi_token,
        } = self;

        token.to_tokens(tokens);
        ident.to_tokens(tokens);
        eq_token.to_tokens(tokens);
        body.to_tokens(tokens);
        semi_token.to_tokens(tokens);
    }
}

impl ToTokens for TokenBody {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self { pat, op } = self;

        pat.to_tokens(tokens);
        op.to_tokens(tokens);
    }
}

impl ToTokens for TokenOp {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self { arrow_token, lit } = self;

        arrow_token.to_tokens(tokens);
        lit.to_tokens(tokens);
    }
}

impl ToTokens for GrammarDef {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            vis,
            grammar_token,
            ident,
            colon_token,
            output,
            op,
            root,
        } = self;

        vis.to_tokens(tokens);
        grammar_token.to_tokens(tokens);
        ident.to_tokens(tokens);
        colon_token.to_tokens(tokens);
        output.to_tokens(tokens);
        op.to_tokens(tokens);
        root.to_tokens(tokens);
    }
}

impl ToTokens for GrammarNode {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            label,
            leading_vert,
            tok_refs,
            kind,
        } = self;

        label.to_tokens(tokens);
        leading_vert.to_tokens(tokens);
        tok_refs.to_tokens(tokens);
        kind.to_tokens(tokens);
    }
}

impl ToTokens for TokenRef {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Named(i) => i.to_tokens(tokens),
            Self::Anon(t, b, o) => {
                t.to_tokens(tokens);
                b.surround(tokens, |tokens| o.to_tokens(tokens));
            },
        }
    }
}

impl ToTokens for NodeKind {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Leaf(l) => l.to_tokens(tokens),
            Self::Branch(b) => b.to_tokens(tokens),
        }
    }
}

impl ToTokens for NodeLeaf {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            arrow_token,
            out,
            semi_token,
        } = self;

        arrow_token.to_tokens(tokens);
        out.to_tokens(tokens);
        semi_token.to_tokens(tokens);
    }
}

impl ToTokens for Output {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Expr(y, e) => {
                y.to_tokens(tokens);
                e.to_tokens(tokens);
            },
            Self::Goto(g, l, e) => {
                g.to_tokens(tokens);
                l.to_tokens(tokens);

                if let Some((c, e)) = e.as_ref() {
                    c.to_tokens(tokens);
                    e.to_tokens(tokens);
                }
            },
            Self::Continue(c) => c.to_tokens(tokens),
        }
    }
}

impl ToTokens for NodeBranch {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            brace,
            out,
            nodes,
            extends,
            default,
        } = self;

        brace.surround(tokens, |tokens| {
            out.to_tokens(tokens);

            for n in nodes {
                n.to_tokens(tokens);
            }

            for e in extends {
                e.to_tokens(tokens);
            }

            default.to_tokens(tokens);
        });
    }
}

impl ToTokens for BranchYield {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            yield_token,
            expr,
            semi_token,
        } = self;

        yield_token.to_tokens(tokens);
        expr.to_tokens(tokens);
        semi_token.to_tokens(tokens);
    }
}

impl ToTokens for BranchDefault {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self { ddot_token, out } = self;

        ddot_token.to_tokens(tokens);
        out.to_tokens(tokens);
    }
}

impl ToTokens for GrammarExtend {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            extend_token,
            ident,
            kind,
        } = self;

        extend_token.to_tokens(tokens);
        ident.to_tokens(tokens);
        kind.to_tokens(tokens);
    }
}

impl ToTokens for ExtendKind {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::From(s) => s.to_tokens(tokens),
            Self::With(w) => w.to_tokens(tokens),
        }
    }
}

impl ToTokens for ExtendWith {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self { paren, pat, block } = self;

        paren.surround(tokens, |tokens| {
            pat.to_tokens(tokens);
            block.to_tokens(tokens);
        });
    }
}
