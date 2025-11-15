use std::fmt;

use hashbrown::HashMap;
use shibari_grammar::{
    table::TreeStatePath,
    tree::{self, LeafBuilder},
};
use syn::{Block, Expr, Ident, Label, Lifetime, LitStr, Pat, Type, Visibility};

use super::parse;
use crate::prelude::*;

#[derive(Clone, PartialEq, Eq, Hash)]
pub(super) enum TokenName {
    Named(Ident),
    Anon(Pat),
}

impl fmt::Display for TokenName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Named(i) => write!(f, "named token {i}"),
            Self::Anon(_) => write!(f, "anonymous token"),
        }
    }
}

impl ToTokens for TokenName {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Named(i) => i.to_tokens(tokens),
            Self::Anon(p) => p.to_tokens(tokens),
        }
    }
}

pub(super) type RootName = Ident;

pub(super) enum Output {
    Expr(LitStr, Expr),
    Trap,
    Advance,
}

impl fmt::Debug for Output {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Expr(l, e) => write!(f, "({}, {})", l.to_token_stream(), e.to_token_stream()),
            Self::Trap => write!(f, "<Self::Output as AcceptorOutput>::TRAP"),
            Self::Advance => write!(f, "<Self::Output as AcceptorOutput>::ADVANCE"),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub(super) struct ExtendConversion {
    pub source_grammar: Ident,
    pub kind: ExtendConversionKind,
}

impl fmt::Debug for ExtendConversion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            source_grammar,
            kind,
        } = self;
        match kind {
            ExtendConversionKind::From => write!(f, "<{source_grammar}::Output as From>::from"),
            ExtendConversionKind::Explicit { param, body } => {
                write!(
                    f,
                    "|{}: {source_grammar}::Output| {}",
                    param.to_token_stream(),
                    body.to_token_stream()
                )
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[expect(clippy::large_enum_variant)]
pub(super) enum ExtendConversionKind {
    From,
    Explicit { param: Pat, body: Block },
}

pub(super) struct TokenDef {
    name: TokenName,
    pub pat: Pat,
    pub op: LitStr,
}

impl fmt::Debug for TokenDef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "<{}> {}",
            self.op.to_token_stream(),
            self.pat.to_token_stream()
        )
    }
}

pub(super) type TreeLabel = Lifetime;

pub(super) struct RootExtra {
    pub vis: Visibility,
    pub ident: Ident,
    pub output: Type,
    pub op: Option<LitStr>,
}

impl fmt::Debug for RootExtra {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            vis,
            ident,
            output,
            op,
        } = self;
        write!(
            f,
            "{} {ident} -> {}",
            vis.to_token_stream(),
            output.to_token_stream(),
        )?;

        if let Some(op) = op {
            write!(f, "=> {}", op.to_token_stream())?;
        }

        Ok(())
    }
}

pub(super) struct LeafExtra {
    token_ref_span: Span,
}

impl fmt::Debug for LeafExtra {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str("LeafExtra") }
}

pub(super) struct TreeExtra {
    id_span: Span,
    label: Option<Label>,
}

impl fmt::Debug for TreeExtra {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str("TreeExtra") }
}

pub(super) type TreeMapBuilder = tree::TreeMapBuilder<
    TokenName,
    RootName,
    Output,
    ExtendConversion,
    TokenDef,
    TreeLabel,
    RootExtra,
    LeafExtra,
    TreeExtra,
>;
pub(super) type TreeBuilder = tree::TreeBuilder<
    TokenName,
    RootName,
    Output,
    ExtendConversion,
    TreeLabel,
    LeafExtra,
    TreeExtra,
>;
pub(super) type BuildError =
    tree::BuildError<TokenName, RootName, TokenDef, TreeLabel, RootExtra, LeafExtra, TreeExtra>;
pub(super) type TreeMap =
    tree::TreeMap<Output, ExtendConversion, TokenDef, RootExtra, LeafExtra, TreeExtra>;

fn leaf_builder(output: &parse::Output, op: &LitStr) -> tree::LeafBuilder<Output, TreeLabel> {
    match output {
        parse::Output::Expr(_, e) => LeafBuilder::Output(Output::Expr(op.clone(), e.clone())),
        parse::Output::Goto(_, l, o) => LeafBuilder::Goto(
            l.clone(),
            o.as_ref().map(|(_, e)| Output::Expr(op.clone(), e.clone())),
        ),
        parse::Output::Continue(_) => LeafBuilder::Retry(None),
    }
}

