use std::hash::Hash;

use hashbrown::HashMap;
use indexmap::IndexMap;
use smallvec::SmallVec;

use super::{
    Branch, BranchBuilder, DeltaBuilder, LabelId, Leaf, LeafBuilder, Root, RootId, TokenId, Tree,
    TreeBuilder, TreeDelta, TreeMap, TreeMapBuilder,
};

#[derive(Debug, thiserror::Error)]
pub enum BuildError<TokenName, RootName, TokenDef, TreeLabel, RootExtra, LeafExtra, TreeExtra> {
    #[error("Duplicate token definition")]
    DuplicateToken(TokenName, TokenDef),
    #[error("Duplicate root definition")]
    DuplicateRoot(RootName, RootExtra, TreeExtra),
    #[error("Duplicate leaf delta")]
    DuplicateLeaf(RootName, TokenName, LeafExtra),
    #[error("Duplicate branch delta")]
    DuplicateBranch(RootName, TokenName, TreeExtra),
    #[error("Duplicate tree label")]
    DuplicateLabel(RootName, TreeLabel),

    #[error("Unknown token name")]
    UnknownToken(TokenName),
    #[error("Unknown root name")]
    UnknownRoot(RootName),
    #[error("Unknown label name")]
    UnknownLabel(RootName, TreeLabel),
}

#[inline]
fn token_id(idx: usize) -> TokenId {
    TokenId(
        idx.try_into()
            .unwrap_or_else(|_| panic!("Token ID overflow")),
    )
}

#[inline]
fn root_id(idx: usize) -> RootId {
    RootId(
        idx.try_into()
            .unwrap_or_else(|_| panic!("Root ID overflow")),
    )
}

#[inline]
fn label_id(idx: usize) -> LabelId {
    LabelId(
        idx.try_into()
            .unwrap_or_else(|_| panic!("Label ID overflow")),
    )
}

enum LabelResolution<TreeLabel> {
    Unresolved(Vec<TreeLabel>),
    Resolved,
    Duplicate,
}

type LabelMap<RootName, TreeLabel> =
    HashMap<TreeLabel, (LabelId, HashMap<RootName, LabelResolution<TreeLabel>>)>;

#[inline]
fn resolve_label<
    RootName: Clone + Eq + Hash,
    TreeLabel: Clone + Eq + Hash,
    E: Extend<BuildError<TokenName, RootName, TokenDef, TreeLabel, RootExtra, LeafExtra, TreeExtra>>,
    TokenName,
    TokenDef,
    RootExtra,
    LeafExtra,
    TreeExtra,
>(
    label: TreeLabel,
    root: &RootName,
    used: bool,
    map: &mut LabelMap<RootName, TreeLabel>,
    errors: &mut E,
) -> LabelId {
    use hashbrown::hash_map::RawEntryMut;

    let free = map.len();
    match map.raw_entry_mut().from_key(&label) {
        RawEntryMut::Occupied(o) => {
            let (old_label, (id, res)) = o.into_key_value();

            match (used, res.raw_entry_mut().from_key(root)) {
                (true, RawEntryMut::Occupied(o)) => {
                    let res = o.into_mut();

                    match res {
                        LabelResolution::Unresolved(_) => {
                            *old_label = label;
                            *res = LabelResolution::Resolved;
                        },
                        LabelResolution::Resolved => {
                            errors.extend([
                                BuildError::DuplicateLabel(root.clone(), old_label.clone()),
                                BuildError::DuplicateLabel(root.clone(), label),
                            ]);
                            *res = LabelResolution::Duplicate;
                        },
                        LabelResolution::Duplicate => {
                            errors.extend([BuildError::DuplicateLabel(root.clone(), label)]);
                        },
                    }
                },
                (false, RawEntryMut::Occupied(o)) => {
                    if let LabelResolution::Unresolved(v) = o.into_mut() {
                        v.push(label);
                    }
                },
                (u, RawEntryMut::Vacant(v)) => {
                    v.insert(
                        root.clone(),
                        if u {
                            LabelResolution::Resolved
                        } else {
                            LabelResolution::Unresolved(vec![])
                        },
                    );
                },
            }

            *id
        },
        RawEntryMut::Vacant(v) => {
            v.insert(
                label,
                (
                    label_id(free),
                    [(
                        root.clone(),
                        if used {
                            LabelResolution::Resolved
                        } else {
                            LabelResolution::Unresolved(vec![])
                        },
                    )]
                    .into_iter()
                    .collect(),
                ),
            )
            .1
             .0
        },
    }
}

