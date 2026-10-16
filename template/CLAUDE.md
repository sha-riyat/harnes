# Projet piloté par harnes

Ce projet est piloté par l'application **harnes** (Rust + Tauri). L'application lance l'équipage dans des panes séparés et les fait communiquer par fichiers JSON dans `.harnes/runs/<run>/` :

| Rôle | Agent | Modèle | Effort | Sortie |
|---|---|---|---|---|
| Orchestrateur | `orchestrator` | Opus | medium | parle à l'utilisateur, pilote le run |
| Planner | `planner` | Opus | medium | `plan.json` |
| Teacher | `teacher` | Opus | medium | `bar.json`, `gate-<n>.json` |
| Workers | `worker` | Sonnet | high | `reports/<lot>.json` |
| Scouts | `scout` | Haiku | low | sous-agents rapides |
| Reviewer | `reviewer` | Opus | medium | `review.json` |

## Fils de discussion
Les agents communiquent par **fils** (MCP harnes-ctx : `thread_list`, `thread_read`, `thread_post`, `thread_create`, `thread_close`) : un fil `run-<id>` par run et un fil `lot-<id>-<lot>` par lot. Lis le fil de ton lot avant de commencer, pose-y tes questions, résume-y ton travail avant de finir. Un sous-agent signe avec `as` (ex. `scout`). Les fils clos restent consultables.

## Outils intégrés
- **harnes-ctx** (MCP) : `ctx_execute`, `ctx_execute_file`, `ctx_search`, `ctx_retrieve`, `ctx_index`, `ctx_stats`. Utilise-les pour toute sortie volumineuse (tests, builds, logs, gros fichiers).
- **Proxy harnes** : compresse les gros résultats d'outils avant l'envoi au modèle. Un marqueur `[harnes: … elided … id=XYZ]` signifie que l'original est lisible avec `ctx_retrieve id=XYZ`.
- **harnes-e2e** (MCP) : tests `tests/**/*.e2e.yaml` mêlant étapes exactes et objectifs `act` en langage naturel. Les agents résolvent eux-mêmes les `act` en pilotant le vrai navigateur ; les actions sont rejouées ensuite sans modèle. harnes relance toute la suite avant chaque revue.
- Skills : **Ponytail** (le moins de code possible), **Caveman** (sorties compactes), **TasteSkill v2** `design-taste-frontend` (pages web publiques seulement).

## Règles maison
- Écris tes sorties JSON exactement au chemin donné, au format demandé.
- Une action = un commit. Aucun commit sans accord explicite de l'utilisateur après ses tests.
- Jamais de `Co-authored-by` ni d'attribution Claude dans un message de commit.
- Ne jamais committer un `.md` créé par un agent, sauf `README.md`.
- Fichiers : 200-300 lignes, 400 max.
- Node.js : uniquement via `nvm`.
