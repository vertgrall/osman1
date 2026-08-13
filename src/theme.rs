use freya::prelude::{Border, Color};

/// User-selectable theme — ten light palettes plus four super-black dark modes.
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
    HyperBerry,
    VoidSignal,
    ObsidianBloom,
    StudioAsh,
    PlasmaGate,
}

impl AppTheme {
    pub const ALL: [Self; 14] = [
        Self::ClinicalSage,
        Self::OceanPulse,
        Self::SunriseMonitor,
        Self::LabViolet,
        Self::ForestScope,
        Self::CardinalScope,
        Self::PineMonitor,
        Self::SolarScope,
        Self::PrimarySignal,
        Self::HyperBerry,
        Self::VoidSignal,
        Self::ObsidianBloom,
        Self::StudioAsh,
        Self::PlasmaGate,
    ];

    pub const LIGHT: [Self; 10] = [
        Self::ClinicalSage,
        Self::OceanPulse,
        Self::SunriseMonitor,
        Self::LabViolet,
        Self::ForestScope,
        Self::CardinalScope,
        Self::PineMonitor,
        Self::SolarScope,
        Self::PrimarySignal,
        Self::HyperBerry,
    ];

    pub const DARK: [Self; 4] = [
        Self::VoidSignal,
        Self::ObsidianBloom,
        Self::StudioAsh,
        Self::PlasmaGate,
    ];

    pub fn default_theme() -> Self {
        Self::ClinicalSage
    }

    pub fn is_dark(self) -> bool {
        matches!(
            self,
            Self::VoidSignal | Self::ObsidianBloom | Self::StudioAsh | Self::PlasmaGate
        )
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
            Self::HyperBerry => "hyper_berry",
            Self::VoidSignal => "void_signal",
            Self::ObsidianBloom => "obsidian_bloom",
            Self::StudioAsh => "studio_ash",
            Self::PlasmaGate => "plasma_gate",
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
            Self::HyperBerry => "Hyper Berry",
            Self::VoidSignal => "Void Signal",
            Self::ObsidianBloom => "Obsidian Bloom",
            Self::StudioAsh => "Studio Ash",
            Self::PlasmaGate => "Plasma Gate",
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
            "hyper_berry" => Self::HyperBerry,
            "void_signal" => Self::VoidSignal,
            "obsidian_bloom" => Self::ObsidianBloom,
            "studio_ash" => Self::StudioAsh,
            "plasma_gate" => Self::PlasmaGate,
            _ => Self::ClinicalSage,
        }
    }

    pub fn chart_well_tag(self) -> Option<&'static str> {
        match self.palette().chart_well {
            ChartWell::White => Some("White scope"),
            ChartWell::Black => Some("Black scope"),
            ChartWell::Soft => None,
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
            Self::HyperBerry => Palette::hyper_berry(),
            Self::VoidSignal => Palette::void_signal(),
            Self::ObsidianBloom => Palette::obsidian_bloom(),
            Self::StudioAsh => Palette::studio_ash(),
            Self::PlasmaGate => Palette::plasma_gate(),
        }
    }
}

/// Hero/detail chart background treatment — independent of light vs dark chrome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChartWell {
    /// Theme default (soft off-white on light chrome, ink black on dark chrome).
    Soft,
    /// Pure white plot — crisp clinical paper.
    White,
    /// Ink-black plot — oscilloscope inset on light chrome.
    Black,
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
    pub is_dark: bool,
    pub chart_well: ChartWell,
    pub bg: Color,
    pub panel: Color,
    pub panel_edge: Color,
    pub text: Color,
    pub muted: Color,
    pub title: Color,
    /// Receive — inbound waveform.
    pub receive: Color,
    /// Send — outbound waveform.
    pub send: Color,
    /// Total — aggregate waveform.
    pub total: Color,
    /// Brand accent — buttons, active nav, alerts pill.
    pub accent: Color,
    pub bar_track: Color,
    pub chart_fill: (u8, u8, u8),
    pub chart_grid: (u8, u8, u8),
    pub chart_label: (u8, u8, u8),
}

impl Palette {
    pub fn light() -> Self {
        Self::clinical_sage()
    }

