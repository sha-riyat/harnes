<p align="center">
  <img src="site/brand/mark.svg" width="72" alt="harnes logo: an h set in terminal cells with a cursor after it">
</p>

<p align="center">
  <img src="docs/field.svg" width="100%" alt="harnes: a field of ASCII characters drifting from clouds to a turning torus, a vortex and a network">
</p>

<p align="center">
  A desktop cockpit for a crew of coding agents.<br>
  <sub>Rust · Tauri 2 · SolidJS · libghostty-vt · black and white · MIT</sub>
</p>

<p align="center">
  <b>English</b> · <a href="README.fr.md">Français</a> · <a href="https://sha-riyat.github.io/harnes/">website</a>
</p>

<p align="center">
  <a href="#the-crew">crew</a> ·
  <a href="#interface">interface</a> ·
  <a href="#threads">threads</a> ·
  <a href="#methods">methods</a> ·
  <a href="#harnes-e2e">e2e</a> ·
  <a href="#build-windows">build</a>
</p>

---

You talk to one agent, the **orchestrator**. It plans, starts a crew, follows every hand-off and comes back when the work meets a bar set before it started. Terminals stay out of sight; what needs you comes to you as a card.

Claude Code is the default runtime. Each role can also run on Codex, Gemini CLI or OpenCode. Terminal panes are rendered by **libghostty-vt**, Ghostty's VT engine.

| | | |
|---|---|---|
| **Terminals** | ConPTY panes rendered by libghostty-vt, agent state from Claude Code hooks | `harnes-vt` `harnes-pty` |
| **Tokens** | Local Anthropic proxy: deterministic `tool_result` compression and usage stats | `harnes-proxy` |
| **Context** | MCP server `harnes-ctx`: command runs, SQLite FTS5 index, retrieval | `harnes-ctx` |
| **e2e tests** | YAML specs, natural-language `act` steps resolved by the agents, model-free replay | `harnes-e2e` |
| **Every agent** | Machine-wide view of Claude Code, Codex, Gemini CLI and status-file sessions | `harnes-lookout` |
| **Crew** | Planner → Teacher → Workers → Reviewer → Teacher gate | `harnes-crew` |
| **Methods** | 24 OS packs (doctrine, oracle, skills, routing) shipped inside harnes, loaded by role | `harnes-os` |

Ponytail, Caveman and TasteSkill v2 (`design-taste-frontend`) remain Claude Code skills.

## The crew

<p align="center"><img src="docs/crew.svg" width="100%" alt="Agents drawn as one character each: breathing while they work, blinking when they need you, with the run's progress and a message travelling to a thread"></p>

| Role | Model | Job | Output |
|---|---|---|---|
| Orchestrator | Opus · medium | Your only interlocutor. Turns your messages into runs (`crew_start`), follows the threads, checks results were integrated, reports back. | its replies |
| Planner | Opus · medium | Splits the task into lots and waves. Lots in one wave never share files. | `plan.json` |
| Teacher | Opus · medium | Sets the bar before work starts: criteria, the evidence each needs, when to involve you. | `bar.json` |
| Worker × N | Sonnet · high | Implements one lot, then writes and runs its own e2e test. | `reports/<lot>-r<n>.json` |
| Reviewer | Opus · medium | Reviews the diff against the plan and the real e2e results. | `review-<n>.json` |
| Teacher (gate) | Opus · medium | Checks the bar against the results, then sends the work back or tells you it is ready. | `gate-<n>.json` |
| scout | Haiku · high | Subagent any role starts for short parallel tasks: finding code, summarising files, running a command. | its reply |

### The loop

