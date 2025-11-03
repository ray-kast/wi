use std::{
    hash::Hash,
    ops::{Deref, DerefMut},
};

use indexmap::IndexMap;
use smallvec::SmallVec;

type MultiMap<K, V, const N: usize> = IndexMap<K, SmallVec<[V; N]>>;

#[must_use = "Call .build() to use this builder"]
#[derive(Debug)]
pub struct TreeMapBuilder<
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
    pub(super) tokens: IndexMap<TokenName, SmallVec<[TokenDef; 1]>>,
    #[expect(clippy::type_complexity)]
    pub(super) roots: MultiMap<
        RootName,
        (
            TreeBuilder<
                TokenName,
                RootName,
                Output,
                ExtendConversion,
                TreeLabel,
                LeafExtra,
                TreeExtra,
            >,
            RootExtra,
        ),
        1,
    >,
}

impl<A, B, C, D, E, F, G, H, I> Default for TreeMapBuilder<A, B, C, D, E, F, G, H, I> {
    #[inline]
    fn default() -> Self {
        Self {
            tokens: IndexMap::new(),
            roots: IndexMap::new(),
        }
    }
}

impl<
        TokenName: Eq + Hash,
        RootName: Eq + Hash,
        Output,
        ExtendConversion,
        TokenDef,
        TreeLabel,
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
    pub fn token(&mut self, name: TokenName, def: TokenDef) -> &mut Self {
        self.tokens.entry(name).or_default().push(def);
        self
    }

    pub fn root_with_extra<
        F: FnOnce(
            &mut TreeBuilder<
                TokenName,
                RootName,
                Output,
                ExtendConversion,
                TreeLabel,
                LeafExtra,
                TreeExtra,
            >,
        ) -> &mut TreeBuilder<
            TokenName,
            RootName,
            Output,
            ExtendConversion,
            TreeLabel,
            LeafExtra,
            TreeExtra,
        >,
    >(
        &mut self,
        name: RootName,
        extra: TreeExtra,
        root_extra: RootExtra,
        with: F,
    ) -> &mut Self {
        let mut tree = TreeBuilder {
            deltas: IndexMap::new(),
            extends: SmallVec::new_const(),
            default: None,
            extra,
        };
        with(&mut tree);

        self.roots.entry(name).or_default().push((tree, root_extra));

        self
    }

    #[inline]
    pub fn root<
        F: FnOnce(
            &mut TreeBuilder<
                TokenName,
                RootName,
                Output,
                ExtendConversion,
                TreeLabel,
                LeafExtra,
                TreeExtra,
            >,
        ) -> &mut TreeBuilder<
            TokenName,
            RootName,
            Output,
            ExtendConversion,
            TreeLabel,
            LeafExtra,
            TreeExtra,
        >,
    >(
        &mut self,
        name: RootName,
        with: F,
    ) -> &mut Self
    where
        RootExtra: Default,
        TreeExtra: Default,
    {
        self.root_with_extra(name, TreeExtra::default(), RootExtra::default(), with)
    }
}

#[derive(Debug)]
pub struct TreeBuilder<
    TokenName,
    RootName,
    Output,
    ExtendConversion,
    TreeLabel,
    LeafExtra,
    TreeExtra,
> {
    #[expect(clippy::type_complexity)]
    pub(super) deltas: MultiMap<
        TokenName,
        DeltaBuilder<
            TokenName,
            RootName,
            Output,
            ExtendConversion,
            TreeLabel,
            LeafExtra,
            TreeExtra,
        >,
        1,
    >,
    pub(super) extends: SmallVec<[(RootName, Option<TreeLabel>, ExtendConversion); 1]>,
    pub(super) default: Option<LeafBuilder<Output, TreeLabel>>,
    pub(super) extra: TreeExtra,
}

#[derive(Debug)]
pub enum LeafBuilder<Output, TreeLabel> {
    Output(Output),
    Goto(TreeLabel, Option<Output>),
    Retry(Option<TreeLabel>),
}

#[derive(Debug)]
pub(super) enum DeltaBuilder<
    TokenName,
    RootName,
    Output,
    ExtendConversion,
    TreeLabel,
    LeafExtra,
    TreeExtra,
> {
    Leaf(LeafBuilder<Output, TreeLabel>, LeafExtra),
    Branch(
        BranchBuilder<
            TokenName,
            RootName,
            Output,
            ExtendConversion,
            TreeLabel,
            LeafExtra,
            TreeExtra,
        >,
    ),
}