    pub fn chart_well_is_dark(self) -> bool {
        matches!(self.chart_well, ChartWell::Black) || self.is_dark
    }

    /// Area fill alpha for hero receive lane.
    pub fn wave_rx_fill_alpha(self) -> f32 {
        if self.chart_well_is_dark() {
            0.44
        } else {
            0.40
        }
    }

    /// Area fill alpha for hero send lane.
    pub fn wave_tx_fill_alpha(self) -> f32 {
        if self.chart_well_is_dark() {
            0.38
        } else {
            0.34
        }
    }

    /// Area fill alpha for mini spark receive lane.
    pub fn wave_spark_rx_alpha(self) -> f32 {
        if self.chart_well_is_dark() {
            0.40
        } else {
            0.32
        }
    }

    /// Area fill alpha for mini spark send lane.
    pub fn wave_spark_tx_alpha(self) -> f32 {
        if self.chart_well_is_dark() {
            0.34
        } else {
            0.26
        }
    }

    pub fn wave_total_stroke(self) -> f32 {
        if self.chart_well_is_dark() {
            2.8
        } else {
            2.2
        }
    }

    pub fn wave_spark_total_stroke(self) -> f32 {
        if self.chart_well_is_dark() {
            2.0
        } else {
            1.6
        }
    }

    pub fn wave_area_stroke(self) -> f32 {
        if self.chart_well_is_dark() {
            1.65
        } else {
            1.25
        }
    }

    pub fn wave_line_alpha(self) -> f32 {
        if self.chart_well_is_dark() {
            1.0
        } else {
            0.96
        }
    }

    pub fn wave_glow(self) -> bool {
        self.chart_well_is_dark()
    }

    fn light_palette(receive: Color, send: Color, total: Color, accent: Color) -> Self {
        Self {
            is_dark: false,
            chart_well: ChartWell::Soft,
            bg: Color::from_rgb(246, 247, 250),
            panel: Color::from_rgb(255, 255, 255),
            panel_edge: Color::from_argb(32, 22, 26, 34),
            text: Color::from_rgb(22, 26, 34),
            muted: Color::from_rgb(108, 116, 128),
            title: Color::from_rgb(14, 18, 26),
            receive,
            send,
            total,
            accent,
            bar_track: Color::from_argb(12, 22, 26, 34),
            chart_fill: (252, 252, 254),
            chart_grid: (218, 222, 230),
            chart_label: (108, 116, 128),
        }
    }

    fn dark_palette(receive: Color, send: Color, total: Color, accent: Color) -> Self {
        Self {
            is_dark: true,
            chart_well: ChartWell::Soft,
            bg: Color::from_rgb(4, 5, 8),
            panel: Color::from_rgb(11, 13, 18),
            panel_edge: Color::from_argb(48, 180, 188, 204),
            text: Color::from_rgb(236, 238, 244),
            muted: Color::from_rgb(132, 140, 158),
            title: Color::from_rgb(248, 249, 252),
            receive,
            send,
            total,
            accent,
            bar_track: Color::from_argb(28, 180, 188, 204),
            chart_fill: (8, 10, 14),
            chart_grid: (38, 42, 54),
            chart_label: (132, 140, 158),
        }
    }

    fn with_chart_well(mut self, well: ChartWell) -> Self {
        self.chart_well = well;
        match well {
            ChartWell::Soft => self,
            ChartWell::White => {
                self.chart_fill = (255, 255, 255);
                self.chart_grid = (210, 214, 222);
                self.chart_label = (96, 104, 116);
                self.bar_track = Color::from_argb(10, 22, 26, 34);
                self
            }
            ChartWell::Black => {
                self.chart_fill = (6, 8, 12);
                self.chart_grid = (54, 60, 76);
                self.chart_label = (168, 174, 188);
                if !self.is_dark {
                    self.bar_track = Color::from_argb(48, 6, 8, 12);
                }
                self
            }
        }
    }

    /// Mint receive · coral send · violet total · white scope plot.
    pub fn clinical_sage() -> Self {
        Self::light_palette(
            Color::from_rgb(0, 200, 150),
            Color::from_rgb(255, 107, 53),
            Color::from_rgb(123, 97, 255),
            Color::from_rgb(255, 107, 53),
        )
        .with_chart_well(ChartWell::White)
    }

