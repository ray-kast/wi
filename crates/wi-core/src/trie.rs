pub trait Acceptor<T>: Default + PartialEq {
    type Output;

    fn pending_op(&self) -> &'static str;

    fn accept(&mut self, input: T) -> Self::Output;
}

pub mod accept {
    use super::Acceptor;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub enum Priority {
        Accept,
        Nop,
        Reject,
    }

    pub trait HasPriority {
        fn priority(&self) -> Priority;
    }

    #[derive(Debug, Default, Clone, Copy, PartialEq, Hash)]
    pub struct Overlay<A, B> {
        pub over: B,
        pub under: A,
    }

    impl<
            A: Acceptor<T, Output: Into<B::Output>>,
            B: Acceptor<T, Output: HasPriority>,
            T: Clone,
        > Acceptor<T> for Overlay<A, B>
    {
        type Output = B::Output;

        #[inline]
        fn pending_op(&self) -> &'static str {
            if self.over == B::default() {
                self.under.pending_op()
            } else {
                self.over.pending_op()
            }
        }

        fn accept(&mut self, input: T) -> Self::Output {
            let over_out = self.over.accept(input.clone());
            let under_out = self.under.accept(input).into();

            let over_pri = over_out.priority();
            let under_pri = under_out.priority();

            if let Priority::Accept = over_pri {
                self.under = A::default();
            } else if let Priority::Accept = under_pri {
                self.over = B::default();
            }

            if over_pri <= under_pri {
                over_out
            } else {
                under_out
            }
        }
    }
}

