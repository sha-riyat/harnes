//! The model policy for every crew role: the Planner alone runs on Opus, in
//! medium effort; the orchestrator, Teacher, Workers and reviewers run on
//! Sonnet, in high effort; scouts (short, read-only errands) run on Haiku,
//! in high effort. The run's mode and the weekly pace never change
//! that split.

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
///
/// `opus` and `sonnet` are Claude Code aliases for the latest models, currently
/// Opus 5.5 and Sonnet 5.5. Keeping the aliases avoids pinning a dated API id
/// that would go stale with the next patch release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tune {
    pub model: &'static str,
    pub effort: &'static str,
}

pub const OPUS_MEDIUM: Tune = Tune {
    model: "opus",
    effort: "medium",
};
pub const SONNET_HIGH: Tune = Tune {
    model: "sonnet",
    effort: "high",
};
/// Scouts only look things up for the others and never edit.
pub const HAIKU_HIGH: Tune = Tune {
    model: "haiku",
    effort: "high",
};
/// Interactive Claude panes (the orchestrator included) are launched outside
/// the role engine, so they use the same non-Planner policy explicitly.
pub const SESSION: Tune = SONNET_HIGH;

/// Reviewers that look for problems in a deep review, before the verifier.
pub const FINDERS: u32 = 3;

/// The model policy for `role`. It is deliberately independent of the run mode
/// and of eco weeks: only the Planner gets Opus.
pub fn tune(role: Role) -> Tune {
    match role {
        Role::Planner => OPUS_MEDIUM,
        Role::Teacher | Role::Worker | Role::Reviewer => SONNET_HIGH,
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
    fn only_the_planner_gets_opus() {
        assert_eq!(tune(Role::Planner), OPUS_MEDIUM);
        assert_eq!(tune(Role::Teacher), SONNET_HIGH);
        assert_eq!(tune(Role::Worker), SONNET_HIGH);
        assert_eq!(tune(Role::Reviewer), SONNET_HIGH);
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
