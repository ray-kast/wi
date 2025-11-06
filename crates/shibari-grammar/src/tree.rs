use indexmap::IndexMap;
use smallvec::SmallVec;

mod build;
mod builder;
mod unused;

use std::{fmt, ops::Deref};

pub use build::*;
pub use builder::*;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct TokenId(u32);

impl fmt::Debug for TokenId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "[token #{}]", self.0) }
}

impl TokenId {
    #[inline]
    #[must_use]
    pub fn as_u32(self) -> u32 { self.0 }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct RootId(u32);

impl fmt::Debug for RootId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "[root #{}]", self.0) }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct LabelId(u32);

impl fmt::Debug for LabelId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "[label #{}]", self.0) }
}

pub struct TreeMap<Output, ExtendConversion, TokenDef, RootExtra, LeafExtra, TreeExtra> {
    tokens: Vec<TokenDef>,
    roots: Vec<Root<Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>>,
}

mod debug_imp {
    use super::{fmt, Branch, IndexMap, Root, RootId, TokenId, Tree, TreeDelta};

    pub struct Roots<'a, Output, ExtendConversion, TokenDef, RootExtra, LeafExtra, TreeExtra>(
        pub &'a [TokenDef],
        pub &'a [Root<Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>],
    );

    impl<
            Output: fmt::Debug,
            ExtendConversion: fmt::Debug,
            TokenDef: fmt::Debug,
            RootExtra: fmt::Debug,
            LeafExtra: fmt::Debug,
            TreeExtra: fmt::Debug,
        > fmt::Debug
        for Roots<'_, Output, ExtendConversion, TokenDef, RootExtra, LeafExtra, TreeExtra>
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_map()
                .entries(self.1.iter().enumerate().map(|(i, r)| {
                    (
                        RootId(i.try_into().unwrap_or_else(|_| unreachable!())),
                        DbgRoot(self.0, r),
                    )
                }))
                .finish()
        }
    }

    struct DbgRoot<'a, Output, ExtendConversion, TokenDef, RootExtra, LeafExtra, TreeExtra>(
        &'a [TokenDef],
        &'a Root<Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>,
    );

    impl<
            Output: fmt::Debug,
            ExtendConversion: fmt::Debug,
            TokenDef: fmt::Debug,
            RootExtra: fmt::Debug,
            LeafExtra: fmt::Debug,
            TreeExtra: fmt::Debug,
        > fmt::Debug
        for DbgRoot<'_, Output, ExtendConversion, TokenDef, RootExtra, LeafExtra, TreeExtra>
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_tuple("Root")
                .field(&self.1.extra)
                .field(&DbgTree(self.0, &self.1.tree))
                .finish()
        }
    }

    struct DbgTree<'a, Output, ExtendConversion, TokenDef, LeafExtra, TreeExtra>(
        &'a [TokenDef],
        &'a Tree<Output, ExtendConversion, LeafExtra, TreeExtra>,
    );

    impl<
            Output: fmt::Debug,
            ExtendConversion: fmt::Debug,
            TokenDef: fmt::Debug,
            LeafExtra: fmt::Debug,
            TreeExtra: fmt::Debug,
        > fmt::Debug for DbgTree<'_, Output, ExtendConversion, TokenDef, LeafExtra, TreeExtra>
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("Tree")
                .field("extra", &self.1.extra)
                .field("deltas", &DbgDeltas(self.0, &self.1.deltas))
                .field("extends", &self.1.extends)
                .field("default", &self.1.default)
                .finish()
        }
    }

    struct DbgDeltas<'a, Output, ExtendConversion, TokenDef, LeafExtra, TreeExtra>(
        &'a [TokenDef],
        &'a IndexMap<TokenId, TreeDelta<Output, ExtendConversion, LeafExtra, TreeExtra>>,
    );

    impl<
            Output: fmt::Debug,
            ExtendConversion: fmt::Debug,
            TokenDef: fmt::Debug,
            LeafExtra: fmt::Debug,
            TreeExtra: fmt::Debug,
        > fmt::Debug for DbgDeltas<'_, Output, ExtendConversion, TokenDef, LeafExtra, TreeExtra>
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_map()
                .entries(self.1.iter().map(|(t, d)| {
                    (
                        &self.0[usize::try_from(t.0).unwrap_or_else(|_| unreachable!())],
                        DbgDelta(self.0, d),
                    )
                }))
                .finish()
        }
    }

    struct DbgDelta<'a, Output, ExtendConversion, TokenDef, LeafExtra, TreeExtra>(
        &'a [TokenDef],
        &'a TreeDelta<Output, ExtendConversion, LeafExtra, TreeExtra>,
    );

    impl<
            Output: fmt::Debug,
            ExtendConversion: fmt::Debug,
            TokenDef: fmt::Debug,
            LeafExtra: fmt::Debug,
            TreeExtra: fmt::Debug,
        > fmt::Debug for DbgDelta<'_, Output, ExtendConversion, TokenDef, LeafExtra, TreeExtra>
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self.1 {
                TreeDelta::Leaf(l, x) => f.debug_tuple("Leaf").field(&l).field(&x).finish(),
                TreeDelta::Branch(b) => DbgBranch(self.0, b).fmt(f),
            }
        }
    }

    struct DbgBranch<'a, Output, ExtendConversion, TokenDef, LeafExtra, TreeExtra>(
        &'a [TokenDef],
        &'a Branch<Output, ExtendConversion, LeafExtra, TreeExtra>,
    );

    impl<
            Output: fmt::Debug,
            ExtendConversion: fmt::Debug,
            TokenDef: fmt::Debug,
            LeafExtra: fmt::Debug,
            TreeExtra: fmt::Debug,
        > fmt::Debug for DbgBranch<'_, Output, ExtendConversion, TokenDef, LeafExtra, TreeExtra>
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("Branch")
                .field("label", &self.1.label)
                .field("out", &self.1.out)
                .field("tree", &DbgTree(self.0, &self.1.tree))
                .finish()
        }
    }
}