    /// Electric cyan receive · hot pink send · aqua total · white scope plot.
    pub fn ocean_pulse() -> Self {
        Self::light_palette(
            Color::from_rgb(0, 180, 255),
            Color::from_rgb(255, 64, 129),
            Color::from_rgb(0, 229, 204),
            Color::from_rgb(0, 136, 255),
        )
        .with_chart_well(ChartWell::White)
    }

    /// Amber receive · crimson send · tangerine total.
    pub fn sunrise_monitor() -> Self {
        Self::light_palette(
            Color::from_rgb(255, 176, 32),
            Color::from_rgb(255, 51, 102),
            Color::from_rgb(255, 136, 0),
            Color::from_rgb(255, 51, 102),
        )
    }

    /// Cobalt receive · magenta send · orchid total · black scope plot.
    pub fn lab_violet() -> Self {
        Self::light_palette(
            Color::from_rgb(79, 124, 255),
            Color::from_rgb(255, 71, 168),
            Color::from_rgb(157, 78, 221),
            Color::from_rgb(255, 71, 168),
        )
        .with_chart_well(ChartWell::Black)
    }

    /// Neon green receive · blaze orange send · sky total.
    pub fn forest_scope() -> Self {
        Self::light_palette(
            Color::from_rgb(0, 255, 136),
            Color::from_rgb(255, 149, 0),
            Color::from_rgb(0, 212, 255),
            Color::from_rgb(0, 255, 136),
        )
    }

    /// Emerald receive · scarlet send · rose total.
    pub fn cardinal_scope() -> Self {
        Self::light_palette(
            Color::from_rgb(46, 204, 113),
            Color::from_rgb(230, 57, 70),
            Color::from_rgb(255, 107, 107),
            Color::from_rgb(230, 57, 70),
        )
    }

    /// Teal receive · amber send · gold total.
    pub fn pine_monitor() -> Self {
        Self::light_palette(
            Color::from_rgb(0, 212, 170),
            Color::from_rgb(255, 140, 0),
            Color::from_rgb(255, 214, 10),
            Color::from_rgb(0, 184, 148),
        )
    }

    /// Jade receive · solar send · ember total.
    pub fn solar_scope() -> Self {
        Self::light_palette(
            Color::from_rgb(82, 183, 136),
            Color::from_rgb(255, 183, 3),
            Color::from_rgb(251, 133, 0),
            Color::from_rgb(255, 183, 3),
        )
    }

    /// Traffic-light lanes — saturated green / red / yellow.
    pub fn primary_signal() -> Self {
        Self::light_palette(
            Color::from_rgb(0, 200, 83),
            Color::from_rgb(255, 23, 68),
            Color::from_rgb(255, 214, 0),
            Color::from_rgb(255, 214, 0),
        )
    }

    /// Royal blue receive · hot pink send · electric violet total · black scope plot.
    pub fn hyper_berry() -> Self {
        Self::light_palette(
            Color::from_rgb(61, 90, 254),
            Color::from_rgb(255, 0, 128),
            Color::from_rgb(170, 0, 255),
            Color::from_rgb(255, 0, 128),
        )
        .with_chart_well(ChartWell::Black)
    }

    /// Super-black · neon green / blaze orange / cyan total.
    pub fn void_signal() -> Self {
        Self::dark_palette(
            Color::from_rgb(57, 255, 20),
            Color::from_rgb(255, 69, 0),
            Color::from_rgb(0, 255, 255),
            Color::from_rgb(57, 255, 20),
        )
    }

    /// Super-black · hot magenta / electric blue / violet total.
    pub fn obsidian_bloom() -> Self {
        Self::dark_palette(
            Color::from_rgb(255, 0, 110),
            Color::from_rgb(0, 180, 255),
            Color::from_rgb(191, 90, 242),
            Color::from_rgb(255, 0, 110),
        )
    }

