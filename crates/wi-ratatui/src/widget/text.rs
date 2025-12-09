use ratatui::{
    buffer::Buffer,
    layout::{HorizontalAlignment, Rect},
    style::Style,
    text::{Span, StyledGrapheme},
    widgets::Widget,
};

use crate::vector::SignedRect;

pub struct Layout<'a> {
    span: &'a Span<'a>,
    style: Style,
    x: i32,
    y: u16,
    width: u16,
}

impl<'a> Layout<'a> {
    const fn empty(span: &'a Span<'a>) -> Self {
        Self {
            span,
            style: Style::new(),
            x: 0,
            y: 0,
            width: 0,
        }
    }

    pub fn prepare(
        area: SignedRect,
        align: HorizontalAlignment,
        span: &'a Span<'a>,
        style: Style,
    ) -> Self {
        if area.is_empty() {
            return Self::empty(span);
        }

        let Ok(y) = u16::try_from(area.y) else {
            return Self::empty(span);
        };

        let span_width = span.width().try_into().unwrap_or(u32::MAX);
        if span_width == 0 {
            return Self::empty(span);
        }

        let x = area.x.saturating_add_unsigned(match align {
            HorizontalAlignment::Left => 0,
            HorizontalAlignment::Center => area.width.saturating_sub(span_width) / 2,
            HorizontalAlignment::Right => area.width.saturating_sub(span_width),
        });

        Self {
            span,
            style,
            x,
            y,
            width: span_width.max(area.width).try_into().unwrap_or(u16::MAX),
        }
    }
}

impl Widget for Layout<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let area = area.intersection(buf.area);

        if area.is_empty() {
            return;
        }

        let Self {
            span,
            style,
            x,
            y,
            width,
        } = self;

        let skip = u16::from(x < 0) * x.saturating_neg().try_into().unwrap_or(u16::MAX);
        let rect = Rect {
            x: u16::from(x >= 0) * x.try_into().unwrap_or(u16::MAX),
            y,
            width: width.saturating_sub(skip),
            height: 1,
        };
        let skip = skip.saturating_add(area.x.saturating_sub(rect.x));
        let area = area.intersection(rect);

        if area.is_empty() {
            return;
        }

        for (pos, StyledGrapheme { symbol, style }) in area
            .positions()
            .zip(span.styled_graphemes(style).skip(skip.into()))
        {
            buf[pos].set_style(style).set_symbol(symbol);
        }
    }
}
