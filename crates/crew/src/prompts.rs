//! Initial prompts given to each agent. Agent-specific rules live in the
//! agent definitions (`.claude/agents/*.md`); prompts only carry the job.

use crate::plan::{Fix, Lot, Plan};

fn fwd(p: &str) -> String {
    p.replace('\\', "/")
}

pub fn planner(task: &str, plan_path: &str) -> String {
    format!(
        "Tâche de l'utilisateur : {task}\n\n\
         Écris le plan JSON dans `{}` en suivant ton schéma. N'écris aucun autre fichier. \
         Termine par PLAN OK.",
        fwd(plan_path)
    )
}

pub fn worker(plan: &Plan, lot: &Lot, report_path: &str, fixes: &[Fix], thread: &str) -> String {
    let lot_json = serde_json::to_string_pretty(lot).unwrap_or_default();
    let mut s = format!(
        "Contexte du projet : {}\nCommande de test : {}\n\nTon lot :\n{lot_json}\n",
        plan.context,
        plan.test_command.as_deref().unwrap_or("(aucune)")
    );
    if !fixes.is_empty() {
        s.push_str("\nLe Reviewer demande ces corrections pour ton lot :\n");
        for f in fixes {
            s.push_str(&format!("- {} : {} -> {}\n", f.r#where, f.problem, f.fix));
        }
    }
    s.push_str(&format!(
        "\nTon fil de discussion : `{thread}` (MCP thread_read / thread_post). Lis-le avant de commencer ; \
         poses-y tes questions aux autres agents et résume-y ce que tu as fait avant de finir.\n\
         Réalise le lot, lance les tests, puis écris ton rapport JSON dans `{}`. Termine par LOT OK.",
        fwd(report_path)
    ));
    s
}

pub fn reviewer(task: &str, plan_path: &str, reports: &[String], review_path: &str, round: u32, e2e: Option<&str>, previous: &[String]) -> String {
    let list = reports.iter().map(|r| format!("- `{}`", fwd(r))).collect::<Vec<_>>().join("\n");
    let e2e = e2e
        .map(|r| {
            format!(
                "\nharnes vient de lancer toute la suite e2e (harnes-e2e) : {r}\n\
                 Détails : `.harnes/e2e/report.json`, traces des échecs dans `.harnes/e2e/results/`.\n"
            )
        })
        .unwrap_or_default();
    // Later rounds look at what the fixes changed, against what was already said.
    let before = if previous.is_empty() {
        String::new()
    } else {
        format!(
            "\nTes verdicts précédents : {}. Relis seulement ce que les corrections ont changé et ne répète aucun problème déjà signalé.\n",
            previous.iter().map(|p| format!("`{}`", fwd(p))).collect::<Vec<_>>().join(", ")
        )
    };
    format!(
        "Tâche d'origine : {task}\nTour de revue : {}\n\nPlan : `{}`\nRapports des Workers :\n{list}\n{e2e}{before}\n\
         Relis le diff contre le plan, lance les tests, puis écris ton verdict JSON dans `{}`. \
         Termine par REVIEW OK.",
        round + 1,
        fwd(plan_path),
        fwd(review_path)
    )
}

/// The last step of a deep review: keep what the finders agree on.
pub fn verifier(task: &str, found: &[String], review_path: &str) -> String {
    let list = found.iter().map(|f| format!("- `{}`", fwd(f))).collect::<Vec<_>>().join("\n");
    format!(
        "Revue renforcée. Tâche d'origine : {task}\n\nPlusieurs relecteurs ont cherché des problèmes, chacun de son côté :\n{list}\n\n\
         Garde un problème seulement s'il est signalé par au moins deux relecteurs, ou si tu le confirmes toi-même dans le code \
         avec une preuve `fichier:ligne`. Fusionne les doublons, garde la gravité la plus juste, et écris le verdict final dans `{}` \
         avec le même schéma. Termine par REVIEW OK.",
        fwd(review_path)
    )
}

pub fn teacher_bar(task: &str, plan_path: &str, bar_path: &str, e2e_ready: bool) -> String {
    let e2e = if e2e_ready { "Le projet a une suite harnes-e2e." } else { "Le projet n'a pas encore de suite harnes-e2e (le plan peut en créer une)." };
    format!(
        "Calibrage. Tâche de l'utilisateur : {task}
Plan : `{}`
{e2e}

         Fixe la barre de validation et écris-la dans `{}`. Termine par BAR OK.",
        fwd(plan_path),
        fwd(bar_path)
    )
}

pub fn teacher_gate(task: &str, bar_path: &str, review_path: &str, e2e: Option<&str>, gate_path: &str) -> String {
    let e2e = e2e.map(|r| format!("Résultat du testeur (harnes-e2e, suite complète) : {r}
")).unwrap_or_else(|| "Pas de suite e2e dans ce projet.
".into());
    format!(
        "Porte de validation. Tâche : {task}
Barre : `{}`
Verdict du Reviewer : `{}`
{e2e}
         Décide si le travail est prêt à être montré à l'utilisateur et écris ta décision dans `{}`. Termine par GATE OK.",
        fwd(bar_path),
        fwd(review_path),
        fwd(gate_path)
    )
}

pub fn run_thread_note(run: &str) -> String {
    format!("
Fil du run : `{}` (MCP thread_post) pour signaler une ambiguïté ou une décision importante.", crate::threading::run_thread(run))
}

pub fn threads_note(run: &str, plan: &Plan) -> String {
    let lots = plan.lots.iter().map(|l| format!("`{}`", crate::threading::lot_thread(run, &l.id))).collect::<Vec<_>>().join(", ");
    format!("
Fils des lots : {lots}. Lis-les (thread_read) : questions, décisions et résumés des Workers.")
}