```
you ─ chat ─► orchestrator      scout · os_route · os_brief · crew_start · crew_status
                  │
                  ▼
              planner           Read · Grep · ctx_search · os_brief    ─► plan.json, lot threads
                  │
                  ▼
              teacher · bar     os_brief                               ─► bar.json
                  │
                  ▼
        ┌───► workers × N       thread_read · thread_post · ctx_execute
        │         │             e2e_observe ─► click / fill ─► check ─► save
        │         │                                                    ─► reports/<lot>-r<n>.json
        │         ▼
        │     e2e suite         harnes-e2e run: cached replay, no model
        │         │
        │         ▼
        ├──── reviewer          diff + plan + real e2e results         ─► review-<n>.json
        │ CHANGES │ PASS
        │         ▼
        └──── teacher · gate    bar against the results                ─► gate-<n>.json
        not ready │ ready
                  ▼
you ◄── ready for you: the Teacher's summary and the proposed commits

all along: proxy (tool_result compression, usage) · hooks (agent state)
           Ghostty screen reading (menus → cards) · SQLite threads (questions, summaries)
```

- You are interrupted only once the Teacher's bar is met, and the test results back it up.
- Commits are proposed, never created automatically.
- Agents hand work to each other through JSON files in `.harnes/runs/<id>/`.
- Claude Code agents report their lifecycle through hooks; other providers are tracked through their output file.

## Interface