impl<
        Output: fmt::Debug,
        ExtendConversion: fmt::Debug,
        TokenDef: fmt::Debug,
        RootExtra: fmt::Debug,
        LeafExtra: fmt::Debug,
        TreeExtra: fmt::Debug,
    > fmt::Debug for TreeMap<Output, ExtendConversion, TokenDef, RootExtra, LeafExtra, TreeExtra>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TreeMap")
            .field("tokens", &self.tokens)
            .field("roots", &debug_imp::Roots(&self.tokens, &self.roots))
            .finish()
    }
}

impl<Output, ExtendConversion, TokenDef, RootExtra, LeafExtra, TreeExtra>
    TreeMap<Output, ExtendConversion, TokenDef, RootExtra, LeafExtra, TreeExtra>
{
    pub fn builder<TokenName, RootName, TreeLabel>() -> TreeMapBuilder<
        TokenName,
        RootName,
        Output,
        ExtendConversion,
        TokenDef,
        TreeLabel,
        RootExtra,
        LeafExtra,
        TreeExtra,
    > {
        TreeMapBuilder::default()
    }

    #[inline]
    #[must_use]
    pub fn as_tokens(&self) -> &[TokenDef] { &self.tokens }

    pub fn tokens(&self) -> impl Iterator<Item = (TokenId, &TokenDef)> {
        self.tokens
            .iter()
            .enumerate()
            .map(|(i, r)| (TokenId(i.try_into().unwrap_or_else(|_| unreachable!())), r))
    }

    #[inline]
    #[must_use]
    pub fn token(&self, id: TokenId) -> Option<&TokenDef> {
        self.tokens
            .get(usize::try_from(id.0).unwrap_or_else(|_| unreachable!()))
    }

    #[inline]
    #[must_use]
    pub(crate) fn token_always(&self, id: TokenId) -> &TokenDef {
        self.tokens
            .get(usize::try_from(id.0).unwrap_or_else(|_| unreachable!()))
            .unwrap_or_else(|| unreachable!())
    }

    #[inline]
    pub fn root_ids(&self) -> impl Iterator<Item = RootId> {
        (0..self.roots.len()).map(|i| RootId(i.try_into().unwrap_or_else(|_| unreachable!())))
    }

    #[inline]
    pub fn roots(
        &self,
    ) -> impl Iterator<
        Item = (
            RootId,
            &Root<Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>,
        ),
    > {
        self.roots
            .iter()
            .enumerate()
            .map(|(i, r)| (RootId(i.try_into().unwrap_or_else(|_| unreachable!())), r))
    }

    #[inline]
    #[must_use]
    pub fn root(
        &self,
        id: RootId,
    ) -> Option<&Root<Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>> {
        self.roots
            .get(usize::try_from(id.0).unwrap_or_else(|_| unreachable!()))
    }

    #[inline]
    #[must_use]
    pub(crate) fn root_always(
        &self,
        id: RootId,
    ) -> &Root<Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra> {
        self.root(id).unwrap_or_else(|| unreachable!())
    }

    pub fn labels(&self) -> Labels<'_, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra> {
        Labels::new(&self.roots)
    }
}

