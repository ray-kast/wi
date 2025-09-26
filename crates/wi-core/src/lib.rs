mod keyboard;

pub trait GraphWidget<N> {
    fn in_edges<'a>(&'a self, node: &N) -> impl IntoIterator<Item = &'a N>
    where N: 'a;

    fn out_edges<'a>(&'a self, node: &N) -> impl IntoIterator<Item = &'a N>
    where N: 'a;
}

#[must_use]
#[derive(Debug)]
pub struct GraphWidgetDriver<N> {
    focus_node: N,
    keyboard: keyboard::KeyboardHandler,
}

impl<N> GraphWidgetDriver<N> {
    pub fn new(focus_node: N) -> Self {
        Self {
            focus_node,
            keyboard: keyboard::KeyboardHandler::default(),
        }
    }

    pub fn focus_node(&self) -> &N { &self.focus_node }
}