#[inline]
fn dedup<
    K,
    A: smallvec::Array,
    E: Extend<BuildError<TokenName, RootName, TokenDef, TreeLabel, RootExtra, LeafExtra, TreeExtra>>,
    R: FnMut(
        &K,
        A::Item,
    )
        -> BuildError<TokenName, RootName, TokenDef, TreeLabel, RootExtra, LeafExtra, TreeExtra>,
    // F: FnMut(&K, &mut A::Item),
    TokenName,
    RootName,
    TokenDef,
    TreeLabel,
    RootExtra,
    LeafExtra,
    TreeExtra,
>(
    map: &mut IndexMap<K, SmallVec<A>>,
    errors: &mut E,
    mut error: R,
    // mut f: F,
) -> impl Iterator<Item = K> {
    map.extract_if(.., move |_, v| {
        if v.len() != 1 {
            return true;
        }

        // f(k, v.first_mut().unwrap_or_else(|| unreachable!()));

        false
    })
    .map(move |(k, v)| {
        if v.is_empty() {
            unreachable!();
        }

        errors.extend(v.into_iter().map(|v| error(&k, v)));

        k
    })
}

#[inline]
fn extract_deduped<T>(v: SmallVec<[T; 1]>) -> T {
    let Ok([v]) = v.into_inner() else {
        unreachable!();
    };

    v
}

impl<
        TokenName: Clone + Eq + Hash,
        RootName: Clone + Eq + Hash,
        Output,
        ExtendConversion,
        TokenDef,
        TreeLabel: Clone + Eq + Hash,
        RootExtra,
        LeafExtra,
        TreeExtra,
    >
    TreeMapBuilder<
        TokenName,
        RootName,
        Output,
        ExtendConversion,
        TokenDef,
        TreeLabel,
        RootExtra,
        LeafExtra,
        TreeExtra,
    >
{
    pub fn build_recoverable<
        E: Extend<
            BuildError<TokenName, RootName, TokenDef, TreeLabel, RootExtra, LeafExtra, TreeExtra>,
        >,
        FT: Copy + Fn() -> Output,
        FA: Copy + Fn() -> Output,
    >(
        &mut self,
        errors: &mut E,
        trap_out: FT,
        advance_out: FA,
    ) -> TreeMap<Output, ExtendConversion, TokenDef, RootExtra, LeafExtra, TreeExtra> {
        let Self {
            tokens: in_tokens,
            roots: in_roots,
        } = self;

        let token_ids: HashMap<_, _> = dedup(in_tokens, errors, |n, d| {
            BuildError::DuplicateToken(n.clone(), d)
        })
        .map(|k| (k, None))
        .collect();

        let mut zipped = (token_ids, vec![]);
        zipped.extend(
            in_tokens
                .drain(..)
                .enumerate()
                .map(|(i, (n, d))| ((n, Some(token_id(i))), extract_deduped(d))),
        );
        let (token_ids, tokens) = zipped;

        let root_ids: IndexMap<_, _> = dedup(in_roots, errors, |n, r| {
            BuildError::DuplicateRoot(n.clone(), r.1, r.0.extra)
        })
        .map(|k| (k, None))
        .collect();

        let root_id_start = root_ids.len();
        let mut zipped = (root_ids, vec![]);
        zipped.extend(
            in_roots
                .drain(..)
                .enumerate()
                .map(|(i, (n, r))| ((n, Some(root_id(i))), extract_deduped(r))),
        );
        let (root_ids, roots) = zipped;

        #[cfg(debug_assertions)]
        let root_len = roots.len();

        let mut label_ids = HashMap::new();
        let roots = roots
            .into_iter()
            .zip(&root_ids[root_id_start..])
            .map(|((r, extra), (n, _))| Root {
                tree: r.build_recoverable(
                    RootCx {
                        root: n,
                        token_ids: &token_ids,
                        root_ids: &root_ids,
                    },
                    &mut label_ids,
                    trap_out,
                    advance_out,
                    errors,
                ),
                extra,
            })
            .collect();

        debug_assert!(Vec::len(&roots) == root_len);

        for (label, (_, res)) in label_ids {
            for (root, res) in res {
                if let LabelResolution::Unresolved(v) = res {
                    errors.extend(
                        [label.clone()]
                            .into_iter()
                            .chain(v)
                            .map(|l| BuildError::UnknownLabel(root.clone(), l)),
                    );
                }
            }
        }

        TreeMap { tokens, roots }
    }
}

