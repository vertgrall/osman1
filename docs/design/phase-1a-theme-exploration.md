# Phase 1A — Light theme exploration (waveform colors)

**Status:** Design review  
**Owner:** Aug 2026

Five light-theme directions for Receive / Send / Total waveform colors. Mocks in `resources/brand/osman-theme-mock-*.png`.

---

## Comparison

| # | Name | Receive | Send | Total | Surface vibe |
|---|------|---------|------|-------|--------------|
| **1** | **Clinical Sage** *(current)* | `#62A86C` sage | `#D97840` orange | `#9A9086` taupe | Cream taupe |
| **2** | **Ocean Pulse** | `#20B2AA` teal | `#FF7F50` coral | `#64748B` slate | Cool blue-white |
| **3** | **Sunrise Monitor** | `#E6A836` amber | `#E06070` rose | `#A89888` stone | Warm ivory |
| **4** | **Lab Violet** | `#5872C4` periwinkle | `#C45496` magenta | `#82808C` cool gray | Clinical white-lilac |
| **5** | **Forest Scope** | `#2E8B57` emerald | `#B87333` copper | `#788470` moss | Soft green tint |

All themes stay **light-only** (Phase 1.0 non-goal: dark mode).

---

## Mock files

| File | Theme |
|------|-------|
| `osman-theme-mock-01-clinical-sage.png` | Current production palette |
| `osman-theme-mock-02-ocean-pulse.png` | Cool / clinical |
| `osman-theme-mock-03-sunrise-monitor.png` | Warm / energetic |
| `osman-theme-mock-04-lab-violet.png` | Distinct / scope-like |
| `osman-theme-mock-05-forest-scope.png` | Natural / pairs with Clinical Scope icon |

---

## Signoff

| Role | Name | Status | Date |
|------|------|--------|------|
| Design | Jon | ☐ Pick direction | |
| Implementation | | ☐ After signoff | |

**Next:** Pick 1 (or mix — e.g. Forest waveforms + Clinical surfaces) → add `Palette` variant in `theme.rs` + prefs selector in 1A.2.