    /// Super-black · white receive / beige send / grey total — neutral studio waves.
    pub fn studio_ash() -> Self {
        Self::dark_palette(
            Color::from_rgb(248, 248, 244),
            Color::from_rgb(214, 196, 168),
            Color::from_rgb(156, 164, 176),
            Color::from_rgb(232, 224, 208),
        )
    }

    /// Super-black · molten gold / hot coral / ice total.
    pub fn plasma_gate() -> Self {
        Self::dark_palette(
            Color::from_rgb(255, 208, 0),
            Color::from_rgb(255, 48, 92),
            Color::from_rgb(176, 248, 255),
            Color::from_rgb(255, 196, 0),
        )
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
        } else if self.is_dark {
            Color::from_argb(18, 180, 188, 204)
        } else {
            Color::from_argb(16, 22, 26, 34)
        }
    }

    pub fn selected_bg(self) -> Color {
        let alpha = if self.is_dark { 48 } else { 36 };
        Color::from_argb(alpha, self.accent.r(), self.accent.g(), self.accent.b())
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
    fn light_and_dark_catalog_sizes() {
        assert_eq!(AppTheme::LIGHT.len(), 10);
        assert_eq!(AppTheme::DARK.len(), 4);
        assert_eq!(AppTheme::ALL.len(), 14);
    }

    #[test]
    fn light_themes_use_neutral_surfaces() {
        for theme in AppTheme::LIGHT {
            let palette = theme.palette();
            assert!(!palette.is_dark);
            assert!(palette.bg.g() > 240, "{} bg should stay neutral-light", theme.id());
            assert!(palette.receive.g() > 120 || palette.receive.b() > 120);
            assert!(palette.send.r() > 180 || palette.send.g() > 120);
        }
    }

    #[test]
    fn dark_themes_are_super_black_with_glow_waves() {
        for theme in AppTheme::DARK {
            let palette = theme.palette();
            assert!(palette.is_dark);
            assert!(palette.bg.r() < 12 && palette.bg.g() < 12 && palette.bg.b() < 16);
            assert!(palette.wave_glow());
            assert!(palette.receive.r() > 180 || palette.receive.g() > 180);
        }
    }

    #[test]
    fn popping_lane_colors_are_saturated() {
        let primary = AppTheme::PrimarySignal.palette();
        assert!(primary.receive.g() > primary.receive.r());
        assert!(primary.send.r() > primary.send.g());
        assert!(primary.accent.g() > 140);

        let void = AppTheme::VoidSignal.palette();
        assert!(void.receive.g() > 200);
        assert!(void.send.r() > 200);
    }

    #[test]
    fn studio_ash_uses_neutral_wave_lanes() {
        let ash = AppTheme::StudioAsh.palette();
        assert!(ash.is_dark);
        assert!(ash.receive.r() > 240 && ash.receive.g() > 240);
        assert!(ash.send.r() > 200 && ash.send.g() > 180 && ash.send.b() > 140);
        assert!(
            ash.total.r() > 140 && ash.total.r() < 180,
            "total lane should read as cool grey"
        );
    }

    #[test]
    fn chart_well_white_and_black_variants() {
        let white = AppTheme::ClinicalSage.palette();
        assert_eq!(white.chart_well, ChartWell::White);
        assert!(white.chart_fill.0 > 250);
        assert!(!white.chart_well_is_dark());

        let black = AppTheme::LabViolet.palette();
        assert_eq!(black.chart_well, ChartWell::Black);
        assert!(black.chart_fill.0 < 16);
        assert!(black.chart_well_is_dark());
        assert!(black.wave_glow());

        let dark_white = AppTheme::OceanPulse.palette();
        assert_eq!(dark_white.chart_well, ChartWell::White);
        assert!(dark_white.chart_fill.0 > 250);
    }
}

/// Compact rate for the menubar title (no unit suffix).
pub fn format_rate_compact(bytes_per_sec: f64) -> String {
    if bytes_per_sec >= 1_000_000.0 {
        format!("{:.1}", bytes_per_sec / 1_000_000.0)
    } else if bytes_per_sec >= 1_000.0 {
        format!("{:.1}", bytes_per_sec / 1_000.0)
    } else {
        format!("{:.0}", bytes_per_sec)
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
