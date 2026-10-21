<p align="center">
  <img src="site/brand/mark.svg" width="72" alt="logo de harnes : un h en cellules de terminal suivi d'un curseur">
</p>

<p align="center">
  <img src="docs/field.svg" width="100%" alt="harnes : un champ de caractères ASCII qui passe de nuages à un tore en rotation, un vortex et un réseau">
</p>

<p align="center">
  Un cockpit de bureau pour un équipage d'agents de code.<br>
  <sub>Rust · Tauri 2 · SolidJS · libghostty-vt · noir et blanc · MIT</sub>
</p>

<p align="center">
  <a href="README.md">English</a> · <b>Français</b> · <a href="https://sha-riyat.github.io/harnes/fr/">site</a>
</p>

<p align="center">
  <a href="#léquipage">équipage</a> ·
  <a href="#interface">interface</a> ·
  <a href="#fils">fils</a> ·
  <a href="#méthodes">méthodes</a> ·
  <a href="#harnes-e2e">e2e</a> ·
  <a href="#compiler-windows">compiler</a>
</p>

---

Vous parlez à un seul agent, l'**orchestrateur**. Il planifie, lance un équipage, suit chaque passage de relais et revient quand le travail atteint une barre fixée avant le début. Les terminaux restent hors de vue ; ce qui a besoin de vous arrive sous forme de carte.

Claude Code est le runtime par défaut. Chaque rôle peut aussi tourner sur Codex, Gemini CLI ou OpenCode. Les terminaux sont rendus par **libghostty-vt**, le moteur VT de Ghostty.

| | | |
|---|---|---|
| **Terminaux** | Panneaux ConPTY rendus par libghostty-vt, état des agents via les hooks Claude Code | `harnes-vt` `harnes-pty` |
| **Tokens** | Proxy Anthropic local : compression déterministe des `tool_result` et statistiques d'usage | `harnes-proxy` |
| **Contexte** | Serveur MCP `harnes-ctx` : exécution de commandes, index SQLite FTS5, recherche | `harnes-ctx` |
| **Tests e2e** | Specs YAML, étapes `act` en langage naturel résolues par les agents, rejeu sans modèle | `harnes-e2e` |
| **Tous les agents** | Vue de toute la machine : sessions Claude Code, Codex, Gemini CLI et fichiers de statut | `harnes-lookout` |
| **Équipage** | Planner → Teacher → Workers → Reviewer → gate du Teacher | `harnes-crew` |
| **Méthodes** | 27 packs OS (doctrine, oracle, skills, routage), dont 80 skills ECC sélectionnées, chargés par rôle | `harnes-os` |
| **Plan de contrôle** | Journal d'événements, vue de run typée, gardes, worktrees, mémoire revue par un humain, évals et HarnessSpec | `harnes-telemetry` `harnes-guards` `harnes-memory` `harnes-eval` |

Ponytail, Caveman et TasteSkill v2 (`design-taste-frontend`) restent des skills Claude Code.

## L'équipage

<p align="center"><img src="docs/crew.svg" width="100%" alt="Les agents dessinés par un caractère chacun : il respire pendant le travail, clignote quand on a besoin de vous, avec l'avancement du run et un message qui voyage vers un fil"></p>

| Rôle | Modèle | Travail | Sortie |
|---|---|---|---|
| Orchestrateur | Opus · medium | Votre seul interlocuteur. Transforme vos messages en runs (`crew_start`), suit les fils, vérifie que les résultats sont intégrés, vous fait son rapport. | ses réponses |
| Planner | Opus · medium | Découpe la tâche en lots et en vagues. Deux lots d'une même vague ne partagent jamais de fichier. | `plan.json` |
| Teacher | Opus · medium | Fixe la barre avant le travail : critères, preuve exigée pour chacun, moment où vous impliquer. | `bar.json` |
| Worker × N | Sonnet · high | Réalise un lot, puis écrit et lance son propre test e2e. | `reports/<lot>-r<n>.json` |
| Reviewer | Opus · medium | Relit le diff face au plan et aux vrais résultats e2e. | `review-<n>.json` |
| Teacher (gate) | Opus · medium | Confronte la barre aux résultats, puis renvoie le travail ou vous dit qu'il est prêt. | `gate-<n>.json` |
| scout | Haiku · high | Sous-agent que tout rôle lance pour de courtes tâches en parallèle : trouver du code, résumer des fichiers, lancer une commande. | sa réponse |

