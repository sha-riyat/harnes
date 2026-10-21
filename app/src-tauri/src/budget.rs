//! The account's usage limits, as Anthropic reports them on every response
//! that goes through a project's proxy: the 5-hour and 7-day windows. Runs
//! start in eco when the week runs ahead of a straight line to its reset or
//! the 5-hour window is nearly spent, and no run starts once it is spent.

use crate::state::Shared;
use harnes_crew::budget::Pace;
use harnes_proxy::stats::Limits;
use serde_json::{json, Value};
use tauri::State;

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// The freshest limits any project's proxy has seen.
pub fn limits(st: &Shared) -> Option<Limits> {
    st.projects
        .read()
        .iter()
        .filter_map(|p| p.proxy.snapshot().limits)
        .max_by_key(|l| l.seen)
}

pub fn pace(st: &Shared) -> Option<Pace> {
    limits(st).map(|l| Pace::new(l.seven_day, l.seven_day_reset, l.five_hour, now()))
}

/// Why a run cannot start now, if the 5-hour window is spent.
pub fn blocked(st: &Shared) -> Option<String> {
    let l = limits(st)?;
    let t = now();
    (l.five_hour >= 1.0 && l.five_hour_reset > t).then(|| {
        format!(
            "limite des 5 h atteinte : reprise dans {} min",
            (l.five_hour_reset - t).div_ceil(60)
        )
    })
}

#[tauri::command]
pub fn budget_get(st: State<'_, Shared>) -> Value {
    let Some(l) = limits(st.inner()) else {
        return json!(null);
    };
    let p = Pace::new(l.seven_day, l.seven_day_reset, l.five_hour, now());
    json!({ "limits": l, "week_elapsed": p.week_elapsed, "eco": p.eco() })
}
