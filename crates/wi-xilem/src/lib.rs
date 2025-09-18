mod widget;
mod view;

pub use view::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Port {
    node: usize,
    port: usize,
}