### La boucle

```
vous ─ chat ─► orchestrator     scout · os_route · os_brief · crew_start · crew_status
                  │
                  ▼
              planner           Read · Grep · ctx_search · os_brief    ─► plan.json, fils de lot
                  │
                  ▼
              teacher · bar     os_brief                               ─► bar.json
                  │
                  ▼
        ┌───► workers × N       thread_read · thread_post · ctx_execute
        │         │             e2e_observe ─► click / fill ─► check ─► save
        │         │                                                    ─► reports/<lot>-r<n>.json
        │         ▼
        │     suite e2e         harnes-e2e run : rejeu du cache, sans modèle
        │         │
        │         ▼
        ├──── reviewer          diff + plan + vrais résultats e2e      ─► review-<n>.json
        │ CHANGES │ PASS
        │         ▼
        └──── teacher · gate    barre face aux résultats               ─► gate-<n>.json
         pas prêt │ prêt
                  ▼
vous ◄── ready for you : le résumé du Teacher et les commits proposés

en continu : proxy (compression des tool_result, usage) · hooks (état des agents)
             lecture d'écran Ghostty (menus → cartes) · fils SQLite (questions, résumés)
```

- On ne vous interrompt qu'une fois la barre du Teacher atteinte, preuves de test à l'appui.
- Les commits sont proposés, jamais créés automatiquement.
- Les agents se passent le travail par des fichiers JSON dans `.harnes/runs/<id>/`.
- Les agents Claude Code signalent leur cycle de vie par des hooks ; les autres fournisseurs sont suivis via leur fichier de sortie.

### La revue et l'enveloppe de la semaine

La boucle de revue est faite pour aboutir en peu de tours et ne dépenser Opus que là où un oubli coûte le plus.

- **Trois modes**, choisis par l'orchestrateur avec `crew_start` : `light` (un worker Sonnet et une revue Sonnet, sans planner ni Teacher séparé), `full`, et `deep` (trois relecteurs Sonnet cherchent chacun de leur côté, puis un vérificateur Opus garde ce qu'au moins deux signalent ou ce qu'il confirme lui-même). Un run que le Teacher classe à risque `high` (CI, auth, paiement, migrations, secrets, ou chemins marqués sensibles dans `REVIEW.md`) a la revue renforcée et vous demande de relire le diff.
- **Gravités.** Chaque problème est `important`, `nit` ou `pre-existing`, avec un `fichier:ligne` comme preuve. Seuls les `important` repartent vers un worker ; les nits sont abandonnés, car un agent auteur les corrige tous au prix d'un tour ; les bugs déjà là avant le run vont dans le fil du run.
- **Convergence.** Dès le deuxième tour, le relecteur ne lit que ce que les corrections ont changé, face à ses verdicts précédents, ne répète aucun problème et nomme une fois toute une classe de bug. Après trois tours, le run s'arrête et l'orchestrateur propose une autre approche au lieu de relancer le même run.
- **`REVIEW.md`** (facultatif, écrit par vous à la racine du projet) : ce qu'est un échec par zone, les chemins à ignorer, les chemins sensibles.
- **Vos limites, lues en direct.** Chaque réponse d'Anthropic qui passe par le proxy d'un projet porte l'usage du compte : la fenêtre de 5 h et la semaine, en part utilisée, avec leurs heures de reset. harnes les lit là, donc rien à configurer, et l'onglet usage les dessine face à une ligne droite jusqu'au reset. Quand la semaine va plus vite que cette ligne, ou que la fenêtre de 5 h dépasse 85 %, les runs démarrent en économie : planner, Teacher et relecteur passent en Sonnet, et il n'y a plus de revue renforcée. Une fois la fenêtre de 5 h épuisée, aucun run ne démarre avant son reset.
- **Mesuré.** Chaque run laisse `metrics.json` (mode, tours, problèmes par gravité, réponse des workers à chaque correction : corrigé, faux, voulu, sans importance). L'onglet usage montre les revues par run, la part qui passe du premier coup, les problèmes bloquants et la part de problèmes faux.
- Les workers ne préchargent plus TasteSkill : ils ne le chargent que pour une page web publique.

