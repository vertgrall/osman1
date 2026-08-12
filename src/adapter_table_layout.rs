//! Shared column sizing for the Overview adapter table.
//!
//! Rate columns use fixed pixel widths so Freya never squeezes "0 B/s" vertically.

use freya::prelude::Size;

/// Minimum width for Receive / Send rate labels (regression: vertical glyph stack).
pub const MIN_RATE_LABEL_WIDTH: f32 = 72.0;
/// Sparkline track must stay wide enough to read activity.
pub const MIN_SPARKLINE_WIDTH: f32 = 160.0;
pub const SPARKLINE_HEIGHT: f32 = 56.0;
/// Hero chart canvas height on Overview.
pub const HERO_CHART_HEIGHT: f32 = 280.0;

const RATE_COL_PX: f32 = 92.0;
const TOTAL_COL_PX: f32 = 100.0;
pub const ACTIVITY_COL_PX: f32 = 300.0;
pub const ADAPTER_NAME_COL_PX: f32 = 240.0;
const CHEVRON_PX: f32 = 16.0;

/// Minimum row width so fixed columns are never pushed off-screen.
pub const ADAPTER_ROW_MIN_WIDTH: f32 =
    ADAPTER_NAME_COL_PX + ACTIVITY_COL_PX + RATE_COL_PX + RATE_COL_PX + TOTAL_COL_PX + 32.0;

/// Overview shows a fixed row count — no scroll, matches the mock.
pub const OVERVIEW_STATIC_ADAPTER_ROWS: usize = 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdapterTableMode {
    /// Overview: top N adapters, content-sized, no scroll.
    OverviewStatic,
    /// Adapters page: full list in a scroll view.
    FullList,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AdapterTableLayout;

impl AdapterTableLayout {
    pub fn adapter_name() -> Size {
        Size::px(ADAPTER_NAME_COL_PX)
    }

    pub fn activity_sparkline() -> Size {
        Size::px(ACTIVITY_COL_PX)
    }

    pub fn activity_sparkline_canvas() -> Size {
        Size::px(ACTIVITY_COL_PX)
    }

    pub fn receive_rate() -> Size {
        Size::px(RATE_COL_PX)
    }

    pub fn send_rate() -> Size {
        Size::px(RATE_COL_PX)
    }

    pub fn total_rate() -> Size {
        Size::px(TOTAL_COL_PX - CHEVRON_PX)
    }

    pub fn chevron() -> Size {
        Size::px(CHEVRON_PX)
    }

    pub fn total_cell() -> Size {
        Size::px(TOTAL_COL_PX)
    }

    /// Fixed-width columns must never use flex/percent (layout regression guard).
    pub fn fixed_column_widths() -> [f32; 4] {
        [ACTIVITY_COL_PX, RATE_COL_PX, RATE_COL_PX, TOTAL_COL_PX]
    }

    pub fn validates_fixed_columns() -> bool {
        Self::fixed_column_widths()
            .iter()
            .all(|w| *w >= MIN_RATE_LABEL_WIDTH)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_name_uses_fixed_pixel_width() {
        assert!(matches!(
            AdapterTableLayout::adapter_name(),
            Size::Pixels { .. }
        ));
    }

    #[test]
    fn rate_columns_use_pixel_widths() {
        assert!(matches!(AdapterTableLayout::receive_rate(), Size::Pixels { .. }));
        assert!(matches!(AdapterTableLayout::send_rate(), Size::Pixels { .. }));
        assert!(matches!(AdapterTableLayout::total_cell(), Size::Pixels { .. }));
        assert!(matches!(
            AdapterTableLayout::activity_sparkline(),
            Size::Pixels { .. }
        ));
    }

    #[test]
    fn fixed_columns_meet_minimum_widths() {
        assert!(AdapterTableLayout::validates_fixed_columns());
        for width in AdapterTableLayout::fixed_column_widths() {
            assert!(
                width >= MIN_RATE_LABEL_WIDTH,
                "column width {width} below minimum"
            );
        }
        assert!(AdapterTableLayout::fixed_column_widths()[0] >= MIN_SPARKLINE_WIDTH);
    }
}