impl<TokenName: Eq + Hash, RootName, Output, ExtendConversion, TreeLabel, LeafExtra, TreeExtra>
    TreeBuilder<TokenName, RootName, Output, ExtendConversion, TreeLabel, LeafExtra, TreeExtra>
{
    pub fn leaf_with_extra(
        &mut self,
        token: TokenName,
        leaf: LeafBuilder<Output, TreeLabel>,
        extra: LeafExtra,
    ) -> &mut Self {
        self.deltas
            .entry(token)
            .or_default()
            .push(DeltaBuilder::Leaf(leaf, extra));

        self
    }

    #[inline]
    pub fn leaf(&mut self, token: TokenName, leaf: LeafBuilder<Output, TreeLabel>) -> &mut Self
    where LeafExtra: Default {
        self.leaf_with_extra(token, leaf, LeafExtra::default())
    }

    pub fn extend(
        &mut self,
        root: RootName,
        label: Option<TreeLabel>,
        conversion: ExtendConversion,
    ) -> &mut Self {
        self.extends.push((root, label, conversion));
        self
    }

    pub fn default(&mut self, leaf: LeafBuilder<Output, TreeLabel>) -> &mut Self {
        self.default.replace(leaf);
        self
    }

    pub fn branch_with_extra<
        F: FnOnce(
            &mut BranchBuilder<
                TokenName,
                RootName,
                Output,
                ExtendConversion,
                TreeLabel,
                LeafExtra,
                TreeExtra,
            >,
        ) -> &mut BranchBuilder<
            TokenName,
            RootName,
            Output,
            ExtendConversion,
            TreeLabel,
            LeafExtra,
            TreeExtra,
        >,
    >(
        &mut self,
        token: TokenName,
        extra: TreeExtra,
        with: F,
    ) -> &mut Self {
        let mut branch = BranchBuilder {
            label: None,
            out: None,
            tree: TreeBuilder {
                deltas: IndexMap::new(),
                extends: SmallVec::new_const(),
                default: None,
                extra,
            },
        };
        with(&mut branch);

        self.deltas
            .entry(token)
            .or_default()
            .push(DeltaBuilder::Branch(branch));

        self
    }

    #[inline]
    pub fn branch<
        F: FnOnce(
            &mut BranchBuilder<
                TokenName,
                RootName,
                Output,
                ExtendConversion,
                TreeLabel,
                LeafExtra,
                TreeExtra,
            >,
        ) -> &mut BranchBuilder<
            TokenName,
            RootName,
            Output,
            ExtendConversion,
            TreeLabel,
            LeafExtra,
            TreeExtra,
        >,
    >(
        &mut self,
        token: TokenName,
        with: F,
    ) -> &mut Self
    where
        TreeExtra: Default,
    {
        self.branch_with_extra(token, TreeExtra::default(), with)
    }
}

#[derive(Debug)]
pub struct BranchBuilder<
    TokenName,
    RootName,
    Output,
    ExtendConversion,
    TreeLabel,
    LeafExtra,
    TreeExtra,
> {
    pub(super) label: Option<TreeLabel>,
    pub(super) out: Option<Output>,
    pub(super) tree:
        TreeBuilder<TokenName, RootName, Output, ExtendConversion, TreeLabel, LeafExtra, TreeExtra>,
}

impl<TokenName, RootName, Output, ExtendConversion, TreeLabel, LeafExtra, TreeExtra> Deref
    for BranchBuilder<
        TokenName,
        RootName,
        Output,
        ExtendConversion,
        TreeLabel,
        LeafExtra,
        TreeExtra,
    >
{
    type Target =
        TreeBuilder<TokenName, RootName, Output, ExtendConversion, TreeLabel, LeafExtra, TreeExtra>;

    #[inline]
    fn deref(&self) -> &Self::Target { &self.tree }
}

impl<TokenName, RootName, Output, ExtendConversion, TreeLabel, LeafExtra, TreeExtra> DerefMut
    for BranchBuilder<
        TokenName,
        RootName,
        Output,
        ExtendConversion,
        TreeLabel,
        LeafExtra,
        TreeExtra,
    >
{
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.tree }
}

impl<TokenName, RootName, Output, ExtendConversion, TreeLabel, LeafExtra, TreeExtra>
    BranchBuilder<TokenName, RootName, Output, ExtendConversion, TreeLabel, LeafExtra, TreeExtra>
{
    pub fn label(&mut self, label: TreeLabel) -> &mut Self {
        self.label.replace(label);
        self
    }

    pub fn out(&mut self, out: Output) -> &mut Self {
        self.out.replace(out);
        self
    }
}