Black and white, square corners, no shadows. Shape, fill and motion carry state, never colour. A row under the pointer gets a thin rule on its left, the current one a solid rule. Buttons answer with their outline. Overlays are flat sheets with a heavy top rule over a dark veil. Icons are [Phosphor](https://phosphoricons.com/) (light, MIT).

Everything that lives is drawn in animated ASCII, on one shared loop that runs only while an element is on screen. With reduced motion, each piece holds a still frame.

```
 +  orchestrator     working      plan --- bar --- w#rk ·:=+· review ··· gate ··· ready
 !  worker-L1      needs you      worker-L1 ····#=-:···· lot-1-L1
 =  worker-L2        working      ·:=+*#*+=:·:-=+*#  thinking
```

- **Chat first.** The main view is the chat with the orchestrator. Its terminal stays hidden; its Claude transcript becomes messages with real Markdown (bold, lists, code, quotes), and one mono line per tool call.
- **The field.** An empty view fills with a field of characters that takes the whole centre and widens when the side panels close. It moves through 23 motifs, organic and geometric in turn: clouds, a blueprint being drawn, typing messages, halftone, currents, a turning torus, a node network, Truchet tiles, ripples, type specimens, oscilloscope signals, moiré, contours, a pen plotter, a murmuration, Bauhaus shapes, a vortex, a layout grid, aurora, phyllotaxis, rain, swatches and tides. Each dissolves into the next along a noisy front, and the field scrambles under the pointer.
- **Views.** `chat`, `methods` (the OS packs) and `machine` (every agent on the computer). The terminal grid is in the palette (`Ctrl K`).
- **Docked terminals.** A terminal opens only when you ask, in a dock under the main view: one tab per terminal, resizable, folded away in one click. Terminal colours follow the light or dark theme.
- **Permissions as cards.** Each agent gets a blocking Claude Code `PermissionRequest` hook. The request shows as *Allow · Always · Deny*; your answer is the hook's decision. Without an answer within about 10 minutes, Claude Code falls back to its own prompt.
- **Questions read from the screen.** Like a computer-use agent reading an accessibility tree, harnes reads each agent's Ghostty grid, finds numbered menus and `(y/n)` prompts, and shows them as cards. Your choice goes back as arrow keys and Enter, one key at a time.
- **Side panel.** Three icons: threads, the run log, usage.
- `?mock` (dev only) fakes the backend so the UI can be worked on in a browser. `npm run readme:art` redraws the animations on this page from the app's own scenes.

## Threads

Agents, sub-agents, the orchestrator and you talk through threads stored in SQLite (`.harnes/threads.db`), shared by every agent process. Nothing runs in the cloud.

- Each run gets a `run-<id>` thread and each lot a `lot-<id>-<lot>` thread, seeded with the lot's brief.
- The engine posts every step of the run, and the Reviewer's fixes in the matching lot thread.
- Workers read their lot thread before starting, ask their questions there, and sum up there before finishing.
- Tools in `harnes-ctx`: `thread_list`, `thread_read`, `thread_post`, `thread_create`, `thread_close`. Messages are signed (`HARNES_AGENT`); sub-agents add `as`, shown as `worker-L1/scout`.
- Threads close when the gate opens but stay readable. Posting again reopens one.

SQLite rather than a hosted backend such as Convex: one machine, offline, no extra service, and your conversations never leave your computer.

## Methods

harnes ships 24 OS packs inside its binary (`crates/os/packs`) and installs them into `%APPDATA%\dev.harnes.app\os` once per build. Each pack is a method: a doctrine (`OS.md`), an oracle of non-negotiable defaults and refusals, first principles, skills with schemas, playbooks, workflows, intent routing and evals.

| Role | Packs |
|---|---|
| orchestrator | orchestration, agent, team-delegation, decision |
| planner | project, stepper, blueprint, execution |
| workers | builder, tool-integration, documentation |
| reviewer | quality-evaluation, security, builder |
| teacher | validation, quality-evaluation |
| scouts | market-research, customer-discovery, documentation |

Product packs (brand, pricing, offer, positioning, design…) are reached on demand.

<details>
<summary>Tools and import</summary>

- `os_list`: the installed packs.
- `os_brief(id)`: a pack's doctrine, oracle, first principles and skills, in the packs' own context-packing order.
- `os_route(text)`: the pack and skill that fit a request.
- `os_read(id, path)`: one file of a pack.

Import a zip, a folder of zips or pack folders from the methods view, or:

```
harnes-ctx os-import PATH
```

Open projects are re-equipped straight away.
</details>

## Projects

harnes works project by project, like Herdr workspaces or projects in Claude Desktop.

- **Sidebar.** Every open project. The active one shows its orchestrator, its run's agents and its sessions. A badge counts the agents waiting for you. Hovering a project shows: new terminal, new Claude session, close.
- **Several at once.** Each project has its own proxy, context store, threads and run, so runs proceed in parallel.
- **Restore.** harnes saves the workspace to `%APPDATA%\dev.harnes.app\session.json` and brings back projects, terminals, Claude sessions (`claude --resume <id>`) and the run in progress. An agent whose output file exists is skipped, so an interrupted run continues where it stopped.
- **Switching provider.** The provider badge on a session pane moves it to Claude, Codex, Gemini or OpenCode. harnes writes the old transcript and recent thread messages to `.harnes/context/<session>-<ts>.md` and starts the new provider on that file.

## harnes-e2e

```yaml
# tests/billing.e2e.yaml
name: a member upgrades to Pro
steps:
  - open: /settings/billing
  - act: upgrade the workspace to the Pro plan            # natural language
  - expect: { target: { role: status }, contains: Pro }  # exact check
```

- Exact steps `open` `click` `fill` `press` `wait` `expect`; natural-language steps `act`.
- Locators: `role`, `name`, `label`, `text`, `placeholder`, `testid`, `css`, `exact`, `nth`.
- The first time, a Worker resolves an `act` step through the `harnes-e2e` MCP server (`e2e_prepare`, `e2e_click` / `e2e_fill` / `e2e_press`, `e2e_check`, `e2e_save`), driving the real browser. The actions go to `.harnes/e2e/cache/`; later runs replay them with no model call. If the app changes and replay breaks, the step goes back to "to resolve".
- Exit codes: 0 pass, 1 failure, 2 invalid spec or config, 3 app or browser unavailable, 5 `act` steps to resolve.
- Drives Chrome or Edge over CDP; starts the app from `app.command` when needed and stops its process tree.
- `harnes-e2e run [files] [--json]` · `harnes-e2e init --url … --command …` · `harnes-e2e mcp`

## Machine

The `machine` view polls every 2 s the files each tool already writes. It never starts, stops or messages a session.

| Source | Reads |
|---|---|
| Claude Code | `~/.claude/sessions/<pid>.json` (status, waiting reason); dead PIDs filtered out |
| Codex | `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl` (turns, token counts) |
| Gemini CLI | `~/.gemini/tmp/<project>/chats/session-*.jsonl` (status from write activity) |
| Status files | `~/.harnes/sessions/*.json`, `~/.agent-lookout/sessions/*.json`: `{"agent","status","name"?,"cwd"?,"pid"?}` |

It shows who needs you (longest wait first, one click to its pane), the last hour, every session with its provider, an event log and the health of each source. harnes' own agents are tagged.

<details>
<summary>Providers</summary>

`.harnes/providers.json` (optional):

```json
{
  "roles": { "worker": "codex", "scout": "claude" },
  "providers": { "codex": { "program": "codex", "args": ["--full-auto", "{prompt_with_role}"], "hooks": false } }
}
```

- Built in: `claude` (default, with hooks), `codex`, `gemini`, `opencode`.
- Placeholders: `{agent}`, `{settings}`, `{prompt}`, `{prompt_with_role}` (adds an instruction to follow `.claude/agents/<role>.md`).
- Non-Claude defaults are best-effort. Override their flags if a CLI changes.
</details>

<details>
<summary>Layout</summary>

```
crates/vt       libghostty-vt FFI (C shim) → frames of cells
crates/pty      ConPTY panes feeding the VT
crates/ctx      harnes-ctx MCP server + SQLite FTS5 store
crates/proxy    Anthropic proxy: compression, SSE pass-through, usage
crates/e2e      harnes-e2e: specs, CDP browser, replay cache, runner, MCP
crates/lookout  machine-wide agent sessions: adapters, events, history
crates/threads  agent threads (SQLite): runs, lots, questions, history
crates/os       OS packs: bundled zips, import, index, routing by role
crates/crew     setup, hooks, providers, crew engine, context hand-offs, JSON contracts
app/            Tauri 2 app (src-tauri) + SolidJS UI
template/       agents (orchestrator, planner, teacher, worker, reviewer, scout), settings, CLAUDE.md
docs/           the animations on this page
site/           the website (English and French) and the logo
```
</details>

## Build (Windows)

Rust (MSVC) with Visual Studio Build Tools 2022, Zig 0.16.0, Node.js via `nvm`, Chrome or Edge, and the Claude Code CLI signed in.

```powershell
# libghostty-vt, once (set GHOSTTY_VT_DIR if it lives elsewhere)
git clone --depth 1 https://github.com/ghostty-org/ghostty.git $HOME\tools\ghostty
cd $HOME\tools\ghostty; zig build -Demit-lib-vt -Doptimize=ReleaseFast -Dsimd=false

cargo test --workspace

cd app; npm install
npm run app        # builds harnes-ctx + harnes-e2e, then tauri dev
npm run app:build  # release + NSIS installer
```

## Using it

1. **+** next to *projects*, pick a folder. harnes writes the agents, settings and `.mcp.json` (`harnes-ctx`, `harnes-e2e`) and starts the proxy.
2. The first time, open a Claude session and accept the folder trust prompt.
3. Talk to the orchestrator in the chat. Follow the work in the sidebar, the `machine` view and the threads.
4. When the gate opens, read *ready for you* and the proposed commits. Test the result, then approve.

## Notes

- `harnes-ctx` keeps output out of the model context. It is not a security sandbox.
- Proxy compression is deterministic, so prompt caching keeps working.
- harnes-e2e and harnes-lookout are independent implementations inspired by TesterArmy e2e (Apache-2.0) and agent-lookout (MIT). No code from either is included.
- libghostty-vt's API still moves. `crates/vt/shim/hv.c` is the only file to adapt.