struct RootCx<'a, TokenName, RootName> {
    root: &'a RootName,
    token_ids: &'a HashMap<TokenName, Option<TokenId>>,
    root_ids: &'a IndexMap<RootName, Option<RootId>>,
}

impl<T, R> Copy for RootCx<'_, T, R> {}

impl<T, R> Clone for RootCx<'_, T, R> {
    #[inline]
    fn clone(&self) -> Self { *self }
}

impl<
        TokenName: Clone + Eq + Hash,
        RootName: Clone + Eq + Hash,
        Output,
        ExtendConversion,
        TreeLabel: Clone + Eq + Hash,
        LeafExtra,
        TreeExtra,
    > TreeBuilder<TokenName, RootName, Output, ExtendConversion, TreeLabel, LeafExtra, TreeExtra>
{
    fn build_recoverable<
        E: Extend<
            BuildError<TokenName, RootName, TokenDef, TreeLabel, RootExtra, LeafExtra, TreeExtra>,
        >,
        TokenDef,
        RootExtra,
    >(
        self,
        cx: RootCx<TokenName, RootName>,
        label_ids: &mut LabelMap<RootName, TreeLabel>,
        trap_out: impl Copy + Fn() -> Output,
        advance_out: impl Copy + Fn() -> Output,
        errors: &mut E,
    ) -> Tree<Output, ExtendConversion, LeafExtra, TreeExtra> {
        let Self {
            mut deltas,
            extends,
            default,
            extra,
        } = self;

        let () = dedup(&mut deltas, errors, |t, d| match d {
            DeltaBuilder::Leaf(_, e) => BuildError::DuplicateLeaf(cx.root.clone(), t.clone(), e),
            DeltaBuilder::Branch(b) => {
                BuildError::DuplicateBranch(cx.root.clone(), t.clone(), b.tree.extra)
            },
        })
        .map(|_| ())
        .collect();

        Tree {
            deltas: deltas
                .into_iter()
                .filter_map(|(t, d)| {
                    let Some(&id) = cx.token_ids.get(&t) else {
                        errors.extend([BuildError::UnknownToken(t)]);
                        return None;
                    };
                    let id = id?;
                    Some((id, match extract_deduped(d) {
                        DeltaBuilder::Leaf(l, e) => {
                            TreeDelta::Leaf(l.build(cx.root, label_ids, advance_out, errors), e)
                        },
                        DeltaBuilder::Branch(BranchBuilder { label, out, tree }) => {
                            TreeDelta::Branch(Branch {
                                label: label
                                    .map(|l| resolve_label(l, cx.root, true, label_ids, errors)),
                                out: out.unwrap_or_else(advance_out),
                                tree: tree.build_recoverable(
                                    cx,
                                    label_ids,
                                    trap_out,
                                    advance_out,
                                    errors,
                                ),
                            })
                        },
                    }))
                })
                .collect(),
            extends: extends
                .into_iter()
                .filter_map(|(r, l, c)| {
                    let Some(&id) = cx.root_ids.get(&r) else {
                        errors.extend([BuildError::UnknownRoot(r)]);
                        return None;
                    };
                    let id = id?;
                    Some((
                        id,
                        l.map(|l| resolve_label(l, cx.root, false, label_ids, errors)),
                        c,
                    ))
                })
                .collect(),
            default: default.map_or_else(
                || Leaf::Output(trap_out()),
                |l| l.build(cx.root, label_ids, advance_out, errors),
            ),
            extra,
        }
    }
}

impl<Output, TreeLabel: Clone + Eq + Hash> LeafBuilder<Output, TreeLabel> {
    fn build<
        E: Extend<
            BuildError<TokenName, RootName, TokenDef, TreeLabel, RootExtra, LeafExtra, TreeExtra>,
        >,
        TokenName,
        RootName: Clone + Eq + Hash,
        TokenDef,
        RootExtra,
        LeafExtra,
        TreeExtra,
    >(
        self,
        root: &RootName,
        label_ids: &mut LabelMap<RootName, TreeLabel>,
        advance_out: impl Fn() -> Output,
        errors: &mut E,
    ) -> Leaf<Output> {
        match self {
            LeafBuilder::Output(o) => Leaf::Output(o),
            LeafBuilder::Goto(l, o) => Leaf::Goto(
                resolve_label(l, root, false, label_ids, errors),
                o.unwrap_or_else(advance_out),
            ),
            LeafBuilder::Retry(l) => {
                Leaf::Retry(l.map(|l| resolve_label(l, root, false, label_ids, errors)))
            },
        }
    }
}