pub(super) struct StateExtra {
    pub op: LitStr,
}

impl fmt::Debug for StateExtra {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.op.to_token_stream())
    }
}

impl StateExtra {
    pub fn from_path(path: &TreeStatePath<TokenDef, RootExtra, TreeExtra>) -> Self {
        Self {
            op: LitStr::new(
                &path
                    .root
                    .op
                    .iter()
                    .map(syn::LitStr::value)
                    .chain(path.path.iter().map(|&(_, d, _)| d.op.value()))
                    .collect::<String>(),
                path.path
                    .last()
                    .map_or_else(|| &path.root_tree, |(_, _, t)| t)
                    .id_span,
            ),
        }
    }
}

struct ErrorAdapter<'d>(&'d mut TokenStream);

impl Extend<BuildError> for ErrorAdapter<'_> {
    fn extend<T: IntoIterator<Item = BuildError>>(&mut self, iter: T) {
        self.0.extend(iter.into_iter().map(|e| {
            match e {
                tree::BuildError::DuplicateToken(n, d) => d
                    .name
                    .span()
                    .error(format!("Duplicate token definition for {n}")),
                tree::BuildError::DuplicateRoot(n, _, t) => t
                    .id_span
                    .error(format!("Duplicate grammar definition for {n}")),

                tree::BuildError::DuplicateLeaf(r, k, l) => l
                    .token_ref_span
                    .error(format!("Duplicate leaf delta for {k} in grammar {r}")),

                tree::BuildError::DuplicateBranch(r, k, t) => t
                    .id_span
                    .error(format!("Duplicate branch delta for {k} in grammar {r}")),

                tree::BuildError::DuplicateLabel(r, l) => {
                    l.span().error(format!("Duplicate label in grammar {r}"))
                },
                tree::BuildError::UnknownToken(k) => k.span().error("Unknown token name"),
                tree::BuildError::UnknownRoot(r) => r.span().error("Unknown root name"),
                tree::BuildError::UnknownLabel(r, l) => {
                    l.span().error(format!("Unknown label name in grammar {r}"))
                },
            }
            .into_compile_error()
        }));
    }
}

