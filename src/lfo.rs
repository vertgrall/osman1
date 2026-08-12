use std::f32::consts::TAU;

use crate::theme::ProcessLane;

const MIN_HZ: f32 = 0.06;
const MAX_HZ: f32 = 7.5;
/// ~500 KB/s sits in the middle of the LFO rate range.
const REF_BPS: f64 = 500_000.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Waveform {
    Sine,
    Triangle,
    Square,
    Saw,
}

impl Waveform {
    pub fn sample(self, phase: f32) -> f32 {
        match self {
            Self::Sine => phase.sin(),
            Self::Triangle => (2.0 / std::f32::consts::PI) * (phase.sin()).asin(),
            Self::Square => {
                if phase.sin() >= 0.0 {
                    1.0
                } else {
                    -1.0
                }
            }
            Self::Saw => {
                let t = (phase % std::f32::consts::TAU) / std::f32::consts::TAU;
                2.0 * t - 1.0
            }
        }
    }

    /// Chaotic multiplex — sine with deterministic jitter.
    pub fn sample_chaotic(phase: f32) -> f32 {
        let base = phase.sin();
        let jitter = ((phase * 2.71).sin() * 0.35 + (phase * 5.17).cos() * 0.22);
        (base + jitter).clamp(-1.0, 1.0)
    }

    /// Idle listen — flat baseline with sparse pulses.
    pub fn sample_idle(phase: f32) -> f32 {
        let bucket = (phase / std::f32::consts::TAU) as i32;
        if bucket % 7 == 0 && (phase % std::f32::consts::TAU) < 0.35 {
            0.85
        } else {
            0.02 * (phase * 0.2).sin()
        }
    }
}

impl ProcessLane {
    pub fn waveform(self) -> Waveform {
        match self {
            Self::Red => Waveform::Sine,
            Self::Blue => Waveform::Triangle,
            Self::Green => Waveform::Square,
        }
    }

    /// Phase offset so lanes don't lock together.
    pub fn lfo_phase_offset(self) -> f32 {
        match self {
            Self::Red => 0.0,
            Self::Blue => TAU * 0.33,
            Self::Green => TAU * 0.66,
        }
    }
}

/// Map live throughput → LFO rate in Hz.
pub fn bps_to_hz(bps: f64) -> f32 {
    if bps <= 1.0 {
        return MIN_HZ;
    }

    let log_span = (bps / REF_BPS).log10().clamp(-2.0, 2.0);
    let norm = ((log_span + 2.0) / 4.0) as f32;
    MIN_HZ + norm * (MAX_HZ - MIN_HZ)
}
