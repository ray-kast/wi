use std::{collections::VecDeque, fmt, rc::Rc};

use hashbrown::HashMap;
use indexmap::{
    map::{Entry, RawEntryApiV1},
    IndexMap,
};
use smallvec::SmallVec;

use super::{Delta, DeltaKind, SparseState, SparseTable, StateId};
use crate::tree::{Leaf, Root, RootId, TokenId, Tree, TreeDelta, TreeMap};

#[inline]
fn state_id(idx: usize) -> StateId {
    StateId(
        idx.try_into()
            .unwrap_or_else(|_| panic!("State ID overflow")),
    )
}

pub type ConversionPath<'tree, ExtendConversion> = SmallVec<[&'tree ExtendConversion; 1]>;

#[derive(Debug)]
pub struct TreeOutput<'tree, Output, ExtendConversion> {
    pub output: &'tree Output,
    pub conversion: ConversionPath<'tree, ExtendConversion>,
}

#[derive(Debug)]
pub struct TreeStatePath<'tree, TokenDef, RootExtra, TreeExtra> {
    pub root: &'tree RootExtra,
    pub root_tree: &'tree TreeExtra,
    pub path: Vec<(TokenId, &'tree TokenDef, &'tree TreeExtra)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Path {
    root: RootId,
    toks: SmallVec<[TokenId; 2]>,
}

struct ResolvedExtend<'tree, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra> {
    root: &'tree Root<Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>,
    tree: &'tree Tree<Output, ExtendConversion, LeafExtra, TreeExtra>,
    conversion: ConversionPath<'tree, ExtendConversion>,
}

impl<Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra> Clone
    for ResolvedExtend<'_, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>
{
    fn clone(&self) -> Self {
        Self {
            root: self.root,
            tree: self.tree,
            conversion: self.conversion.clone(),
        }
    }
}

impl<
        Output,
        ExtendConversion: fmt::Debug,
        RootExtra: fmt::Debug,
        LeafExtra,
        TreeExtra: fmt::Debug,
    > fmt::Debug for ResolvedExtend<'_, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResolvedExtend")
            .field("root", self.root.extra())
            .field("tree", self.tree.extra())
            .field("conversion", &self.conversion)
            .finish()
    }
}

type ExtendMap<'tree, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra> = IndexMap<
    Path,
    ResolvedExtend<'tree, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>,
>;
type RcExtendMap<'tree, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra> =
    Rc<ExtendMap<'tree, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>>;

#[derive(Debug)]
struct ResolvedDelta<'tree, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra> {
    next: RcExtendMap<'tree, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>,
    output: Option<&'tree Output>,
    conversion: ConversionPath<'tree, ExtendConversion>,
}

impl<'tree, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>
    ResolvedDelta<'tree, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>
{
    fn into_kind(self) -> DeltaKind<TreeOutput<'tree, Output, ExtendConversion>> {
        let Self {
            next: _,
            output,
            conversion,
        } = self;
        output.map_or(DeltaKind::Continue, |output| {
            DeltaKind::Yield(TreeOutput { output, conversion })
        })
    }
}

#[derive(Debug)]
enum LeafOrBranch<T> {
    Leaf(T),
    Branch(T),
}

impl<T> LeafOrBranch<T> {
    #[inline]
    fn inner(&self) -> &T {
        match self {
            LeafOrBranch::Leaf(t) | LeafOrBranch::Branch(t) => t,
        }
    }

    #[inline]
    fn into_inner(self) -> T {
        match self {
            LeafOrBranch::Leaf(t) | LeafOrBranch::Branch(t) => t,
        }
    }
}

#[allow(clippy::type_complexity)]
struct PathClosures<'tree, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra> {
    closed: IndexMap<
        Path,
        RcExtendMap<'tree, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>,
    >,
    scratch: ExtendMap<'tree, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>,
}

