use std::{borrow::Cow, fmt::Write};

use masonry::{
    core::{StyleProperty, WidgetMut, WidgetPod},
    peniko::color::{AlphaColor, Srgb},
    theme::TEXT_COLOR,
    widgets::{Flex, Label, SizedBox},
};
use wi_core::{ModeKind, Status};

pub struct RenderedStatus {
    mode: Cow<'static, str>,
    mode_brush: AlphaColor<Srgb>,
    last_action: &'static str,
    chord: String,
}

impl RenderedStatus {
    pub fn new(status: Status) -> Self {
        let Status {
            count,
            mode,
            last_action,
            current_operator,
            pending_op,
            debug: _,
        } = status;
        let mut chord = String::new();

        if let Some(count) = count {
            write!(chord, "{count}").unwrap();
        }

        let last_action = last_action.map_or("", wi_core::ActionKind::name);

        write!(chord, "{pending_op}").unwrap();

        let mut default_mode = matches!(mode, ModeKind::Normal);
        let mut mode = Cow::Borrowed(match mode {
            ModeKind::Normal => "Normal",
        });

        if let Some(operator) = current_operator {
            mode = format!("{mode} \u{2014} {}", operator.name()).into();
            default_mode = false;
        }

        Self {
            mode,
            mode_brush: TEXT_COLOR.with_alpha(if default_mode { 0.4 } else { 1.0 }),
            last_action,
            chord,
        }
    }

    pub fn create(self) -> WidgetPod<Flex> {
        let Self {
            mode,
            mode_brush,
            last_action,
            chord,
        } = self;

        WidgetPod::new(
            Flex::row()
                .gap(8.0)
                .with_spacer(4.0)
                .with_child(
                    Label::new(mode)
                        .with_style(StyleProperty::FontSize(18.0))
                        .with_brush(mode_brush),
                )
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
            mode_brush,
            last_action,
            chord,
        } = self;

        {
            let mut lbl = Flex::child_mut(&mut bar, 1).unwrap();
            let mut lbl = lbl.downcast();
            Label::set_text(&mut lbl, mode);
            Label::set_brush(&mut lbl, mode_brush);
        }

        {
            Label::set_text(
                &mut Flex::child_mut(&mut bar, 3).unwrap().downcast(),
                last_action,
            );
        }

        Label::set_text(
            &mut SizedBox::child_mut(&mut Flex::child_mut(&mut bar, 4).unwrap().downcast())
                .unwrap()
                .downcast(),
            chord,
        );
    }
}
