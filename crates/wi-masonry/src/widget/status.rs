use std::{borrow::Cow, fmt::Write};

use masonry::{
    core::{NewWidget, Properties, StyleProperty, WidgetMut, WidgetPod},
    peniko::color::{AlphaColor, Srgb},
    properties::{types::Length, ContentColor},
    theme::{DISABLED_TEXT_COLOR, TEXT_COLOR},
    widgets::{Flex, Label, SizedBox},
};
use wi_core::{LastOp, ModeKind, Status};

pub struct RenderedStatus {
    mode: Cow<'static, str>,
    mode_brush: AlphaColor<Srgb>,
    last_action: &'static str,
    chord: String,
    chord_brush: AlphaColor<Srgb>,
}

impl RenderedStatus {
    pub fn new(status: Status) -> Self {
        let Status {
            count,
            mode,
            last_action,
            current_operator,
            pending_op,
            last_op,
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

        if let Some((kind, pending)) = current_operator {
            mode = format!("{mode} \u{2014} {}", kind.name()).into();
            chord.push_str(&pending);
            default_mode = false;
        }

        let mode_brush = if default_mode {
            DISABLED_TEXT_COLOR
        } else {
            TEXT_COLOR
        };

        let chord_brush = if chord.is_empty() {
            let LastOp {
                count,
                chord: last_chord,
            } = last_op;

            if let Some(count) = count {
                write!(chord, "{count}{last_chord}").unwrap();
            } else {
                chord = last_chord.into_owned();
            }

            DISABLED_TEXT_COLOR
        } else {
            TEXT_COLOR
        };

        Self {
            mode,
            mode_brush,
            last_action,
            chord,
            chord_brush,
        }
    }

    pub fn create(self) -> WidgetPod<Flex> {
        let Self {
            mode,
            mode_brush,
            last_action,
            chord,
            chord_brush,
        } = self;

        WidgetPod::new(
            Flex::row()
                .with_gap(Length::px(8.0))
                .with_spacer(Length::px(4.0))
                .with_child(NewWidget::new_with_props(
                    Label::new(mode).with_style(StyleProperty::FontSize(18.0)),
                    Properties::one(ContentColor::new(mode_brush)),
                ))
                .with_flex_spacer(1.0)
                .with_child(NewWidget::new_with_props(
                    Label::new(last_action).with_style(StyleProperty::FontSize(18.0)),
                    Properties::one(ContentColor::new(DISABLED_TEXT_COLOR)),
                ))
                .with_child(NewWidget::new_with_props(
                    SizedBox::new(NewWidget::new(
                        Label::new(chord).with_style(StyleProperty::FontSize(18.0)),
                    ))
                    .width(Length::px(128.0)),
                    Properties::one(ContentColor::new(chord_brush)),
                ))
                .with_spacer(Length::px(4.0)),
        )
    }

    pub fn update(self, mut bar: WidgetMut<Flex>) {
        let Self {
            mode,
            mode_brush,
            last_action,
            chord,
            chord_brush,
        } = self;

        {
            let mut lbl = Flex::child_mut(&mut bar, 1).unwrap();
            let mut lbl = lbl.downcast();
            Label::set_text(&mut lbl, mode);
            lbl.insert_prop(ContentColor::new(mode_brush));
        }

        {
            Label::set_text(
                &mut Flex::child_mut(&mut bar, 3).unwrap().downcast(),
                last_action,
            );
        }

        {
            let mut sized_box = Flex::child_mut(&mut bar, 4).unwrap();
            let mut sized_box = sized_box.downcast();
            let mut lbl = SizedBox::child_mut(&mut sized_box).unwrap();
            let mut lbl = lbl.downcast();
            Label::set_text(&mut lbl, chord);
            lbl.insert_prop(ContentColor::new(chord_brush));
        }
    }
}
