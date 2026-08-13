use freya::prelude::{Border, Color};

/// User-selectable light theme (waveform + surface palette).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppTheme {
    ClinicalSage,
    OceanPulse,
    SunriseMonitor,
    LabViolet,
    ForestScope,
}

impl AppTheme {
    pub const ALL: [Self; 5] = [
        Self::ClinicalSage,
        Self::OceanPulse,
        Self::SunriseMonitor,
        Self::LabViolet,
        Self::ForestScope,
    ];

    pub fn default_theme() -> Self {
        Self::ClinicalSage
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::ClinicalSage => "clinical_sage",
            Self::OceanPulse => "ocean_pulse",
            Self::SunriseMonitor => "sunrise_monitor",
            Self::LabViolet => "lab_violet",
            Self::ForestScope => "forest_scope",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::ClinicalSage => "Clinical Sage",
            Self::OceanPulse => "Ocean Pulse",
            Self::SunriseMonitor => "Sunrise Monitor",
            Self::LabViolet => "Lab Violet",
            Self::ForestScope => "Forest Scope",
        }
    }

    pub fn from_id(id: &str) -> Self {
        match id {
            "ocean_pulse" => Self::OceanPulse,
            "sunrise_monitor" => Self::SunriseMonitor,
            "lab_violet" => Self::LabViolet,
            "forest_scope" => Self::ForestScope,
            _ => Self::ClinicalSage,
        }
    }

    pub fn palette(self) -> Palette {
        match self {
            Self::ClinicalSage => Palette::clinical_sage(),
            Self::OceanPulse => Palette::ocean_pulse(),
            Self::SunriseMonitor => Palette::sunrise_monitor(),
            Self::LabViolet => Palette::lab_violet(),
            Self::ForestScope => Palette::forest_scope(),
        }
    }
}

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
        Self::clinical_sage()
    }

    pub fn clinical_sage() -> Self {
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

    pub fn ocean_pulse() -> Self {
        Self {
            bg: Color::from_rgb(235, 240, 248),
            panel: Color::from_rgb(248, 250, 252),
            panel_edge: Color::from_argb(36, 51, 65, 85),
            text: Color::from_rgb(51, 65, 85),
            muted: Color::from_rgb(100, 116, 139),
            title: Color::from_rgb(30, 41, 59),
            receive: Color::from_rgb(32, 178, 170),
            send: Color::from_rgb(255, 127, 80),
            total: Color::from_rgb(100, 116, 139),
            accent: Color::from_rgb(255, 127, 80),
            bar_track: Color::from_argb(14, 51, 65, 85),
            chart_fill: (248, 250, 252),
            chart_grid: (203, 213, 225),
            chart_label: (100, 116, 139),
        }
    }

    pub fn sunrise_monitor() -> Self {
        Self {
            bg: Color::from_rgb(248, 242, 232),
            panel: Color::from_rgb(252, 248, 240),
            panel_edge: Color::from_argb(36, 68, 58, 48),
            text: Color::from_rgb(68, 58, 48),
            muted: Color::from_rgb(168, 152, 136),
            title: Color::from_rgb(52, 44, 36),
            receive: Color::from_rgb(230, 168, 54),
            send: Color::from_rgb(224, 96, 112),
            total: Color::from_rgb(168, 152, 136),
            accent: Color::from_rgb(224, 96, 112),
            bar_track: Color::from_argb(14, 68, 58, 48),
            chart_fill: (252, 248, 240),
            chart_grid: (228, 216, 200),
            chart_label: (168, 152, 136),
        }
    }

    pub fn lab_violet() -> Self {
        Self {
            bg: Color::from_rgb(240, 238, 248),
            panel: Color::from_rgb(250, 248, 252),
            panel_edge: Color::from_argb(36, 55, 52, 68),
            text: Color::from_rgb(55, 52, 68),
            muted: Color::from_rgb(130, 128, 140),
            title: Color::from_rgb(42, 40, 54),
            receive: Color::from_rgb(88, 114, 196),
            send: Color::from_rgb(196, 84, 150),
            total: Color::from_rgb(130, 128, 140),
            accent: Color::from_rgb(196, 84, 150),
            bar_track: Color::from_argb(14, 55, 52, 68),
            chart_fill: (250, 248, 252),
            chart_grid: (216, 212, 228),
            chart_label: (130, 128, 140),
        }
    }

    pub fn forest_scope() -> Self {
        Self {
            bg: Color::from_rgb(232, 238, 228),
            panel: Color::from_rgb(244, 246, 240),
            panel_edge: Color::from_argb(36, 48, 56, 44),
            text: Color::from_rgb(48, 56, 44),
            muted: Color::from_rgb(120, 132, 112),
            title: Color::from_rgb(36, 44, 32),
            receive: Color::from_rgb(46, 139, 87),
            send: Color::from_rgb(184, 115, 51),
            total: Color::from_rgb(120, 132, 112),
            accent: Color::from_rgb(184, 115, 51),
            bar_track: Color::from_argb(14, 48, 56, 44),
            chart_fill: (244, 246, 240),
            chart_grid: (204, 212, 196),
            chart_label: (120, 132, 112),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_theme_is_clinical_sage() {
        assert_eq!(AppTheme::default_theme(), AppTheme::ClinicalSage);
        assert_eq!(AppTheme::default_theme().id(), "clinical_sage");
    }

    #[test]
    fn theme_id_round_trip() {
        for theme in AppTheme::ALL {
            assert_eq!(AppTheme::from_id(theme.id()), theme);
        }
        assert_eq!(
            AppTheme::from_id("unknown"),
            AppTheme::ClinicalSage,
            "unknown ids fall back to default"
        );
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
