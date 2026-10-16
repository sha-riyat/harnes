//! Which packs a role loads, and which pack/skill fits a piece of text.

use crate::Library;
use serde::Serialize;

/// Default packs per crew role. Product packs (brand, pricing, offer…) are
/// not bound to a role: agents reach them through `os_route`.
pub const ROLES: &[(&str, &[&str])] = &[
    ("orchestrator", &["orchestration-os", "agent-os", "team-delegation-os", "decision-os"]),
    ("planner", &["project-os", "stepper-os", "blueprint-os", "execution-os"]),
    ("worker", &["builder-os", "tool-integration-os", "documentation-os"]),
    ("reviewer", &["quality-evaluation-os", "security-os", "builder-os"]),
    ("teacher", &["validation-os", "quality-evaluation-os"]),
    ("scout", &["market-research-os", "customer-discovery-os", "documentation-os"]),
];

/// Installed packs for a role.
pub fn for_role(lib: &Library, role: &str) -> Vec<String> {
    let installed = lib.list();
    ROLES
        .iter()
        .find(|(r, _)| *r == role)
        .map(|(_, ids)| ids.iter().filter(|i| installed.iter().any(|p| p.id == **i)).map(|i| i.to_string()).collect())
        .unwrap_or_default()
}

/// One line for an agent's first prompt: the packs to load before acting.
pub fn role_note(lib: &Library, role: &str) -> String {
    let ids = for_role(lib, role);
    if ids.is_empty() {
        return String::new();
    }
    format!(
        "\nMéthodes de ton rôle (packs OS) : {}. Charge-les avec os_brief avant de décider et respecte leur oracle.",
        ids.join(", ")
    )
}

#[derive(Debug, Clone, Serialize)]
pub struct Hit {
    pub os: String,
    pub intent: String,
    pub route: String,
    pub score: f32,
}

fn words(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 2)
        .map(String::from)
        .collect()
}

impl Library {
    /// Ranks intents of every pack against `text`. An exact phrase scores 1;
    /// otherwise the share of the phrase's words found, plus a little for
    /// words of the pack's purpose.
    pub fn route(&self, text: &str, limit: usize) -> Vec<Hit> {
        let low = text.to_lowercase();
        let tw = words(text);
        let mut hits = Vec::new();
        for p in self.list() {
            let purpose = words(&p.purpose);
            let bonus = purpose.iter().filter(|w| tw.contains(w)).count() as f32 * 0.05;
            for i in self.intents(&p.id) {
                let mut best = 0f32;
                for phrase in i.pattern.split('|').map(str::trim).filter(|s| !s.is_empty()) {
                    let score = if low.contains(&phrase.to_lowercase()) {
                        1.0
                    } else {
                        let pw = words(phrase);
                        if pw.is_empty() { 0.0 } else { pw.iter().filter(|w| tw.contains(w)).count() as f32 / pw.len() as f32 * 0.8 }
                    };
                    best = best.max(score);
                }
                let score = (best + bonus).min(1.0);
                if score > 0.15 {
                    hits.push(Hit { os: p.id.clone(), intent: i.id.clone(), route: i.route.clone(), score });
                }
            }
        }
        hits.sort_by(|a, b| b.score.total_cmp(&a.score));
        hits.truncate(limit);
        hits
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::fixture;

    #[test]
    fn routes_phrases_and_roles() {
        let lib = fixture("route");
        let hits = lib.route("Can you review this diff before merge?", 3);
        assert_eq!(hits[0].route, "skill:review-diff");
        assert!(hits[0].score >= 1.0);
        assert!(lib.route("bake a cake", 3).is_empty());
        assert_eq!(super::for_role(&lib, "worker"), vec!["builder-os"]);
        assert!(super::role_note(&lib, "teacher").is_empty());
    }
}