#[derive(Debug)]
pub struct Root<Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra> {
    tree: Tree<Output, ExtendConversion, LeafExtra, TreeExtra>,
    extra: RootExtra,
}

pub type Resolved<'a, Output, ExtendConversion, LeafExtra, TreeExtra> = (
    &'a Tree<Output, ExtendConversion, LeafExtra, TreeExtra>,
    SmallVec<[TokenId; 2]>,
);

impl<Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>
    Root<Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>
{
    #[inline]
    #[must_use]
    pub fn extra(&self) -> &RootExtra { &self.extra }

    #[inline]
    #[must_use]
    pub fn as_tree(&self) -> &Tree<Output, ExtendConversion, LeafExtra, TreeExtra> { &self.tree }

    #[inline]
    #[must_use]
    pub fn try_resolve_label(
        &self,
        label: Option<LabelId>,
    ) -> Option<&Tree<Output, ExtendConversion, LeafExtra, TreeExtra>> {
        label.map_or(Some(self), |l| self.tree.search_label(l))
    }

    #[inline]
    #[must_use]
    pub fn resolve_label(
        &self,
        label: Option<LabelId>,
    ) -> &Tree<Output, ExtendConversion, LeafExtra, TreeExtra> {
        self.try_resolve_label(label)
            .unwrap_or_else(|| panic!("No subtree found for label"))
    }

    #[expect(dead_code, reason = "Internal helper, might need later")]
    #[inline]
    #[must_use]
    pub(crate) fn resolve_label_always(
        &self,
        label: Option<LabelId>,
    ) -> &Tree<Output, ExtendConversion, LeafExtra, TreeExtra> {
        self.try_resolve_label(label)
            .unwrap_or_else(|| unreachable!())
    }

    #[inline]
    #[must_use]
    pub fn try_resolve_label_path(
        &self,
        label: Option<LabelId>,
    ) -> Option<Resolved<'_, Output, ExtendConversion, LeafExtra, TreeExtra>> {
        label.map_or(Some((self, const { SmallVec::new_const() })), |l| {
            self.tree.search_label_path(0, l)
        })
    }

    #[inline]
    #[must_use]
    pub fn resolve_label_path(
        &self,
        label: Option<LabelId>,
    ) -> Resolved<'_, Output, ExtendConversion, LeafExtra, TreeExtra> {
        self.try_resolve_label_path(label)
            .unwrap_or_else(|| panic!("No subtree found for label"))
    }

    #[inline]
    #[must_use]
    pub(crate) fn resolve_label_path_always(
        &self,
        label: Option<LabelId>,
    ) -> Resolved<'_, Output, ExtendConversion, LeafExtra, TreeExtra> {
        self.try_resolve_label_path(label)
            .unwrap_or_else(|| unreachable!())
    }
}

impl<Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra> Deref
    for Root<Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>
{
    type Target = Tree<Output, ExtendConversion, LeafExtra, TreeExtra>;

    #[inline]
    fn deref(&self) -> &Self::Target { &self.tree }
}

