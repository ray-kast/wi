use std::{collections::VecDeque, hash::Hash};

use hashbrown::{hash_map::Entry as HashEntry, HashMap};
use indexmap::{map::Entry as IndexEntry, IndexMap};
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

#[derive(Debug)]
struct PathData<'tree, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra> {
    root: &'tree Root<Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>,
    tree: &'tree Tree<Output, ExtendConversion, LeafExtra, TreeExtra>,
    conversion: ConversionPath<'tree, ExtendConversion>,
}

impl<Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra> Clone
    for PathData<'_, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>
{
    fn clone(&self) -> Self {
        Self {
            root: self.root,
            tree: self.tree,
            conversion: self.conversion.clone(),
        }
    }
}

type PathSet<'tree, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra> =
    IndexMap<Path, PathData<'tree, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>>;

type PathDeltaMap<'tree, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra> = IndexMap<
    TokenId,
    LeafOrBranch<PathDelta<'tree, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>>,
>;

#[derive(Debug)]
struct PathDelta<'tree, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra> {
    next: PathSet<'tree, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>,
    output: Option<TreeOutput<'tree, Output, ExtendConversion>>,
}

struct PathSetDelta<'tree, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra> {
    by_token: PathDeltaMap<'tree, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>,
    default_delta: PathDelta<'tree, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>,
}

impl<'tree, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>
    PathDelta<'tree, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>
{
    fn build(
        self,
        state_ids: &HashMap<SmallVec<[Path; 1]>, StateId>,
    ) -> Delta<TreeOutput<'tree, Output, ExtendConversion>> {
        let Self { next, output } = self;
        Delta {
            next: *state_ids
                .get(&path_set_id(&next))
                .unwrap_or_else(|| unreachable!()),
            kind: output.map_or(DeltaKind::Continue, DeltaKind::Yield),
        }
    }
}

type StatesByPath<'tree, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra> = IndexMap<
    SmallVec<[Path; 1]>,
    PathSetDelta<'tree, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>,
>;

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

#[inline]
fn path_set_id<Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>(
    set: &PathSet<'_, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>,
) -> SmallVec<[Path; 1]> {
    set.keys().cloned().collect()
}

