pub trait Acceptor<T> {
    type Output;

    fn pending_op(&self) -> &'static str;

    fn accept(&mut self, input: T) -> Self::Output;
}

macro_rules! trie {
    (
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
                trie!(@accept ($ty self $in_id) {} Start () { $($body)* })
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
            (Self::$var, $bind) => (Self::Start, trie!(@accept_out $($out)?)),
        } $var $cur_bind { $($($rest)*)? } $($vars)*)
    };

    (@accept $args:tt { $($body:tt)* } $var:ident $cur_bind:tt {
        $bind:pat => $next:ident $(($out:expr))? @ $op:literal { $($state:tt)* }
        $(, $($rest:tt)*)?
    } $($vars:tt)*) => {
        trie!(@accept $args {
            $($body)*
            (Self::$var, $bind) => (Self::$next, trie!(@accept_out $($out)?)),
        } $var $cur_bind { $($($rest)*)? } $next ($bind) { $($state)* } $($vars)*)
    };

    (@accept $args:tt $body:tt $var:ident $cur_bind:tt {} $($vars:tt)*) => {
        trie!(@accept $args $body $($vars)*)
    };

    (@accept_out) => { Self::Output::default() };
    (@accept_out _) => { Self::Output::default() };
    (@accept_out $out:expr) => { Self::Output::from($out) };

    (@accept_continue ($ty:ident $self:ident $inp:ident)) => {
        { *$self = Self::Start; continue }
    };

    (@accept ($ty:ident $self:ident $inp:ident) {}) => { match (*$self, $inp) {} };

    (@accept ($ty:ident $self:ident $inp:ident) { $($body:tt)+ }) => {
        loop {
            let (next, out) = match (*$self, $inp) { $($body)* };
            *$self = next;
            break out;
        }
    };
}

pub(crate) use trie;
