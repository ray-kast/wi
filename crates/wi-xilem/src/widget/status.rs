use std::{borrow::Cow, fmt::Write};

use masonry::{
    core::{StyleProperty, WidgetMut, WidgetPod},
    theme::TEXT_COLOR,
    widgets::{Flex, Label, SizedBox},
};
use wi_core::status::{ModeKind, Status};

pub struct RenderedStatus {
    mode: &'static str,
    last_action: Cow<'static, str>,
    chord: String,
}

impl RenderedStatus {
    pub fn new(status: Status) -> Self {
        let Status {
            count,
            mode,
            last_action,
            pending_op,
        } = status;
        let mut chord = String::new();

        if let Some(count) = count {
            write!(chord, "{count}").unwrap();
        }

        let last_action = if let Some(action) = last_action {
            action.name()
        } else {
            "".into()
        };

        write!(chord, "{pending_op}").unwrap();

        let mode = match mode {
            ModeKind::Normal => "Normal",
        };

        Self {
            mode,
            last_action,
            chord,
        }
    }

    pub fn create(self) -> WidgetPod<Flex> {
        let Self {
            mode,
            last_action,
            chord,
        } = self;

        WidgetPod::new(
            Flex::row()
                .gap(8.0)
                .with_spacer(4.0)
                .with_child(Label::new(mode).with_style(StyleProperty::FontSize(18.0)))
                .with_flex_spacer(1.0)
                .with_child(
                    Label::new(last_action)
                        .with_style(StyleProperty::FontSize(18.0))
                        .with_brush(TEXT_COLOR.with_alpha(0.6)),
                )
                .with_child(
                    SizedBox::new(Label::new(chord).with_style(StyleProperty::FontSize(18.0)))
                        .width(50.0),
                )
                .with_spacer(4.0),
        )
    }

    pub fn update(self, mut bar: WidgetMut<Flex>) {
        let Self {
            mode,
            last_action,
            chord,
        } = self;

        Label::set_text(&mut Flex::child_mut(&mut bar, 1).unwrap().downcast(), mode);

        Label::set_text(
            &mut Flex::child_mut(&mut bar, 3).unwrap().downcast(),
            last_action,
        );

        Label::set_text(
            &mut SizedBox::child_mut(&mut Flex::child_mut(&mut bar, 4).unwrap().downcast())
                .unwrap()
                .downcast(),
            chord,
        );
    }
}