## Interface

Noir et blanc, angles droits, aucune ombre. La forme, le remplissage et le mouvement portent l'état, jamais la couleur. Une ligne sous le pointeur reçoit un filet fin à gauche, la ligne courante un filet plein. Les boutons répondent par leur contour. Les surcouches sont des feuilles plates avec un filet épais en haut, sur un voile sombre. Les icônes viennent de [Phosphor](https://phosphoricons.com/) (light, MIT).

Tout ce qui vit est dessiné en ASCII animé, sur une seule boucle partagée qui ne tourne que lorsqu'un élément est à l'écran. Avec le mouvement réduit, chaque pièce reste sur une image fixe.

```
 +  orchestrator     working      plan --- bar --- w#rk ·:=+· review ··· gate ··· ready
 !  worker-L1      needs you      worker-L1 ····#=-:···· lot-1-L1
 =  worker-L2        working      ·:=+*#*+=:·:-=+*#  thinking
```

- **Le chat d'abord.** La vue principale est le chat avec l'orchestrateur. Son terminal reste caché ; sa transcription Claude devient des messages en vrai Markdown (gras, listes, code, citations), avec une ligne mono par appel d'outil.
- **Le champ.** Une vue vide se remplit d'un champ de caractères qui occupe tout le centre et s'élargit quand les panneaux latéraux se ferment. Il passe par 23 motifs, organiques et géométriques en alternance : nuages, plan en train de se dessiner, messages qui s'écrivent, trame, courants, tore en rotation, réseau de nœuds, tuiles de Truchet, ondes, spécimens typographiques, signaux d'oscilloscope, moiré, courbes de niveau, traceur, nuée d'oiseaux, formes Bauhaus, vortex, grille de mise en page, aurore, phyllotaxie, pluie, nuanciers et marées. Chacun se dissout dans le suivant le long d'un front bruité, et le champ se brouille sous le pointeur.
- **Vues.** `chat`, `run` (la trace typée, la gate, les revues, la mémoire et l'éval), `methods` (les packs OS) et `machine` (tous les agents de l'ordinateur). La grille de terminaux est dans la palette (`Ctrl K`).
- **Terminaux en dock.** Un terminal ne s'ouvre que si vous le demandez, dans un dock sous la vue principale : un onglet par terminal, redimensionnable, replié en un clic. Les couleurs du terminal suivent le thème clair ou sombre.
- **Permissions en cartes.** Chaque agent reçoit un hook Claude Code `PermissionRequest` bloquant. La demande s'affiche en *Allow · Always · Deny* ; votre réponse devient la décision du hook. Sans réponse en 10 minutes environ, Claude Code revient à sa propre invite.
- **Questions lues à l'écran.** Comme un agent computer-use lit un arbre d'accessibilité, harnes lit la grille Ghostty de chaque agent, repère les menus numérotés et les invites `(y/n)`, et les affiche en cartes. Votre choix repart en flèches puis Entrée, une touche à la fois.
- **Panneau latéral.** Trois icônes : les fils, le journal du run, l'usage.
- `?mock` (développement seulement) simule le backend pour travailler l'interface dans un navigateur. `npm run readme:art` redessine les animations de cette page à partir des scènes de l'app.

## Fils

Les agents, leurs sous-agents, l'orchestrateur et vous échangez dans des fils stockés en SQLite (`.harnes/threads.db`), partagés par tous les processus d'agent. Rien ne tourne dans le cloud.