macro_rules! trie {
    (#[$($attr:tt)*] $($rest:tt)*) => {
        trie!(@parse_attr (fallthrough: (), advance: ()) #[$($attr)*] $($rest)*);
    };

    ($vis:vis fn $($rest:tt)*) => {
        trie!(@items (fallthrough: (), advance: ()) $vis fn $($rest)*);
    };

    (@parse_attr (fallthrough: $fall:tt, advance: ()) #[advance = $adv:expr] $($rest:tt)*) => {
        trie!(@parse_attr (fallthrough: $fall, advance: ($adv)) $($rest)*);
    };

    (@parse_attr (fallthrough: (), advance: $adv:tt) #[fallthrough = $fall:expr] $($rest:tt)*) => {
        trie!(@parse_attr (fallthrough: ($fall), advance: $adv) $($rest)*);
    };

    (@parse_attr $attrs:tt $($rest:tt)*) => {
        trie!(@items $attrs $($rest)*);
    };

    (
        @items (fallthrough: ($($fall:expr)?), advance: ($($adv:expr)?))
        $vis:vis fn $ty:ident($in_id:ident: $inp:ty) -> $out:ty { $($body:tt)* }
        $($($trail:tt)+)?
    ) => {
        trie!(@enum_def ($vis $ty) false {} { $($body)* });

        impl $crate::trie::Acceptor<$inp> for $ty {
            type Output = $out;

            fn pending_op(&self) -> &'static str {
                trie!(@op (self) false {} ("") { $($body)* })
            }

            fn accept(&mut self, $in_id: $inp) -> $out {
                trie!(@accept ($ty self $in_id ($($fall)?, $($adv)?)) {} Start () { $($body)* })
            }
        }

        $(trie!($($trail)+);)?
    };

    (@enum_def $args:tt $any:ident $body:tt { . => $($rest:tt)+ } $($vars:tt)*) => {
        trie!(@enum_def $args $any $body { _ => $($rest)+ } $($vars)*);
    };

    (@enum_def $args:tt $any:ident $body:tt {
        $bind:pat => continue
        $(, $($rest:tt)*)?
    } $($vars:tt)*) => {
        trie!(@enum_def $args true $body { $($($rest)*)? } $($vars)*);
    };

    (@enum_def $args:tt $any:ident { $($body:tt)* } {
        $bind:pat => yield $($out:expr)?
        $(, $($rest:tt)*)?
    } $($vars:tt)*) => {
        trie!(@enum_def $args true { $($body)* } { $($($rest)*)? } $($vars)*);
    };

    (@enum_def $args:tt $any:ident { $($body:tt)* } {
        $bind:pat => $var:ident $(($out:expr))? { .. }
        $(, $($rest:tt)*)?
    } $($vars:tt)*) => {
        trie!(@enum_def $args true { $($body)* } { $($($rest)*)? } $($vars)*);
    };

    (@enum_def $args:tt $any:ident { $($body:tt)* } {
        $bind:pat => $var:ident $(($out:expr))? @ $op:literal { $($state:tt)* }
        $(, $($rest:tt)*)?
    } $($vars:tt)*) => {
        trie!(@enum_def $args true {
            $($body)*
            $var,
        } { $($($rest)*)? } { $($state)* } $($vars)*);
    };

    (@enum_def $args:tt $any:ident $body:tt {} $($vars:tt)*) => {
        trie!(@enum_def $args $any $body $($vars)*);
    };

    (@enum_def ($vis:vis $ty:ident) false {}) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        $vis enum $ty {}
    };

    (@enum_def ($vis:vis $ty:ident) true { $($body:tt)* }) => {
        #[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
        $vis enum $ty { #[default] Start, $($body)* }
    };

    (@op $args:tt $any:ident $body:tt $cur_op:tt { . => $($rest:tt)+ } $($vars:tt)*) => {
        trie!(@op $args $any $body $cur_op { _ => $($rest)+ } $($vars)*)
    };

    (@op $args:tt $any:ident $body:tt $cur_op:tt {
        $bind:pat => continue
        $(, $($rest:tt)*)?
    } $($vars:tt)*) => {
        trie!(@op $args true $body $cur_op { $($($rest)*)? } $($vars)*)
    };

    (@op $args:tt $any:ident $body:tt $cur_op:tt {
        $bind:pat => yield $($out:expr)?
        $(, $($rest:tt)*)?
    } $($vars:tt)*) => {
        trie!(@op $args true $body $cur_op { $($($rest)*)? } $($vars)*)
    };

    (@op $args:tt $any:ident $body:tt $cur_op:tt {
        $bind:pat => $var:ident $(($out:expr))? { .. }
        $(, $($rest:tt)*)?
    } $($vars:tt)*) => {
        trie!(@op $args true $body $cur_op { $($($rest)*)? } $($vars)*)
    };

    (@op $args:tt $any:ident { $($body:tt)* } ($($cur_op:expr),*) {
        $bind:pat => $var:ident $(($out:expr))? @ $op:literal { $($state:tt)* }
        $(, $($rest:tt)*)?
    } $($vars:tt)*) => {
        trie!(@op $args true {
            $($body)*
            Self::$var => concat!($($cur_op,)* $op),
        } ($($cur_op),*) { $($($rest)*)? } ($($cur_op,)* $op) { $($state)* } $($vars)*)
    };

    (@op $args:tt $any:ident $body:tt $cur_op:tt {} $($vars:tt)*) => {
        trie!(@op $args $any $body $($vars)*)
    };

    (@op ($self:ident) false {}) => { match *$self {} };

    (@op ($self:ident) true { $($body:tt)* }) => {
        match *$self { Self::Start => "", $($body)* }
    };

    (@accept $args:tt $body:tt $var:ident ($cur_bind:pat) { . => $($rest:tt)+ } $($vars:tt)*) => {
        trie!(@accept $args $body $var ($cur_bind) { $cur_bind => $($rest)+ } $($vars)*)
    };

    (@accept $args:tt { $($body:tt)* } $var:ident ($cur_bind:pat) {
        $bind:pat => continue
        $(, $($rest:tt)*)?
    } $($vars:tt)*) => {
        trie!(@accept $args {
            $($body)*
            (Self::$var, $bind) => trie!(@accept_continue $args),
        } $var $cur_bind { $($($rest)*)? } $($vars)*)
    };

    (@accept $args:tt { $($body:tt)* } $var:ident $cur_bind:tt {
        $bind:pat => yield $($out:expr)?
        $(, $($rest:tt)*)?
    } $($vars:tt)*) => {
        trie!(@accept $args {
            $($body)*
            (Self::$var, $bind) => (Self::Start, trie!(@accept_out $args $var true $($out)?)),
        } $var $cur_bind { $($($rest)*)? } $($vars)*)
    };

    (@accept $args:tt { $($body:tt)* } $var:ident $cur_bind:tt {
        $bind:pat => $next:ident $(($out:expr))? { .. }
        $(, $($rest:tt)*)?
    } $($vars:tt)*) => {
        trie!(@accept $args {
            $($body)*
            (Self::$var, $bind) => (Self::$next, trie!(@accept_out $args $var false $($out)?)),
        } $var $cur_bind { $($($rest)*)? } $($vars)*)
    };

    (@accept $args:tt { $($body:tt)* } $var:ident $cur_bind:tt {
        $bind:pat => $next:ident $(($out:expr))? @ $op:literal { $($state:tt)* }
        $(, $($rest:tt)*)?
    } $($vars:tt)*) => {
        trie!(@accept $args {
            $($body)*
            (Self::$var, $bind) => (Self::$next, trie!(@accept_out $args $var false $($out)?)),
        } $var $cur_bind { $($($rest)*)? } $next ($bind) { $($state)* } $($vars)*)
    };

    (@accept $args:tt $body:tt $var:ident $cur_bind:tt {} $($vars:tt)*) => {
        trie!(@accept $args $body $($vars)*)
    };

    (@accept_out ($ty:ident $self:ident $inp:ident ($(fall:expr)?, $($adv:expr)?)) $var:ident false) => {
        trie!(@accept_out _ $($adv)?)
    };
    (@accept_out ($ty:ident $self:ident $inp:ident ($($fall:expr)?, $($adv:expr)?)) Start true) => {
        trie!(@accept_out _ $($fall)?)
    };
    (@accept_out $args:tt $var:ident $leaf:ident $($rest:tt)*) => {
        trie!(@accept_out _ $($rest)*)
    };

    (@accept_out _) => { Self::Output::default() };
    (@accept_out _ $out:expr) => { Self::Output::from($out) };

    (@accept_continue ($ty:ident $self:ident $inp:ident $adv:tt)) => {
        { *$self = Self::Start; continue }
    };

    (@accept ($ty:ident $self:ident $inp:ident $adv:tt) {}) => { match (*$self, $inp) {} };

    (@accept ($ty:ident $self:ident $inp:ident $adv:tt) { $($body:tt)+ }) => {
        loop {
            let (next, out) = match (*$self, $inp) { $($body)* };
            *$self = next;
            break out;
        }
    };
}

pub(crate) use trie;
