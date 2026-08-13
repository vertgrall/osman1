use freya::prelude::{Border, Color};

/// User-selectable light theme (waveform + surface palette).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppTheme {
    ClinicalSage,
    OceanPulse,
    SunriseMonitor,
    LabViolet,
    ForestScope,
    CardinalScope,
    PineMonitor,
    SolarScope,
    PrimarySignal,
}

impl AppTheme {
    pub const ALL: [Self; 9] = [
        Self::ClinicalSage,
        Self::OceanPulse,
        Self::SunriseMonitor,
        Self::LabViolet,
        Self::ForestScope,
        Self::CardinalScope,
        Self::PineMonitor,
        Self::SolarScope,
        Self::PrimarySignal,
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
            Self::CardinalScope => "cardinal_scope",
            Self::PineMonitor => "pine_monitor",
            Self::SolarScope => "solar_scope",
            Self::PrimarySignal => "primary_signal",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::ClinicalSage => "Clinical Sage",
            Self::OceanPulse => "Ocean Pulse",
            Self::SunriseMonitor => "Sunrise Monitor",
            Self::LabViolet => "Lab Violet",
            Self::ForestScope => "Forest Scope",
            Self::CardinalScope => "Cardinal Scope",
            Self::PineMonitor => "Pine Monitor",
            Self::SolarScope => "Solar Scope",
            Self::PrimarySignal => "Primary Signal",
        }
    }

    pub fn from_id(id: &str) -> Self {
        match id {
            "ocean_pulse" => Self::OceanPulse,
            "sunrise_monitor" => Self::SunriseMonitor,
            "lab_violet" => Self::LabViolet,
            "forest_scope" => Self::ForestScope,
            "cardinal_scope" => Self::CardinalScope,
            "pine_monitor" => Self::PineMonitor,
            "solar_scope" => Self::SolarScope,
            "primary_signal" => Self::PrimarySignal,
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
            Self::CardinalScope => Palette::cardinal_scope(),
            Self::PineMonitor => Palette::pine_monitor(),
            Self::SolarScope => Palette::solar_scope(),
            Self::PrimarySignal => Palette::primary_signal(),
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

    /// Light rose surfaces — crimson accent and warm red send lane.
    pub fn cardinal_scope() -> Self {
        Self {
            bg: Color::from_rgb(248, 236, 236),
            panel: Color::from_rgb(252, 244, 244),
            panel_edge: Color::from_argb(36, 72, 42, 44),
            text: Color::from_rgb(72, 42, 44),
            muted: Color::from_rgb(148, 118, 120),
            title: Color::from_rgb(56, 28, 32),
            receive: Color::from_rgb(72, 148, 118),
            send: Color::from_rgb(196, 58, 64),
            total: Color::from_rgb(148, 118, 120),
            accent: Color::from_rgb(180, 40, 52),
            bar_track: Color::from_argb(14, 72, 42, 44),
            chart_fill: (252, 244, 244),
            chart_grid: (228, 204, 204),
            chart_label: (148, 118, 120),
        }
    }

    /// Cool mint surfaces — hunter green accent and deep green receive.
    pub fn pine_monitor() -> Self {
        Self {
            bg: Color::from_rgb(228, 236, 230),
            panel: Color::from_rgb(240, 246, 242),
            panel_edge: Color::from_argb(36, 32, 58, 42),
            text: Color::from_rgb(32, 58, 42),
            muted: Color::from_rgb(108, 132, 116),
            title: Color::from_rgb(22, 48, 34),
            receive: Color::from_rgb(34, 110, 68),
            send: Color::from_rgb(196, 132, 52),
            total: Color::from_rgb(108, 132, 116),
            accent: Color::from_rgb(24, 88, 54),
            bar_track: Color::from_argb(14, 32, 58, 42),
            chart_fill: (240, 246, 242),
            chart_grid: (196, 214, 200),
            chart_label: (108, 132, 116),
        }
    }

    /// Warm ivory surfaces — golden yellow accent and amber send.
    pub fn solar_scope() -> Self {
        Self {
            bg: Color::from_rgb(252, 246, 228),
            panel: Color::from_rgb(255, 251, 238),
            panel_edge: Color::from_argb(36, 72, 58, 28),
            text: Color::from_rgb(72, 58, 28),
            muted: Color::from_rgb(156, 140, 104),
            title: Color::from_rgb(56, 44, 20),
            receive: Color::from_rgb(88, 156, 88),
            send: Color::from_rgb(224, 156, 48),
            total: Color::from_rgb(156, 140, 104),
            accent: Color::from_rgb(218, 168, 32),
            bar_track: Color::from_argb(14, 72, 58, 28),
            chart_fill: (255, 251, 238),
            chart_grid: (236, 224, 196),
            chart_label: (156, 140, 104),
        }
    }

    /// Neutral light base — receive green, send red, yellow accent (traffic-light lanes).
    pub fn primary_signal() -> Self {
        Self {
            bg: Color::from_rgb(242, 242, 238),
            panel: Color::from_rgb(250, 250, 246),
            panel_edge: Color::from_argb(36, 52, 52, 48),
            text: Color::from_rgb(52, 52, 48),
            muted: Color::from_rgb(128, 128, 120),
            title: Color::from_rgb(36, 36, 32),
            receive: Color::from_rgb(34, 120, 62),
            send: Color::from_rgb(196, 52, 52),
            total: Color::from_rgb(128, 128, 120),
            accent: Color::from_rgb(228, 184, 32),
            bar_track: Color::from_argb(14, 52, 52, 48),
            chart_fill: (250, 250, 246),
            chart_grid: (216, 216, 208),
            chart_label: (128, 128, 120),
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

    #[test]
    fn all_themes_have_distinct_accents() {
        let accents: Vec<_> = AppTheme::ALL.iter().map(|t| t.palette().accent).collect();
        for (i, a) in accents.iter().enumerate() {
            for (j, b) in accents.iter().enumerate() {
                if i != j {
                    assert_ne!(
                        (a.r(), a.g(), a.b()),
                        (b.r(), b.g(), b.b()),
                        "themes {i} and {j} share the same accent"
                    );
                }
            }
        }
    }

    #[test]
    fn new_color_themes_use_requested_hues() {
        let cardinal = AppTheme::CardinalScope.palette();
        assert!(cardinal.accent.r() > cardinal.accent.g());
        assert!(cardinal.accent.r() > cardinal.accent.b());

        let pine = AppTheme::PineMonitor.palette();
        assert!(pine.accent.g() > pine.accent.r());
        assert!(pine.accent.g() > pine.accent.b());

        let solar = AppTheme::SolarScope.palette();
        assert!(solar.accent.r() > 180 && solar.accent.g() > 140);

        let primary = AppTheme::PrimarySignal.palette();
        assert!(primary.receive.g() > primary.receive.r());
        assert!(primary.send.r() > primary.send.g());
        assert!(primary.accent.g() > 140 && primary.accent.r() > 180);
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