- Chaque run a un fil `run-<id>` et chaque lot un fil `lot-<id>-<lot>`, amorcé avec la consigne du lot.
- Le moteur publie chaque étape du run, et les correctifs du Reviewer dans le fil du lot concerné.
- Les Workers lisent le fil de leur lot avant de commencer, y posent leurs questions et y font leur résumé avant de finir.
- Outils dans `harnes-ctx` : `thread_list`, `thread_read`, `thread_post`, `thread_create`, `thread_close`. Les messages sont signés (`HARNES_AGENT`) ; les sous-agents ajoutent `as`, affiché `worker-L1/scout`.
- Les fils se ferment à l'ouverture de la gate mais restent lisibles. Publier à nouveau en rouvre un.

SQLite plutôt qu'un backend hébergé comme Convex : une seule machine, hors ligne, aucun service en plus, et vos conversations ne quittent jamais votre ordinateur.

## Méthodes

harnes livre 27 packs OS dans son binaire (`crates/os/packs`) et les installe dans `%APPDATA%\dev.harnes.app\os` une fois par build. Chaque pack est une méthode : une doctrine (`OS.md`), un oracle de règles par défaut et de refus non négociables, des principes premiers, des skills avec schémas, des playbooks, des règles, un routage par intention et des évaluations.

| Rôle | Packs |
|---|---|
| orchestrator | orchestration, agent, team-delegation, decision, ecc-agent |
| planner | project, stepper, blueprint, execution, ecc-agent |
| workers | builder, tool-integration, documentation, ecc-engineering |
| reviewer | quality-evaluation, security, builder, ecc-security, ecc-engineering |
| teacher | validation, quality-evaluation |
| scouts | market-research, customer-discovery, documentation |

Les trois packs `ecc-*` ajoutent 80 skills sélectionnées d'[ECC](https://github.com/affaan-m/ECC) (MIT) : 48 ingénierie, 9 sécurité et 23 agents, copiées telles quelles à un commit épinglé. C'est une sélection, pas le catalogue complet de 311 skills. Chaque pack contient `LICENSE-ECC`, `NOTICE` et `PROVENANCE.md`. Les packs produit (brand, pricing, offer, positioning, design…) sont chargés à la demande ; voir [THIRD_PARTY.md](THIRD_PARTY.md) pour toutes les attributions.

<details>
<summary>Outils et import</summary>

- `os_list` : les packs installés.
- `os_brief(id)` : la doctrine, l'oracle, les principes premiers et les skills d'un pack, dans l'ordre d'empaquetage du contexte propre aux packs.
- `os_route(text)` : le pack et le skill adaptés à une demande.
- `os_read(id, path)` : un fichier d'un pack.

Importez un zip, un dossier de zips ou des dossiers de packs depuis la vue méthodes, ou :

```
harnes-ctx os-import PATH
```

Les projets ouverts sont rééquipés immédiatement.
</details>

## Plan de contrôle

La vue de run est typée (`harnes.view.v1`), jamais du HTML généré par le modèle. Elle lit `.harnes/` et réunit les preuves et les décisions :

- **Trace et usage.** Chaque hook, appel d'outil, commande, fichier et token devient un événement dans `.harnes/telemetry.db` et `.harnes/telemetry/events.jsonl` (`harnes.event.v1`). Les secrets sont caviardés avant stockage.
- **Gardes.** `.harnes/guards.json` contient des règles allow/ask/deny. Le garde natif `harnes __hook` (ou `harnes-hook` seul) le lit avant chaque appel d'outil, signale l'événement et continue de fonctionner si le cockpit est fermé. `PermissionRequest` reste une décision humaine bloquante.
- **Worktrees.** Chaque lot travaille dans `.harnes/worktrees/<run>/<lot>` sur `harnes/<run>/<lot>`. Les fusions sont en `--no-ff` ; un conflit annule la fusion, conserve le worktree et attend un humain. Aucune réécriture d'historique.
- **Mémoire.** Un run propose des candidats adossés à des preuves ; seul un humain peut en promouvoir un. `HARNES_MEMORY_REVIEW=1` active l'outil de revue et `HARNES_MEMORY_WRITE=0` désactive les écritures.
- **HarnessSpec et évals.** `.harnes/harness.json` décrit les huit composants du harnais et ses budgets. `.harnes/evals/*.yaml` contient les suites déterministes ; les résultats vont dans `.harnes/evals/results/` et `.harnes/eval-ledger.json`.
- **Réglage.** `tune_plan` propose un balayage d'un seul composant après une baseline ; on garde un changement seulement si la même suite gagne ou reste neutre. Il n'appelle jamais de modèle lui-même.

