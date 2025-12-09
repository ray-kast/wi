use ratatui::{
    buffer::Buffer,
    layout::{HorizontalAlignment, Position, Rect, Size},
    style::Style,
    text::{Span, StyledGrapheme},
    widgets::Widget,
};

pub struct Layout<'a> {
    span: &'a Span<'a>,
    style: Style,
    neg_gap: bool,
    gap: u16,
    width: u16,
}

impl<'a> Layout<'a> {
    const fn empty(span: &'a Span<'a>) -> Self {
        Self {
            span,
            style: Style::new(),
            neg_gap: false,
            gap: 0,
            width: 0,
        }
    }

    pub fn prepare(
        size: Size,
        inner_offs: Position,
        align: HorizontalAlignment,
        span: &'a Span<'a>,
        style: Style,
    ) -> Self {
        if inner_offs.y != 0 || size.width == 0 || size.height == 0 {
            return Self::empty(span);
        }

        let total_width = size.width.saturating_add(inner_offs.x);

        let width = span.width().try_into().unwrap_or(u16::MAX);
        if width == 0 {
            return Self::empty(span);
        }

        let total_gap = match align {
            HorizontalAlignment::Left => 0,
            HorizontalAlignment::Center => total_width.saturating_sub(width) / 2,
            HorizontalAlignment::Right => total_width.saturating_sub(width),
        };

        Self {
            span,
            style,
            neg_gap: inner_offs.x > total_gap,
            gap: total_gap.abs_diff(inner_offs.x),
            width,
        }
    }

    #[inline]
    pub fn width(&self) -> u16 {
        self.width
            .saturating_sub(u16::from(self.neg_gap) * self.gap)
    }
}

impl Widget for Layout<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.y < buf.area.y {
            return;
        }

        let Self {
            span,
            style,
            neg_gap,
            gap,
            width,
        } = self;

        let skip = u16::from(neg_gap) * gap;
        let gap = u16::from(!neg_gap) * gap.min(area.width);
        let area = Rect {
            x: area.x.saturating_add(gap),
            y: area.y,
            width: (area.width - gap).min(width),
            height: area.height.min(1),
        };

        let skip = skip.saturating_add(buf.area.x.saturating_sub(area.x));
        let area = area.intersection(buf.area);

        if area.is_empty() || skip >= width {
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

#[inline]
pub fn render_span(
    area: Rect,
    buf: &mut Buffer,
    inner_offs: Position,
    align: HorizontalAlignment,
    span: &Span,
    style: Style,
) {
    Layout::prepare(area.as_size(), inner_offs, align, span, style).render(area, buf);
}