pub(super) fn parse_grammars(
    token_defs: Vec<parse::TokenDef>,
    grammars: Vec<parse::GrammarDef>,
    diag: &mut TokenStream,
) -> TreeMap {
    let mut b = TreeMap::builder();

    let token_ops: HashMap<_, _> = token_defs
        .iter()
        .map(|d| (d.ident.clone(), d.body.op.lit.clone()))
        .collect();

    for parse::TokenDef {
        token: _,
        ident,
        eq_token: _,
        body:
            parse::TokenBody {
                pat,
                op:
                    parse::TokenOp {
                        arrow_token: _,
                        lit,
                    },
            },
        semi_token: _,
    } in token_defs
    {
        let name = TokenName::Named(ident.clone());
        b.token(name.clone(), TokenDef { name, pat, op: lit });
    }

    for grammar in &grammars {
        visit_anon_tokens(&mut b, &grammar.root);
    }

    for parse::GrammarDef {
        vis,
        grammar_token: _,
        ident,
        colon_token: _,
        output,
        op,
        root:
            parse::NodeBranch {
                brace: _,
                out,
                nodes,
                extends,
                default,
            },
    } in grammars
    {
        if let Some(o) = out {
            diag.extend(
                o.span()
                    .error("Branch yield not allowed for grammar roots")
                    .into_compile_error(),
            );
        }

        b.root_with_extra(
            ident.clone(),
            TreeExtra {
                id_span: ident.span(),
                label: None,
            },
            RootExtra {
                vis,
                ident,
                output,
                op: op.map(
                    |parse::TokenOp {
                         arrow_token: _,
                         lit,
                     }| lit,
                ),
            },
            |b| build_tree(b, &nodes, &extends, default.as_ref(), &token_ops, diag),
        );
    }

    let trees = b.build_recoverable(&mut ErrorAdapter(diag), || Output::Trap, || Output::Advance);

    let mut unused = TokenStream::new();
    for (_, token) in trees.unused_tokens() {
        let name = &token.name;
        unused.extend(quote_spanned! { name.span() => struct #name; });
    }

    for (i, (_, _, _, (tree, _))) in trees.unused_labels().enumerate() {
        let label = tree
            .extra()
            .label
            .as_ref()
            .unwrap_or_else(|| unreachable!());
        let span = label.span();

        let ident = Ident::new(&format!("__{i}"), span);

        unused.extend(quote_spanned! { span => fn #ident() { #label {} } });
    }

    if !unused.is_empty() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        static FREE: AtomicUsize = AtomicUsize::new(0);

        let free = FREE.fetch_add(1, Ordering::SeqCst);
        let id = Ident::new(
            &format!("__shibari_static_acceptors_unused__{free}"),
            Span::call_site(),
        );

        diag.extend(quote_spanned! { Span::call_site() =>
            #[doc(hidden)] mod #id { #unused }
        });
    }

    trees
}

fn visit_anon_tokens(b: &mut TreeMapBuilder, branch: &parse::NodeBranch) {
    for node in &branch.nodes {
        for tok_ref in &node.tok_refs {
            if let parse::TokenRef::Anon(
                _,
                _,
                parse::TokenBody {
                    pat,
                    op:
                        parse::TokenOp {
                            arrow_token: _,
                            lit,
                        },
                },
            ) = tok_ref
            {
                let name = TokenName::Anon(pat.clone());
                b.token(name.clone(), TokenDef {
                    name,
                    pat: pat.clone(),
                    op: lit.clone(),
                });
            }
        }

        if let parse::NodeKind::Branch(ref branch) = node.kind {
            visit_anon_tokens(b, branch);
        }
    }
}

fn build_tree<'b>(
    b: &'b mut TreeBuilder,
    nodes: &[parse::GrammarNode],
    extends: &[parse::GrammarExtend],
    default: Option<&parse::BranchDefault>,
    token_ops: &HashMap<Ident, LitStr>,
    diag: &mut TokenStream,
) -> &'b mut TreeBuilder {
    for parse::GrammarNode {
        label,
        leading_vert: _,
        tok_refs,
        kind,
    } in nodes
    {
        for tok_ref in tok_refs {
            let token_ref_span = tok_ref.span();
            let empty_op = LitStr::new("", token_ref_span);
            let (token_name, token_op) = match tok_ref {
                parse::TokenRef::Named(i) => (
                    TokenName::Named(i.clone()),
                    token_ops.get(i).unwrap_or(&empty_op),
                ),
                parse::TokenRef::Anon(_, _, parse::TokenBody { pat, op }) => {
                    (TokenName::Anon(pat.clone()), &op.lit)
                },
            };

            match &kind {
                parse::NodeKind::Leaf(parse::NodeLeaf {
                    arrow_token: _,
                    out,
                    semi_token: _,
                }) => {
                    if let Some(label) = label {
                        diag.extend(
                            label
                                .span()
                                .error("Labels not allowed for leaf deltas")
                                .into_compile_error(),
                        );
                    }

                    b.leaf_with_extra(token_name, leaf_builder(out, token_op), LeafExtra {
                        token_ref_span,
                    })
                },
                parse::NodeKind::Branch(parse::NodeBranch {
                    brace: _,
                    out,
                    nodes,
                    extends,
                    default,
                }) => b.branch_with_extra(
                    token_name,
                    TreeExtra {
                        id_span: token_ref_span,
                        label: label.clone(),
                    },
                    |b| {
                        if let Some(label) = label {
                            b.label(label.name.clone());
                        }

                        if let Some(parse::BranchYield {
                            yield_token,
                            expr,
                            semi_token: _,
                        }) = out
                        {
                            b.out(Output::Expr(
                                LitStr::new("", yield_token.span()),
                                expr.clone(),
                            ));
                        }

                        build_tree(b, nodes, extends, default.as_ref(), token_ops, diag);
                        b
                    },
                ),
            };
        }
    }

    for parse::GrammarExtend {
        extend_token: _,
        ident,
        kind,
    } in extends
    {
        b.extend(ident.clone(), None, ExtendConversion {
            source_grammar: ident.clone(),
            kind: match kind {
                parse::ExtendKind::From(_) => ExtendConversionKind::From,
                parse::ExtendKind::With(parse::ExtendWith {
                    paren: _,
                    pat,
                    block,
                }) => ExtendConversionKind::Explicit {
                    param: pat.clone(),
                    body: block.clone(),
                },
            },
        });
    }

    if let Some(parse::BranchDefault { ddot_token: _, out }) = default {
        b.default(leaf_builder(out, &LitStr::new("", out.span())));
    }

    b
}