Contrats, fichiers et exemples complets : [docs/control-plane.md](docs/control-plane.md).

## Projets

harnes travaille projet par projet, comme les espaces de travail de Herdr ou les projets de Claude Desktop.

- **Barre latérale.** Tous les projets ouverts. Le projet actif montre son orchestrateur, les agents de son run et ses sessions. Un badge compte les agents qui vous attendent. Au survol d'un projet : nouveau terminal, nouvelle session Claude, fermer.
- **Plusieurs à la fois.** Chaque projet a son propre proxy, son stockage de contexte, ses fils et son run : les runs avancent en parallèle.
- **Restauration.** harnes enregistre l'espace de travail dans `%APPDATA%\dev.harnes.app\session.json` et restaure projets, terminaux, sessions Claude (`claude --resume <id>`) et le run en cours. Un agent dont le fichier de sortie existe est sauté : un run interrompu reprend là où il s'était arrêté.
- **Changer de fournisseur.** Le badge de fournisseur d'un panneau de session le bascule vers Claude, Codex, Gemini ou OpenCode. harnes écrit l'ancienne transcription et les derniers messages des fils dans `.harnes/context/<session>-<ts>.md` et démarre le nouveau fournisseur sur ce fichier.

## harnes-e2e

```yaml
# tests/billing.e2e.yaml
name: un membre passe au plan Pro
steps:
  - open: /settings/billing
  - act: passe l'espace de travail au plan Pro           # langage naturel
  - expect: { target: { role: status }, contains: Pro }  # vérification exacte
```

- Étapes exactes `open` `click` `fill` `press` `wait` `expect` ; étapes en langage naturel `act`.
- Locators : `role`, `name`, `label`, `text`, `placeholder`, `testid`, `css`, `exact`, `nth`.
- La première fois, un Worker résout une étape `act` via le serveur MCP `harnes-e2e` (`e2e_prepare`, `e2e_click` / `e2e_fill` / `e2e_press`, `e2e_check`, `e2e_save`) en pilotant le vrai navigateur. Les actions vont dans `.harnes/e2e/cache/` ; les runs suivants les rejouent sans appel au modèle. Si l'app change et que le rejeu casse, l'étape repasse « à résoudre ».
- Codes de sortie : 0 réussi, 1 échec, 2 spec ou config invalide, 3 app ou navigateur indisponible, 5 étapes `act` à résoudre.
- Pilote Chrome ou Edge via CDP ; démarre l'app depuis `app.command` si besoin et arrête son arbre de processus.
- `harnes-e2e run [files] [--json]` · `harnes-e2e init --url … --command …` · `harnes-e2e mcp`

## Machine

La vue `machine` interroge toutes les 2 s les fichiers que chaque outil écrit déjà. Elle ne démarre, n'arrête ni ne contacte jamais une session.

