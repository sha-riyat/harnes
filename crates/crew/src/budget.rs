//! How much model each step of a run gets. The orchestrator picks the run's
//! mode; the weekly pace can lighten it so the week's allowance lasts until
//! its reset. Opus is kept for the steps where a miss costs the most.

use crate::Role;
use serde::{Deserialize, Serialize};

/// How heavy a run is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// One worker, no planner, no separate Teacher: the reviewer checks the task.
    Light,
    #[default]
    Full,
    /// Several reviewers look independently; a verifier keeps what they agree on.
    Deep,
}

/// Model and effort for one agent, in place of its definition's defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tune {
    pub model: &'static str,
    pub effort: &'static str,
}

pub const SONNET_MEDIUM: Tune = Tune {
    model: "sonnet",
    effort: "medium",
};
pub const SONNET_HIGH: Tune = Tune {
    model: "sonnet",
    effort: "high",
};

/// Reviewers that look for problems in a deep review, before the verifier.
pub const FINDERS: u32 = 3;

/// The override for `role`, if any. `eco` is set when the week runs ahead of pace.
pub fn tune(mode: Mode, eco: bool, role: Role) -> Option<Tune> {
    match (mode, role) {
        (Mode::Light, Role::Worker) => Some(SONNET_MEDIUM),
        (Mode::Light, Role::Reviewer) => Some(SONNET_HIGH),
        (_, Role::Planner | Role::Teacher | Role::Reviewer) if eco => Some(SONNET_HIGH),
        _ => None,
    }
}

/// The mode a run actually gets: an eco week has no deep reviews.
pub fn effective(mode: Mode, eco: bool) -> Mode {
    if eco && mode == Mode::Deep {
        Mode::Full
    } else {
        mode
    }
}

/// Where the account stands, from the limits Anthropic reports (0 to 1 used).
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Pace {
    pub week: f64,
    /// Share of the 7-day window gone, 0 to 1.
    pub week_elapsed: f64,
    pub five_hour: f64,
}

impl Pace {
    /// From the window's use and its reset time (epoch seconds).
    pub fn new(week: f64, week_reset: u64, five_hour: f64, now: u64) -> Pace {
        let left = week_reset.saturating_sub(now) as f64 / (7.0 * 86_400.0);
        Pace {
            week,
            week_elapsed: (1.0 - left).clamp(0.0, 1.0),
            five_hour,
        }
    }

    /// The week runs ahead of a straight line to its reset, or the 5-hour window is nearly spent.
    pub fn eco(&self) -> bool {
        self.week > self.week_elapsed + 0.05 || self.five_hour > 0.85
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_runs_and_eco_weeks_spare_opus() {
        assert_eq!(tune(Mode::Full, false, Role::Reviewer), None);
        assert_eq!(tune(Mode::Light, false, Role::Reviewer), Some(SONNET_HIGH));
        assert_eq!(tune(Mode::Full, true, Role::Teacher), Some(SONNET_HIGH));
        assert_eq!(tune(Mode::Full, true, Role::Worker), None);
        assert_eq!(effective(Mode::Deep, true), Mode::Full);
    }

    #[test]
    fn eco_when_ahead_of_pace() {
        let now = 1_000_000;
        let half = now + 7 * 86_400 / 2; // reset in 3.5 days: half the week gone
        assert!(!Pace::new(0.50, half, 0.2, now).eco());
        assert!(Pace::new(0.65, half, 0.2, now).eco(), "ahead of the line");
        assert!(
            Pace::new(0.10, half, 0.9, now).eco(),
            "5-hour window nearly spent"
        );
    }
}
