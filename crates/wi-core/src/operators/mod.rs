mod imp {
    use enum_dispatch::enum_dispatch;

    #[enum_dispatch]
    pub trait EditorOperator {}

    #[enum_dispatch(EditorOperator)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum Operator {}
}

pub use imp::{EditorOperator, Operator};
