use syn::{token, Block, Expr, Ident, Label, Lifetime, LitStr, Pat, Path, Token, Type, Visibility};

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

    #[expect(unused)]
    pub(super) pream_input_token: kw::input,
    #[expect(unused)]
    pub(super) pream_eq_token: Token![=],
    pub(super) pream_ty: Type,
    #[expect(unused)]
    pub(super) pream_semi_token: Token![;],

    pub(super) tokens: Vec<TokenDef>,
    pub(super) grammars: Vec<GrammarDef>,
}

pub(super) struct CrateAlias {
    #[expect(unused)]
    pub use_token: Token![use],
    pub path: Path,
    #[expect(unused)]
    pub as_token: Token![as],
    #[expect(unused)]
    pub shibari_token: kw::shibari,
    #[expect(unused)]
    pub semi_token: Token![;],
}

pub(super) struct TokenDef {
    #[expect(unused)]
    pub token: kw::token,
    pub ident: Ident,
    #[expect(unused)]
    pub eq_token: Token![=],
    pub pat: Pat,
    pub op: Option<TokenOp>,
    #[expect(unused)]
    pub semi_token: Token![;],
}

pub(super) struct TokenOp {
    #[expect(unused)]
    pub arrow_token: Token![=>],
    pub lit: LitStr,
}

pub(super) struct GrammarDef {
    pub vis: Visibility,
    #[expect(unused)]
    pub grammar_token: kw::grammar,
    pub ident: Ident,
    #[expect(unused)]
    pub colon_token: Token![:],
    pub output: Type,

    pub root: NodeBranch,
}

pub(super) struct GrammarNode {
    pub label: Option<Label>,
    pub tok_ident: Ident,
    pub kind: NodeKind,
}

#[expect(clippy::large_enum_variant)]
pub(super) enum NodeKind {
    Leaf(NodeLeaf),
    Branch(NodeBranch),
}

pub(super) struct NodeLeaf {
    #[expect(unused)]
    pub arrow_token: Token![=>],
    pub out: Output,
    #[expect(unused)]
    pub semi_token: Token![;],
}

pub(super) enum Output {
    Expr(#[expect(unused)] Token![yield], Expr),
    Goto(
        #[expect(unused)] kw::goto,
        Lifetime,
        Option<(Token![,], Expr)>,
    ),
    Continue(#[expect(unused)] Token![continue]),
}

pub(super) struct NodeBranch {
    #[expect(unused)]
    pub brace: token::Brace,
    pub out: Option<BranchYield>,
    pub nodes: Vec<GrammarNode>,
    pub extends: Vec<GrammarExtend>,
    pub default: Option<BranchDefault>,
}

pub struct BranchYield {
    #[expect(unused)]
    pub yield_token: Token![yield],
    pub expr: Expr,
    #[expect(unused)]
    pub semi_token: Token![;],
}

pub struct BranchDefault {
    #[expect(unused)]
    pub ddot_token: Token![..],
    pub out: Output,
}

pub struct GrammarExtend {
    #[expect(unused)]
    pub extend_token: kw::extend,
    pub ident: Ident,
    pub kind: ExtendKind,
}

#[allow(clippy::large_enum_variant)]
pub(super) enum ExtendKind {
    From(#[expect(unused)] Token![;]),
    With(ExtendWith),
}

pub(super) struct ExtendWith {
    #[expect(unused)]
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
            alias,
            pream_input_token,
            pream_eq_token,
            pream_ty,
            pream_semi_token,
            tokens,
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
        let root = input.parse()?;

        Ok(Self {
            vis,
            grammar_token,
            ident,
            colon_token,
            output,
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