impl<Output, ExtendConversion: Eq + Hash, TokenDef, RootExtra, LeafExtra, TreeExtra>
    TreeMap<Output, ExtendConversion, TokenDef, RootExtra, LeafExtra, TreeExtra>
{
    pub fn flatten<
        'tree,
        StateExtra,
        F: FnMut(&TreeStatePath<'tree, TokenDef, RootExtra, TreeExtra>) -> StateExtra,
    >(
        &'tree self,
        mut extra: F,
    ) -> HashMap<
        RootId,
        SparseTable<TokenId, TreeOutput<'tree, Output, ExtendConversion>, StateExtra>,
    > {
        self.root_ids()
            .map(|i| {
                (
                    i,
                    self.flatten_root(i, &mut extra)
                        .unwrap_or_else(|| unreachable!()),
                )
            })
            .collect()
    }

    #[must_use]
    pub fn flatten_root<
        'tree,
        StateExtra,
        F: FnMut(&TreeStatePath<'tree, TokenDef, RootExtra, TreeExtra>) -> StateExtra,
    >(
        &'tree self,
        id: RootId,
        extra: F,
    ) -> Option<SparseTable<TokenId, TreeOutput<'tree, Output, ExtendConversion>, StateExtra>> {
        let table_root_id = id; // Better name
        let table_root = self.root(table_root_id)?;
        let mut q: VecDeque<IndexMap<_, _>> = [[(
            Path {
                root: table_root_id,
                toks: SmallVec::new_const(),
            },
            PathData {
                root: table_root,
                tree: table_root,
                conversion: SmallVec::new_const(),
            },
        )]
        .into_iter()
        .collect()]
        .into_iter()
        .collect();

        let mut path_closures = HashMap::new();
        let mut states_by_path = IndexMap::new();

        while let Some(mut set) = q.pop_front() {
            let HashEntry::Vacant(closure) = path_closures.entry(path_set_id(&set)) else {
                continue;
            };

            self.close_path_set(&mut set);

            let closed = path_set_id(&set);
            closure.insert(closed.clone());
            let IndexEntry::Vacant(state) = states_by_path.entry(closed) else {
                continue;
            };

            let delta = Self::path_set_delta(set, table_root_id, table_root);

            q.extend(
                delta
                    .by_token
                    .values()
                    .map(LeafOrBranch::inner)
                    .chain([&delta.default_delta])
                    .map(|d| d.next.clone()),
            );

            state.insert(delta);
        }

        let state_ids: HashMap<_, _> = path_closures
            .into_iter()
            .map(|(p, c)| {
                (
                    p,
                    state_id(
                        states_by_path
                            .get_index_of(&c)
                            .unwrap_or_else(|| unreachable!()),
                    ),
                )
            })
            .collect();

        Some(self.resolve_state_paths(states_by_path, &state_ids, extra))
    }

    fn resolve_state_paths<'tree, StateExtra>(
        &'tree self,
        states_by_path: StatesByPath<
            'tree,
            Output,
            ExtendConversion,
            RootExtra,
            LeafExtra,
            TreeExtra,
        >,
        state_ids: &HashMap<SmallVec<[Path; 1]>, StateId>,
        mut extra: impl FnMut(&TreeStatePath<'tree, TokenDef, RootExtra, TreeExtra>) -> StateExtra,
    ) -> SparseTable<TokenId, TreeOutput<'tree, Output, ExtendConversion>, StateExtra> {
        SparseTable(
            states_by_path
                .into_iter()
                .map(
                    |(
                        path,
                        PathSetDelta {
                            by_token,
                            default_delta,
                        },
                    )| {
                        let path = path.first().unwrap_or_else(|| unreachable!());
                        let root = self.root_always(path.root);

                        SparseState {
                            delta: by_token
                                .into_iter()
                                .map(|(t, d)| (t, d.into_inner().build(state_ids)))
                                .collect(),
                            default_delta: default_delta.build(state_ids),
                            extra: extra(&TreeStatePath {
                                root: root.extra(),
                                root_tree: root.as_tree().extra(),
                                path: path
                                    .toks
                                    .iter()
                                    .scan(root.as_tree(), |t, &k| {
                                        let Some(TreeDelta::Branch(branch)) = t.deltas().get(&k)
                                        else {
                                            unreachable!()
                                        };
                                        *t = branch;

                                        Some((k, self.token_always(k), t.extra()))
                                    })
                                    .collect(),
                            }),
                        }
                    },
                )
                .collect(),
        )
    }

    fn close_path_set<'tree>(
        &'tree self,
        set: &mut PathSet<'tree, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>,
    ) {
        let mut i = 0;
        while let Some((_, data)) = set.get_index(i) {
            let mut to_insert = vec![];

            for &(root_id, label, ref ext_conversion) in data.tree.extends() {
                let root = self.root_always(root_id);
                let (tree, toks) = root.resolve_label_path_always(label);

                let ext_path = Path {
                    root: root_id,
                    toks,
                };

                let mut conversion = data.conversion.clone();
                conversion.push(ext_conversion);
                if !set.contains_key(&ext_path) {
                    to_insert.push((ext_path, PathData {
                        root,
                        tree,
                        conversion,
                    }));
                }
            }

            set.extend(to_insert);

            i += 1;
        }
    }

    fn path_set_delta<'tree>(
        set: PathSet<'tree, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>,
        table_root_id: RootId,
        table_root: &'tree Root<Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>,
    ) -> PathSetDelta<'tree, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra> {
        let mut by_token = IndexMap::new();
        let mut default_delta = None;

        for (path, data) in set {
            for (&token, delta) in data.tree.deltas() {
                match by_token.entry(token) {
                    IndexEntry::Occupied(o) => match (delta, o.into_mut()) {
                        (TreeDelta::Leaf(leaf, _), entry @ LeafOrBranch::Branch(_)) => {
                            *entry = LeafOrBranch::Leaf(Self::leaf_delta(
                                leaf,
                                data.conversion.clone(),
                                table_root_id,
                                table_root,
                                path.root,
                                data.root,
                            ));
                        },
                        (TreeDelta::Branch(branch), LeafOrBranch::Branch(entry)) => {
                            let mut next = path.clone();
                            next.toks.push(token);

                            entry.next.entry(next).or_insert_with(|| PathData {
                                tree: branch,
                                conversion: data.conversion.clone(),
                                ..data
                            });
                        },
                        _ => (),
                    },
                    IndexEntry::Vacant(v) => {
                        v.insert(match delta {
                            TreeDelta::Leaf(leaf, _) => LeafOrBranch::Leaf(Self::leaf_delta(
                                leaf,
                                data.conversion.clone(),
                                table_root_id,
                                table_root,
                                path.root,
                                data.root,
                            )),
                            TreeDelta::Branch(branch) => {
                                let mut next = path.clone();
                                next.toks.push(token);

                                LeafOrBranch::Branch(PathDelta {
                                    next: [(next, PathData {
                                        tree: branch,
                                        conversion: data.conversion.clone(),
                                        ..data
                                    })]
                                    .into_iter()
                                    .collect(),
                                    output: Some(TreeOutput {
                                        output: branch.out(),
                                        conversion: data.conversion.clone(),
                                    }),
                                })
                            },
                        });
                    },
                }
            }

            if default_delta.is_none() {
                default_delta = Some(Self::leaf_delta(
                    data.tree.default(),
                    data.conversion,
                    table_root_id,
                    table_root,
                    path.root,
                    data.root,
                ));
            }
        }

        PathSetDelta {
            by_token,
            default_delta: default_delta.unwrap_or_else(|| unreachable!()),
        }
    }

    fn leaf_delta<'tree>(
        leaf: &'tree Leaf<Output>,
        conversion: ConversionPath<'tree, ExtendConversion>,
        table_root_id: RootId,
        table_root: &'tree Root<Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>,
        ext_root_id: RootId,
        ext_root: &'tree Root<Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>,
    ) -> PathDelta<'tree, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra> {
        let (path, root, tree, output) = match leaf {
            Leaf::Output(o) => (
                Path {
                    root: table_root_id,
                    toks: SmallVec::new_const(),
                },
                table_root,
                table_root.as_tree(),
                Some(o),
            ),
            &Leaf::Goto(l, ref o) => {
                let (tree, toks) = ext_root.resolve_label_path_always(Some(l));
                (
                    Path {
                        root: ext_root_id,
                        toks,
                    },
                    ext_root,
                    tree,
                    Some(o),
                )
            },
            &Leaf::Retry(None) => (
                Path {
                    root: table_root_id,
                    toks: SmallVec::new_const(),
                },
                table_root,
                table_root.as_tree(),
                None,
            ),
            &Leaf::Retry(Some(l)) => {
                let (table, toks) = ext_root.resolve_label_path_always(Some(l));
                (
                    Path {
                        root: ext_root_id,
                        toks,
                    },
                    ext_root,
                    table,
                    None,
                )
            },
        };

        PathDelta {
            output: output.map(|output| TreeOutput {
                output,
                conversion: conversion.clone(),
            }),
            next: [(path, PathData {
                root,
                tree,
                conversion,
            })]
            .into_iter()
            .collect(),
        }
    }
}
