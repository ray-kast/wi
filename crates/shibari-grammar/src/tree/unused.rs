use hashbrown::HashSet;

use super::{LabelId, LabelInfo, Leaf, RootId, TokenId, Tree, TreeDelta, TreeMap};

impl<Output, ExtendConversion, TokenDef, RootExtra, LeafExtra, TreeExtra>
    TreeMap<Output, ExtendConversion, TokenDef, RootExtra, LeafExtra, TreeExtra>
{
    pub fn unused_tokens(&self) -> impl Iterator<Item = (TokenId, &TokenDef)> {
        let used = self.roots.iter().fold(HashSet::new(), |mut u, r| {
            r.populate_used_tokens(&mut u);
            u
        });

        self.tokens().filter(move |(i, _)| !used.contains(i))
    }

    pub fn unused_labels(
        &self,
    ) -> impl Iterator<Item = LabelInfo<'_, Output, ExtendConversion, RootExtra, LeafExtra, TreeExtra>>
    {
        let used = self.roots().fold(HashSet::new(), |mut u, (i, r)| {
            r.populate_used_labels(i, &mut u);
            u
        });

        self.labels()
            .filter(move |&(r, l, ..)| !used.contains(&(r, l)))
    }
}

impl<Output, ExtendConversion, LeafExtra, TreeExtra>
    Tree<Output, ExtendConversion, LeafExtra, TreeExtra>
{
    fn populate_used_tokens(&self, used: &mut HashSet<TokenId>) {
        for (&token, delta) in &self.deltas {
            used.insert(token);

            if let TreeDelta::Branch(b) = delta {
                b.populate_used_tokens(used);
            }
        }
    }

    fn populate_used_labels(&self, root: RootId, used: &mut HashSet<(RootId, LabelId)>) {
        for delta in self.deltas.values() {
            match delta {
                TreeDelta::Leaf(l, _) => {
                    l.populate_used_labels(root, used);
                },
                TreeDelta::Branch(b) => {
                    b.populate_used_labels(root, used);
                },
            }
        }

        self.default.populate_used_labels(root, used);
    }
}

impl<Output> Leaf<Output> {
    fn populate_used_labels(&self, root: RootId, used: &mut HashSet<(RootId, LabelId)>) {
        match self {
            &Self::Goto(l, _) | &Self::Retry(Some(l)) => {
                used.insert((root, l));
            },
            Self::Output(_) | Self::Retry(None) => (),
        }
    }
}