#[derive(Debug)]
pub struct Tree<Output, ExtendConversion, LeafExtra, TreeExtra> {
    deltas: IndexMap<TokenId, TreeDelta<Output, ExtendConversion, LeafExtra, TreeExtra>>,
    extends: SmallVec<[(RootId, Option<LabelId>, ExtendConversion); 1]>,
    default: Leaf<Output>,
    extra: TreeExtra,
}

impl<Output, ExtendConversion, LeafExtra, TreeExtra>
    Tree<Output, ExtendConversion, LeafExtra, TreeExtra>
{
    #[inline]
    #[must_use]
    pub fn deltas(
        &self,
    ) -> &IndexMap<TokenId, TreeDelta<Output, ExtendConversion, LeafExtra, TreeExtra>> {
        &self.deltas
    }

    #[inline]
    #[must_use]
    pub fn extends(&self) -> &[(RootId, Option<LabelId>, ExtendConversion)] { &self.extends }

    #[inline]
    #[must_use]
    pub fn default(&self) -> &Leaf<Output> { &self.default }

    #[inline]
    #[must_use]
    pub fn extra(&self) -> &TreeExtra { &self.extra }

    fn search_label(&self, label: LabelId) -> Option<&Self> {
        self.deltas.values().find_map(|d| match d {
            TreeDelta::Leaf(..) => None,
            TreeDelta::Branch(Branch {
                label: Some(l),
                tree,
                ..
            }) if *l == label => Some(tree),
            TreeDelta::Branch(b) => b.tree.search_label(label),
        })
    }

    fn search_label_path(
        &self,
        prefix_len: usize,
        label: LabelId,
    ) -> Option<(&Self, SmallVec<[TokenId; 2]>)> {
        self.deltas.iter().find_map(|(&t, d)| {
            let (tree, mut path) = match d {
                TreeDelta::Leaf(..) => None,
                TreeDelta::Branch(Branch {
                    label: Some(l),
                    tree,
                    ..
                }) if *l == label => Some((tree, smallvec::smallvec![TokenId(u32::MAX);
                    prefix_len.checked_add(1).unwrap_or_else(|| unreachable!())])),
                TreeDelta::Branch(b) => b.tree.search_label_path(
                    prefix_len.checked_add(1).unwrap_or_else(|| unreachable!()),
                    label,
                ),
            }?;
            *path.get_mut(prefix_len).unwrap_or_else(|| unreachable!()) = t;
            Some((tree, path))
        })
    }
}

#[derive(Debug)]
pub enum Leaf<Output> {
    Output(Output),
    Goto(LabelId, Output),
    Retry(Option<LabelId>),
}

#[derive(Debug)]
pub enum TreeDelta<Output, ExtendConversion, LeafExtra, TreeExtra> {
    Leaf(Leaf<Output>, LeafExtra),
    Branch(Branch<Output, ExtendConversion, LeafExtra, TreeExtra>),
}

#[derive(Debug)]
pub struct Branch<Output, ExtendConversion, LeafExtra, TreeExtra> {
    label: Option<LabelId>,
    out: Output,
    tree: Tree<Output, ExtendConversion, LeafExtra, TreeExtra>,
}

impl<Output, ExtendConversion, LeafExtra, TreeExtra>
    Branch<Output, ExtendConversion, LeafExtra, TreeExtra>
{
    #[inline]
    #[must_use]
    pub fn label(&self) -> Option<LabelId> { self.label }

    #[inline]
    #[must_use]
    pub fn out(&self) -> &Output { &self.out }

    #[inline]
    #[must_use]
    pub fn as_tree(&self) -> &Tree<Output, ExtendConversion, LeafExtra, TreeExtra> { &self.tree }
}

impl<Output, ExtendConversion, LeafExtra, TreeExtra> Deref
    for Branch<Output, ExtendConversion, LeafExtra, TreeExtra>
{
    type Target = Tree<Output, ExtendConversion, LeafExtra, TreeExtra>;

    #[inline]
    fn deref(&self) -> &Self::Target { &self.tree }
}

pub use labels::{LabelInfo, Labels};

mod labels {
    use std::mem;

    use super::{LabelId, Resolved, Root, RootId, SmallVec, TokenId, Tree, TreeDelta};

    type RootIter<'a, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra> =
        std::slice::Iter<'a, Root<Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>>;