impl<
        Output: std::fmt::Debug,
        ExtendConversion: std::fmt::Debug,
        TokenDef,
        RootExtra: std::fmt::Debug,
        LeafExtra: std::fmt::Debug,
        TreeExtra: std::fmt::Debug,
    > TreeMap<Output, ExtendConversion, TokenDef, RootExtra, LeafExtra, TreeExtra>
{
    #[must_use]
    pub fn flatten<
        F: Fn(&TreeStatePath<TokenDef, RootExtra, TreeExtra>) -> StateExtra,
        StateExtra,
    >(
        &self,
        extra: F,
    ) -> IndexMap<RootId, SparseTable<TokenId, TreeOutput<'_, Output, ExtendConversion>, StateExtra>>
    {
        let mut closures = PathClosures {
            closed: IndexMap::new(),
            scratch: IndexMap::new(),
        };

        self.root_ids()
            .map(|id| self.flatten_root(id, &extra, &mut closures))
            .collect()
    }

    fn flatten_root<'tree, StateExtra>(
        &'tree self,
        id: RootId,
        extra: impl Fn(&TreeStatePath<'tree, TokenDef, RootExtra, TreeExtra>) -> StateExtra,
        closures: &mut PathClosures<
            'tree,
            Output,
            ExtendConversion,
            RootExtra,
            LeafExtra,
            TreeExtra,
        >,
    ) -> (
        RootId,
        SparseTable<TokenId, TreeOutput<'tree, Output, ExtendConversion>, StateExtra>,
    ) {
        let mut q: VecDeque<_> = [(
            self.path_closure(
                Path {
                    root: id,
                    toks: SmallVec::new_const(),
                },
                closures,
            ),
            SmallVec::new_const(),
        )]
        .into_iter()
        .collect();

        let mut resolved = IndexMap::new();

        while let Some((map, conversion)) = q.pop_front() {
            let Entry::Vacant(entry) =
                resolved.entry(map.keys().cloned().collect::<SmallVec<[_; 1]>>())
            else {
                continue;
            };

            let (deltas, default_delta) = self.resolve_deltas(&map, id, &conversion, closures);

            q.extend(
                deltas
                    .values()
                    .map(LeafOrBranch::inner)
                    .chain([&default_delta])
                    .map(|d| (Rc::clone(&d.next), d.conversion.clone())),
            );

            entry.insert((deltas, default_delta));
        }

        let state_ids: HashMap<_, _> = resolved
            .keys()
            .enumerate()
            .map(|(i, k)| (k.clone(), state_id(i)))
            .collect();

        (
            id,
            SparseTable(
                resolved
                    .into_iter()
                    .map(|(p, (d, e))| {
                        let path = p.first().unwrap_or_else(|| unreachable!());
                        let root = self.root_always(path.root);

                        SparseState {
                            delta: d
                                .into_iter()
                                .map(|(t, d)| {
                                    let d = d.into_inner();
                                    (t, Delta {
                                        next: *state_ids
                                            .get(
                                                &d.next
                                                    .keys()
                                                    .cloned()
                                                    .collect::<SmallVec<[_; 1]>>(),
                                            )
                                            .unwrap_or_else(|| unreachable!()),
                                        kind: d.into_kind(),
                                    })
                                })
                                .collect(),
                            default_delta: Delta {
                                next: *state_ids
                                    .get(&e.next.keys().cloned().collect::<SmallVec<[_; 1]>>())
                                    .unwrap_or_else(|| unreachable!()),
                                kind: e.into_kind(),
                            },
                            extra: extra(&TreeStatePath {
                                root: root.extra(),
                                root_tree: root.as_tree().extra(),
                                path: path
                                    .toks
                                    .iter()
                                    .scan(root.as_tree(), |t, &k| {
                                        *t = match t
                                            .deltas()
                                            .get(&k)
                                            .unwrap_or_else(|| unreachable!())
                                        {
                                            TreeDelta::Leaf(..) => unreachable!(),
                                            TreeDelta::Branch(b) => b.as_tree(),
                                        };

                                        Some((k, self.token_always(k), t.extra()))
                                    })
                                    .collect(),
                            }),
                        }
                    })
                    .collect(),
            ),
        )
    }

    #[allow(
        clippy::type_complexity,
        reason = "It's a 2-tuple, not much to be done"
    )]
    fn resolve_deltas<'tree>(
        &'tree self,
        map: &ExtendMap<'tree, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>,
        table_root: RootId,
        conversion_prefix: &ConversionPath<'tree, ExtendConversion>,
        closures: &mut PathClosures<
            'tree,
            Output,
            ExtendConversion,
            RootExtra,
            LeafExtra,
            TreeExtra,
        >,
    ) -> (
        IndexMap<
            TokenId,
            LeafOrBranch<
                ResolvedDelta<'tree, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>,
            >,
        >,
        ResolvedDelta<'tree, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>,
    ) {
        let mut deltas = IndexMap::new();
        let mut default_delta = None;

        for (
            path,
            &ResolvedExtend {
                root,
                tree,
                ref conversion,
            },
        ) in map
        {
            let conversion = || {
                if conversion_prefix.is_empty() {
                    conversion.clone()
                } else {
                    conversion_prefix
                        .iter()
                        .chain(conversion)
                        .copied()
                        .collect()
                }
            };

            for (&tok, delta) in tree.deltas() {
                match delta {
                    TreeDelta::Leaf(leaf, _) => {
                        deltas.insert(
                            tok,
                            LeafOrBranch::Leaf(self.resolve_leaf(
                                leaf,
                                table_root,
                                path.root,
                                root,
                                conversion(),
                                closures,
                            )),
                        );
                    },
                    TreeDelta::Branch(b) => {
                        let next = Path {
                            toks: path.toks.iter().copied().chain([tok]).collect(),
                            ..*path
                        };
                        match deltas.entry(tok) {
                            Entry::Occupied(o) => {
                                let LeafOrBranch::Branch(branch) = o.into_mut() else {
                                    continue;
                                };

                                let map = Rc::make_mut(&mut branch.next);

                                for (path, ext) in &*self.path_closure(next, closures) {
                                    map.raw_entry_mut_v1()
                                        .from_key(path)
                                        .or_insert_with(|| (path.clone(), ext.clone()));
                                }
                            },
                            Entry::Vacant(v) => {
                                v.insert(LeafOrBranch::Branch(ResolvedDelta {
                                    next: self.path_closure(next, closures),
                                    output: Some(b.out()),
                                    conversion: conversion(),
                                }));
                            },
                        }
                    },
                }
            }

            if default_delta.is_none() {
                default_delta = Some(self.resolve_leaf(
                    tree.default(),
                    table_root,
                    path.root,
                    root,
                    conversion(),
                    closures,
                ));
            }
        }

        (deltas, default_delta.unwrap_or_else(|| unreachable!()))
    }

    fn resolve_leaf<'tree>(
        &'tree self,
        leaf: &'tree Leaf<Output>,
        table_root: RootId,
        root: RootId,
        root_ref: &'tree Root<Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>,
        conversion: ConversionPath<'tree, ExtendConversion>,
        closures: &mut PathClosures<
            'tree,
            Output,
            ExtendConversion,
            RootExtra,
            LeafExtra,
            TreeExtra,
        >,
    ) -> ResolvedDelta<'tree, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra> {
        let (path, output) = match leaf {
            Leaf::Output(o) => (
                Path {
                    root: table_root,
                    toks: SmallVec::new_const(),
                },
                Some(o),
            ),
            &Leaf::Goto(l, ref o) => {
                let (_, toks) = root_ref.resolve_label_path_always(Some(l));
                (Path { root, toks }, Some(o))
            },
            &Leaf::Retry(None) => (
                Path {
                    root: table_root,
                    toks: SmallVec::new_const(),
                },
                None,
            ),
            &Leaf::Retry(Some(l)) => {
                let (_, toks) = root_ref.resolve_label_path_always(Some(l));
                (Path { root, toks }, None)
            },
        };

        ResolvedDelta {
            next: self.path_closure(path, closures),
            output,
            conversion,
        }
    }

    #[allow(
        clippy::type_complexity,
        reason = "It's a 2-tuple, not much to be done"
    )]
    fn resolve_path<'tree>(
        &'tree self,
        path: &Path,
    ) -> (
        &'tree Root<Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>,
        &'tree Tree<Output, ExtendConversion, LeafExtra, TreeExtra>,
    ) {
        let &Path { root, ref toks } = path;
        let root = self.root_always(root);
        (
            root,
            toks.iter().fold(root.as_tree(), |t, k| {
                match t.deltas().get(k).unwrap_or_else(|| unreachable!()) {
                    TreeDelta::Leaf(..) => unreachable!(),
                    TreeDelta::Branch(b) => b.as_tree(),
                }
            }),
        )
    }

    fn path_closure<'tree>(
        &'tree self,
        path: Path,
        closures: &mut PathClosures<
            'tree,
            Output,
            ExtendConversion,
            RootExtra,
            LeafExtra,
            TreeExtra,
        >,
    ) -> RcExtendMap<'tree, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra> {
        let entry = match closures.closed.entry(path) {
            Entry::Occupied(o) => return Rc::clone(o.into_mut()),
            Entry::Vacant(v) => v,
        };

        let (root, tree) = self.resolve_path(entry.key());

        debug_assert!(closures.scratch.is_empty());
        let mut q: VecDeque<_> = [(entry.key().clone(), root, tree, SmallVec::new_const())]
            .into_iter()
            .collect();

        while let Some((path, root, tree, conversion)) = q.pop_front() {
            let Entry::Vacant(v) = closures.scratch.entry(path) else {
                continue;
            };
            v.insert(ResolvedExtend {
                root,
                tree,
                conversion: conversion.clone(),
            });

            for &(root, label, ref extend_conv) in tree.extends() {
                let root_ref = self.root_always(root);
                let (tree, toks) = root_ref.resolve_label_path_always(label);

                q.push_back((
                    Path { root, toks },
                    root_ref,
                    tree,
                    conversion.iter().copied().chain([extend_conv]).collect(),
                ));
            }
        }

        Rc::clone(entry.insert(Rc::new(closures.scratch.drain(..).collect())))
    }
}
