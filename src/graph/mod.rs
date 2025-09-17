mod widget;
mod view;

pub use view::*;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Port {
    node: usize,
    port: usize,
}
