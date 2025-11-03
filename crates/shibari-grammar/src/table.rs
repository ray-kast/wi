use std::fmt;

use indexmap::IndexMap;

use crate::tree::TokenId;

mod flatten;

pub use flatten::*;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct StateId(u32);

impl fmt::Debug for StateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "[state #{}]", self.0) }
}

impl StateId {
    pub const START: Self = Self(0);

    #[inline]
    #[must_use]
    pub fn as_u32(self) -> u32 { self.0 }
}

#[derive(Debug)]
pub struct SparseTable<Input, Output, StateExtra>(Vec<SparseState<Input, Output, StateExtra>>);

pub struct WithTokens<'a, TokenDef, Table: ?Sized>(&'a [TokenDef], &'a Table);

impl<TokenDef: fmt::Debug, Output: fmt::Debug, StateExtra: fmt::Debug> fmt::Debug
    for WithTokens<'_, TokenDef, SparseTable<TokenId, Output, StateExtra>>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct DbgStates<'a, TokenDef, Output, StateExtra>(
            &'a [TokenDef],
            &'a [SparseState<TokenId, Output, StateExtra>],
        );

        impl<TokenDef: fmt::Debug, Output: fmt::Debug, StateExtra: fmt::Debug> fmt::Debug
            for DbgStates<'_, TokenDef, Output, StateExtra>
        {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_map()
                    .entries(self.1.iter().enumerate().map(|(i, s)| {
                        (
                            StateId(i.try_into().unwrap_or_else(|_| unreachable!())),
                            DbgState(self.0, s),
                        )
                    }))
                    .finish()
            }
        }

        struct DbgState<'a, TokenDef, Output, StateExtra>(
            &'a [TokenDef],
            &'a SparseState<TokenId, Output, StateExtra>,
        );

        impl<TokenDef: fmt::Debug, Output: fmt::Debug, StateExtra: fmt::Debug> fmt::Debug
            for DbgState<'_, TokenDef, Output, StateExtra>
        {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct("SparseState")
                    .field("extra", &self.1.extra)
                    .field("delta", &DbgDelta(self.0, &self.1.delta))
                    .field("default_delta", &self.1.default_delta)
                    .finish()
            }
        }

        struct DbgDelta<'a, TokenDef, Output>(&'a [TokenDef], &'a IndexMap<TokenId, Delta<Output>>);

        impl<TokenDef: fmt::Debug, Output: fmt::Debug> fmt::Debug for DbgDelta<'_, TokenDef, Output> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_map()
                    .entries(self.1.iter().map(|(t, d)| {
                        (
                            &self.0[usize::try_from(t.as_u32()).unwrap_or_else(|_| unreachable!())],
                            d,
                        )
                    }))
                    .finish()
            }
        }

        f.debug_tuple("SparseTable")
            .field(&DbgStates(self.0, &self.1 .0))
            .finish()
    }
}

impl<Output, StateExtra> SparseTable<TokenId, Output, StateExtra> {
    #[inline]
    pub const fn with_tokens<'a, TokenDef>(
        &'a self,
        tokens: &'a [TokenDef],
    ) -> WithTokens<'a, TokenDef, Self> {
        WithTokens(tokens, self)
    }
}

impl<Input, Output, StateExtra> SparseTable<Input, Output, StateExtra> {
    pub fn states(
        &self,
    ) -> impl Iterator<Item = (StateId, &SparseState<Input, Output, StateExtra>)> {
        self.0
            .iter()
            .enumerate()
            .map(|(i, r)| (StateId(i.try_into().unwrap_or_else(|_| unreachable!())), r))
    }

    #[inline]
    #[must_use]
    pub fn state(&self, id: StateId) -> Option<&SparseState<Input, Output, StateExtra>> {
        self.0
            .get(usize::try_from(id.0).unwrap_or_else(|_| unreachable!()))
    }

    #[expect(dead_code, reason = "Internal helper, might need later")]
    #[inline]
    #[must_use]
    pub(crate) fn state_always(&self, id: StateId) -> &SparseState<Input, Output, StateExtra> {
        self.state(id).unwrap_or_else(|| unreachable!())
    }
}

#[derive(Debug)]
pub struct SparseState<Input, Output, StateExtra> {
    delta: IndexMap<Input, Delta<Output>>,
    default_delta: Delta<Output>,
    extra: StateExtra,
}

#[derive(Debug)]
pub struct Delta<Output> {
    pub next: StateId,
    pub kind: DeltaKind<Output>,
}

#[derive(Debug)]
pub enum DeltaKind<Output> {
    Yield(Output),
    Continue,
}

impl<Input, Output, StateExtra> SparseState<Input, Output, StateExtra> {
    #[inline]
    #[must_use]
    pub fn delta(&self) -> &IndexMap<Input, Delta<Output>> { &self.delta }

    #[inline]
    #[must_use]
    pub fn default_delta(&self) -> &Delta<Output> { &self.default_delta }

    #[inline]
    #[must_use]
    pub fn extra(&self) -> &StateExtra { &self.extra }
}