| Source | Lit |
|---|---|
| Claude Code | `~/.claude/sessions/<pid>.json` (statut, raison d'attente) ; PID morts filtrés |
| Codex | `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl` (tours, nombres de tokens) |
| Gemini CLI | `~/.gemini/tmp/<project>/chats/session-*.jsonl` (statut déduit de l'activité d'écriture) |
| Fichiers de statut | `~/.harnes/sessions/*.json`, `~/.agent-lookout/sessions/*.json` : `{"agent","status","name"?,"cwd"?,"pid"?}` |

Elle montre qui a besoin de vous (la plus longue attente d'abord, un clic pour aller à son panneau), la dernière heure, chaque session avec son fournisseur, un journal d'événements et la santé de chaque source. Les agents de harnes sont marqués.

<details>
<summary>Fournisseurs</summary>

`.harnes/providers.json` (facultatif) :

```json
{
  "roles": { "worker": "codex", "scout": "claude" },
  "providers": { "codex": { "program": "codex", "args": ["--full-auto", "{prompt_with_role}"], "hooks": false } }
}
```

- Intégrés : `claude` (par défaut, avec hooks), `codex`, `gemini`, `opencode`.
- Variables : `{agent}`, `{settings}`, `{prompt}`, `{prompt_with_role}` (ajoute la consigne de suivre `.claude/agents/<role>.md`).
- Les réglages par défaut hors Claude sont indicatifs. Remplacez leurs options si une CLI change.
</details>

<details>
<summary>Arborescence</summary>

```
crates/vt       FFI libghostty-vt (shim C) → images de cellules
crates/pty      panneaux ConPTY qui alimentent le VT
crates/ctx      serveur MCP harnes-ctx + stockage SQLite FTS5
crates/proxy    proxy Anthropic : compression, passage SSE, usage
crates/e2e      harnes-e2e : specs, navigateur CDP, cache de rejeu, exécuteur, MCP
crates/lookout  sessions d'agents de la machine : adaptateurs, événements, historique
crates/threads  fils des agents (SQLite) : runs, lots, questions, historique
crates/os       packs OS : zips livrés, import, index, routage par rôle
crates/telemetry  journal d'événements : schéma, caviardage, index SQLite, résumés
crates/guards   politique runtime, `harnes-hook` natif, findings a posteriori
crates/worktree worktrees git par lot, fusion prudente, conflits conservés
crates/memory   candidats revus par un humain et mémoire approuvée
crates/eval     suites déterministes, résultats, ledger, plans de réglage
crates/crew     installation, hooks, fournisseurs, HarnessSpec, vues typées, passations
app/            app Tauri 2 (src-tauri) + interface SolidJS
template/       agents (orchestrator, planner, teacher, worker, reviewer, scout), réglages, CLAUDE.md
docs/           les animations de cette page et les contrats du plan de contrôle
site/           le site (anglais et français) et le logo
```
</details>

## Compiler (Windows)

Rust (MSVC) avec Visual Studio Build Tools 2022, Zig 0.16.0, Node.js via `nvm`, Chrome ou Edge, et la CLI Claude Code connectée.

```powershell
# libghostty-vt, une seule fois (définissez GHOSTTY_VT_DIR s'il est ailleurs)
git clone --depth 1 https://github.com/ghostty-org/ghostty.git $HOME\tools\ghostty
cd $HOME\tools\ghostty; zig build -Demit-lib-vt -Doptimize=ReleaseFast -Dsimd=false

cargo test --workspace

cd app; npm install
npm run app        # compile harnes-ctx + harnes-e2e, puis tauri dev
npm run app:build  # release + installateur NSIS
```

## Utilisation

1. **+** à côté de *projects*, choisissez un dossier. harnes y écrit les agents, les réglages et `.mcp.json` (`harnes-ctx`, `harnes-e2e`) et démarre le proxy.
2. La première fois, ouvrez une session Claude et acceptez la demande de confiance du dossier.
3. Parlez à l'orchestrateur dans le chat. Suivez le travail dans la barre latérale, la vue `machine` et les fils.
4. Quand la gate s'ouvre, lisez *ready for you* et les commits proposés. Testez le résultat, puis validez.

## Notes

- `harnes-ctx` tient les sorties hors du contexte du modèle. Ce n'est pas un bac à sable de sécurité.
- La compression du proxy est déterministe : le cache des prompts continue de fonctionner.
- harnes-e2e et harnes-lookout sont des implémentations indépendantes inspirées de TesterArmy e2e (Apache-2.0) et agent-lookout (MIT). Aucun code de ces projets n'est inclus.
- Les 80 skills ECC sont sous MIT et livrées avec leur licence et leur provenance. Beacon (MIT) a inspiré la mémoire revue par un humain et Harness 2.0 (Apache-2.0) la méthode de harnais et d'évaluation ; aucun code de ces projets n'est livré. OpenUI et CopilotKit ne sont pas des dépendances. Voir [THIRD_PARTY.md](THIRD_PARTY.md) et [NOTICE](NOTICE).
- L'API de libghostty-vt évolue encore. `crates/vt/shim/hv.c` est le seul fichier à adapter.
