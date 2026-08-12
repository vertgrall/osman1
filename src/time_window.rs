#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TimeWindow {
    #[default]
    Sec60,
    Min5,
    Min15,
}

impl TimeWindow {
    pub fn samples(self) -> usize {
        match self {
            Self::Sec60 => 60,
            Self::Min5 => 300,
            Self::Min15 => 900,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Sec60 => "60s",
            Self::Min5 => "5m",
            Self::Min15 => "15m",
        }
    }

    pub fn subtitle(self) -> &'static str {
        match self {
            Self::Sec60 => "Last 60 seconds",
            Self::Min5 => "Last 5 minutes",
            Self::Min15 => "Last 15 minutes",
        }
    }

    pub fn all() -> [Self; 3] {
        [Self::Sec60, Self::Min5, Self::Min15]
    }

    pub fn x_labels(self) -> [String; 3] {
        match self {
            Self::Sec60 => ["60s".into(), "30s".into(), "0s".into()],
            Self::Min5 => ["5m".into(), "2.5m".into(), "0m".into()],
            Self::Min15 => ["15m".into(), "7.5m".into(), "0m".into()],
        }
    }
}

/// Keep the last `window.samples()` points from a history buffer.
pub fn slice_history(values: &[f64], window: TimeWindow) -> Vec<f64> {
    let keep = window.samples();
    if values.len() <= keep {
        return values.to_vec();
    }
    values[values.len() - keep..].to_vec()
}