    type PathItem<'a, Output, ExtendConversion, LeafExtra, TreeExtra> = (
        Option<TokenId>,
        &'a Tree<Output, ExtendConversion, LeafExtra, TreeExtra>,
    );

    #[must_use = "This struct does nothing unless iterated"]
    #[derive(Debug)]
    pub struct Labels<'a, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra> {
        roots: std::iter::Enumerate<
            RootIter<'a, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>,
        >,

        tree_stack: SmallVec<[PathItem<'a, Output, ExtendConversion, LeafExtra, TreeExtra>; 2]>,
        tree_buf: SmallVec<[PathItem<'a, Output, ExtendConversion, LeafExtra, TreeExtra>; 1]>,
        token_path: SmallVec<[TokenId; 2]>,

        state: State<'a, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>,
    }

    #[derive(Debug)]
    enum State<'a, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra> {
        Base,
        InRoot(InRoot<'a, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>),
        InTree(
            InRoot<'a, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>,
            InTree<'a, Output, ExtendConversion, LeafExtra, TreeExtra>,
        ),
        Poison,
    }

    #[derive(Debug)]
    struct InRoot<'a, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra> {
        root_id: RootId,
        root: &'a Root<Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>,
    }

    #[derive(Debug)]
    struct InTree<'a, Output, ExtendConversion, LeafExtra, TreeExtra> {
        orig_token_path_len: usize,
        deltas: indexmap::map::Iter<
            'a,
            TokenId,
            TreeDelta<Output, ExtendConversion, LeafExtra, TreeExtra>,
        >,
    }

    pub type LabelInfo<'a, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra> = (
        RootId,
        LabelId,
        &'a Root<Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>,
        Resolved<'a, Output, ExtendConversion, LeafExtra, TreeExtra>,
    );

    impl<'a, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>
        Labels<'a, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>
    {
        pub fn new(
            roots: &'a [Root<Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>],
        ) -> Self {
            Self {
                roots: roots.iter().enumerate(),
                tree_stack: SmallVec::new_const(),
                tree_buf: SmallVec::new_const(),
                token_path: SmallVec::new_const(),
                state: State::Base,
            }
        }
    }

    impl<'a, Output: 'a, ExtendConversion: 'a, RootExtra: 'a, LeafExtra: 'a, TreeExtra: 'a> Iterator
        for Labels<'a, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>
    {
        type Item = LabelInfo<'a, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>;

        fn next(&mut self) -> Option<Self::Item> {
            Some('r#yield: loop {
                self.state = match mem::replace(&mut self.state, State::Poison) {
                    State::Base => {
                        let (i, root) = self.roots.next()?;
                        self.tree_stack.push((None, root));

                        State::InRoot(InRoot {
                            root_id: RootId(i.try_into().unwrap_or_else(|_| unreachable!())),
                            root,
                        })
                    },
                    State::InRoot(r) => {
                        if let Some((token, parent)) = self.tree_stack.pop() {
                            let orig_token_path_len = self.token_path.len();
                            self.token_path.extend(token);

                            State::InTree(r, InTree {
                                orig_token_path_len,
                                deltas: parent.deltas.iter(),
                            })
                        } else {
                            State::Base
                        }
                    },
                    State::InTree(r, mut t) => {
                        for (&token, delta) in &mut t.deltas {
                            if let TreeDelta::Branch(branch) = delta {
                                self.tree_buf.push((Some(token), branch));

                                if let Some(label) = branch.label {
                                    let InRoot { root_id, root } = r;
                                    self.state = State::InTree(r, t);
                                    break 'r#yield (
                                        root_id,
                                        label,
                                        root,
                                        (
                                            branch,
                                            self.token_path
                                                .iter()
                                                .copied()
                                                .chain([token])
                                                .collect(),
                                        ),
                                    );
                                }
                            }
                        }

                        self.token_path.truncate(t.orig_token_path_len);
                        self.tree_stack.extend(self.tree_buf.drain(..).rev());

                        State::InRoot(r)
                    },
                    State::Poison => unreachable!(),
                }
            })
        }
    }
}
