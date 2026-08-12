use freya::prelude::{Border, Color};

/// Semantic traffic lanes — internal names map to Receive / Send / Total.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessLane {
    Red,
    Blue,
    Green,
}

impl ProcessLane {
    pub fn color(self, palette: Palette) -> Color {
        match self {
            Self::Red => palette.receive,
            Self::Blue => palette.send,
            Self::Green => palette.total,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Red => "Receive",
            Self::Blue => "Send",
            Self::Green => "Total",
        }
    }
}

#[derive(Clone, Copy)]
pub struct Palette {
    pub bg: Color,
    pub panel: Color,
    pub panel_edge: Color,
    pub text: Color,
    pub muted: Color,
    pub title: Color,
    /// Receive — inbound (light green).
    pub receive: Color,
    /// Send — outbound (orange).
    pub send: Color,
    /// Total — aggregate (taupe).
    pub total: Color,
    /// Brand accent — sidebar logo, active nav (warm orange).
    pub accent: Color,
    pub bar_track: Color,
    pub chart_fill: (u8, u8, u8),
    pub chart_grid: (u8, u8, u8),
    pub chart_label: (u8, u8, u8),
}

impl Palette {
    /// Warm light — cream taupe surfaces, sage receive, orange send.
    pub fn light() -> Self {
        Self {
            bg: Color::from_rgb(232, 226, 218),
            panel: Color::from_rgb(243, 239, 232),
            panel_edge: Color::from_argb(36, 61, 56, 50),
            text: Color::from_rgb(61, 56, 50),
            muted: Color::from_rgb(138, 130, 120),
            title: Color::from_rgb(45, 41, 36),
            receive: Color::from_rgb(98, 168, 108),
            send: Color::from_rgb(217, 120, 64),
            total: Color::from_rgb(154, 144, 134),
            accent: Color::from_rgb(217, 120, 64),
            bar_track: Color::from_argb(14, 61, 56, 50),
            chart_fill: (243, 239, 232),
            chart_grid: (216, 208, 198),
            chart_label: (138, 130, 120),
        }
    }

    pub fn border(self) -> Border {
        Border::new().fill(self.panel_edge).width(1.0)
    }

    pub fn elevated_border(self) -> Border {
        Border::new().fill(self.panel_edge).width(1.0)
    }

    pub fn zebra_bg(self, index: usize) -> Color {
        if index.is_multiple_of(2) {
            self.bg
        } else {
            Color::from_argb(20, self.text.r(), self.text.g(), self.text.b())
        }
    }

    pub fn selected_bg(self) -> Color {
        Color::from_argb(36, self.accent.r(), self.accent.g(), self.accent.b())
    }

    pub fn row_bg(self, index: usize, selected: bool) -> Color {
        if selected {
            self.selected_bg()
        } else {
            self.zebra_bg(index)
        }
    }
}

impl Default for Palette {
    fn default() -> Self {
        Self::light()
    }
}

pub fn format_rate(bytes_per_sec: f64) -> String {
    if bytes_per_sec >= 1_000_000.0 {
        format!("{:.1} MB/s", bytes_per_sec / 1_000_000.0)
    } else if bytes_per_sec >= 1_000.0 {
        format!("{:.1} KB/s", bytes_per_sec / 1_000.0)
    } else {
        format!("{:.0} B/s", bytes_per_sec)
    }
}

pub fn format_total(bytes: u64) -> String {
    if bytes >= 1_000_000_000 {
        format!("{:.2} GB", bytes as f64 / 1_000_000_000.0)
    } else if bytes >= 1_000_000 {
        format!("{:.1} MB", bytes as f64 / 1_000_000.0)
    } else if bytes >= 1_000 {
        format!("{:.1} KB", bytes as f64 / 1_000.0)
    } else {
        format!("{bytes} B")
    }
}
