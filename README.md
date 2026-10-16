# harnes

harnes is a desktop cockpit for running a crew of coding agents. It is built with Rust and Tauri 2, with a SolidJS front end. The terminal panes are rendered by **libghostty-vt**, Ghostty's VT engine.

Claude Code is the default runtime. Each role can also run on Codex, Gemini CLI or OpenCode.

## Components

| Need | harnes component | Crate |
|---|---|---|
| Agent terminals (replaces Herdr) | ConPTY panes rendered by libghostty-vt, with agent state from Claude Code hooks | `harnes-vt`, `harnes-pty` |
| Token savings (replaces Headroom) | Local Anthropic proxy: deterministic `tool_result` compression and usage stats | `harnes-proxy` |
| Context sandbox (replaces Context Mode) | MCP server `harnes-ctx`: command runs, SQLite FTS5 index, retrieval | `harnes-ctx` |
| End-to-end tests (inspired by TesterArmy e2e) | `harnes-e2e`: YAML specs, natural-language `act` steps resolved by the agents themselves, model-free replay | `harnes-e2e` |
| Seeing every agent (inspired by agent-lookout) | Machine-wide view of Claude Code, Codex, Gemini CLI and status-file sessions | `harnes-lookout` |
| Crew orchestration | Planner → Teacher → Workers → Reviewer → Teacher gate | `harnes-crew` |

Ponytail, Caveman and TasteSkill v2 (`design-taste-frontend`) remain Claude Code skills.

## The crew

| Role | Default model | Job | Output |
|---|---|---|---|
| Orchestrator | Opus, medium | Your only interlocutor. Turns your messages into crew runs (`crew_start`), follows every hand-off in the threads, checks that sub-agents' results were integrated, and reports back. | its replies |
| Planner | Opus, medium | Splits the task into lots and waves. Lots in the same wave never share files. | `plan.json` |
| Teacher | Opus, medium | Sets the validation bar before any work starts: criteria, the evidence each one needs, and when to involve you. | `bar.json` |
| Worker × N | Sonnet, high | Implements one lot, then writes and runs its own e2e test. | `reports/<lot>-r<n>.json` |
| Reviewer | Opus, medium | Reviews the diff against the plan and the real e2e results. | `review-<n>.json` |
| Teacher (gate) | Opus, medium | Checks the bar against the tester's results, then either sends the work back or tells you it is ready. | `gate-<n>.json` |
| scout | Haiku, low | Subagent any role can start for short parallel tasks: finding code, summarising files, running a command. | its reply |

```
plan ─► bar ─► waves of workers ─► [e2e suite] ─► review ─┬─ CHANGES ─► fix workers ─┐
                                                          └─ PASS ─► teacher gate ──┤
                                                               ready ─► you   not ready ─► fix workers
```

- You are only interrupted once the Teacher's bar is met. The Teacher fixed that bar before the work started, and the test results back it up.
- Commits are proposed and never created automatically.
- Each agent runs in a visible pane. The agents hand work to each other through JSON files in `.harnes/runs/<id>/`.
- Claude Code agents report their lifecycle through hooks. Agents from other providers are tracked by watching their output file.

## Threads

The agents, their sub-agents, the orchestrator and you communicate through threads stored in SQLite (`.harnes/threads.db`). The file is shared by every agent process, and nothing runs in the cloud.

- **Thread layout**: each run gets a `run-<id>` thread, and each lot gets a `lot-<id>-<lot>` thread, seeded with the lot's brief.
- **What the engine posts**: every step of the run, plus the Reviewer's requested fixes in the matching lot thread.
- **What the agents do**: Workers read their lot thread before starting, ask their questions there, and summarise their work there before finishing.
- **Tools and signatures**: agents use `thread_list`, `thread_read`, `thread_post`, `thread_create` and `thread_close` (in the `harnes-ctx` MCP server). Messages are signed automatically (`HARNES_AGENT`). Sub-agents add `as`, which shows as `worker-L1/scout`.
- **History**: threads are closed when the Teacher's gate opens, but stay readable. Posting again reopens a thread.
- **UI**: the **Threads** tab lists the open and closed threads and shows each conversation. You can write in any thread. An ASCII strip shows the latest messages travelling between agents.

Why SQLite rather than a hosted backend such as Convex: everything runs on one machine and works offline, there is no extra service, and your conversations never leave your computer.

## Projects

harnes works project by project, like Herdr workspaces or the projects in Claude Desktop and ChatGPT.

- **Sidebar**: lists every open project.
  - The active project is expanded and shows its orchestrator, its run's agents and its sessions.
  - A spinner means a run is in progress; a badge counts the agents waiting for you.
  - Hovering a project shows three buttons: new terminal, new Claude session, and close.
- **Adding a project**: **+** opens the native Windows folder picker. The project is then equipped automatically.
- **Several projects at once**: each project has its own proxy, context store, threads and crew run, so runs in different projects can proceed in parallel.
- **Switching**: the centre area and the right-hand panel show only the active project. Switch with a click in the sidebar, the project menu in the top bar, or `Ctrl K`.
- **Restore**: every open project comes back at the next launch, with its sessions and unfinished run, and the active project is restored too.

## Restore and provider switching

- **Automatic restore**: harnes saves the workspace continuously to `%APPDATA%\dev.harnes.app\session.json`. On the next launch it restores:
  - the project;
  - the terminals;
  - the Claude sessions, through `claude --resume <session id>`, with the id captured by the hooks;
  - the run in progress.
- **Resumable runs**: an agent whose output file already exists is skipped, so an interrupted run continues where it stopped.
- **Provider switching without losing context**: the provider badge in a session pane's header switches the session to Claude, Codex, Gemini or OpenCode. harnes then:
  - turns the old session's transcript into a hand-off file in `.harnes/context/<session>-<ts>.md`, along with the recent thread messages;
  - starts the new provider with the instruction to read that file first.

  The chain continues across successive switches, and non-Claude sessions are restored from their last hand-off.

## harnes-e2e

```yaml
# tests/billing.e2e.yaml
name: a member upgrades to Pro
steps:
  - open: /settings/billing
  - act: upgrade the workspace to the Pro plan            # natural language
  - expect: { target: { role: status }, contains: Pro }  # exact check
```

- **Step types**:
  - exact steps: `open`, `click`, `fill`, `press`, `wait`, `expect`;
  - natural-language steps: `act`.
- **Locators**: `role`, `name`, `label`, `text`, `placeholder`, `testid`, `css`, `exact`, `nth`.
- **Resolving an `act` step**:
  - the first time, a Worker resolves it through the `harnes-e2e` MCP server: `e2e_prepare`, then `e2e_click` / `e2e_fill` / `e2e_press`, then `e2e_check` and `e2e_save`;
  - the agent drives the real browser, so the model is the Claude Code session itself, signed in normally;
  - the recorded actions go to `.harnes/e2e/cache/`, which you can commit, and later runs replay them with no model call;
  - if the app changes and the replay breaks, the step goes back to "to resolve".
- **Exit codes**: 0 pass, 1 failure, 2 invalid spec or config, 3 app or browser unavailable, 5 `act` steps still to resolve.
- **Browser**: harnes-e2e drives Chrome or Edge (or any Chromium) over CDP. It starts the app from `app.command` when needed and stops it, process tree included.
- **CLI**: `harnes-e2e run [files] [--json]`, `harnes-e2e init --url … --command …`, `harnes-e2e mcp`.

## Seeing every agent (harnes-lookout)

The **Tous les agents** tab polls every 2 s the files each tool already writes. It never starts, stops or messages a session.

| Source | What it reads |
|---|---|
| Claude Code | `~/.claude/sessions/<pid>.json` (status, waiting reason); dead PIDs are filtered out |
| Codex | `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl` (turns, token counts) |
| Gemini CLI | `~/.gemini/tmp/<project>/chats/session-*.jsonl` (status inferred from write activity) |
| Status files | `~/.harnes/sessions/*.json` and `~/.agent-lookout/sessions/*.json`: `{"agent","status","name"?,"cwd"?,"pid"?}` |

The tab shows:
- a **Te concernent** panel, with the longest-waiting session first and a click to jump to its pane;
- a chart of the last hour;
- the session list with provider badges;
- an event log;
- the health of each source.

harnes' own agents are tagged in the list.

## Providers

`.harnes/providers.json` (optional):

```json
{
  "roles": { "worker": "codex", "scout": "claude" },
  "providers": { "codex": { "program": "codex", "args": ["--full-auto", "{prompt_with_role}"], "hooks": false } }
}
```

- Built-in providers: `claude` (default, with hooks), `codex`, `gemini`, `opencode`.
- Placeholders: `{agent}`, `{settings}`, `{prompt}`, `{prompt_with_role}`. The last one adds an instruction to follow `.claude/agents/<role>.md`.
- The non-Claude defaults are best-effort. Override their flags if a CLI changes.

## Layout

```
crates/vt       libghostty-vt FFI (C shim) → frames of cells
crates/pty      ConPTY panes feeding the VT
crates/ctx      harnes-ctx MCP server + SQLite FTS5 store
crates/proxy    Anthropic proxy: compression, SSE pass-through, usage
crates/e2e      harnes-e2e: specs, CDP browser, replay cache, runner, MCP
crates/lookout  machine-wide agent sessions: adapters, events, history
crates/threads  agent threads (SQLite): runs, lots, questions, history
crates/crew     setup, hooks, providers, crew engine, context hand-offs, JSON contracts
app/            Tauri 2 app (src-tauri) + SolidJS UI
template/       agents (orchestrator, planner, teacher, worker, reviewer, scout), settings, CLAUDE.md
```

## Build (Windows)

Prerequisites:
- Rust (MSVC) and Visual Studio Build Tools 2022
- Zig 0.16.0
- Node.js via `nvm`
- Chrome or Edge
- Claude Code CLI, signed in

1. Build libghostty-vt once:

   ```powershell
   git clone --depth 1 https://github.com/ghostty-org/ghostty.git $HOME\tools\ghostty
   cd $HOME\tools\ghostty; zig build -Demit-lib-vt -Doptimize=ReleaseFast -Dsimd=false
   ```

   Set `GHOSTTY_VT_DIR` if it lives elsewhere.

2. Run the tests:

   ```powershell
   cargo test --workspace
   ```

3. Start the app:

   ```powershell
   cd app; npm install
   npm run app        # builds harnes-ctx + harnes-e2e, then tauri dev
   npm run app:build  # release + NSIS installer
   ```

## Using it

1. Click **+** next to *Projets* and pick a folder. This writes the agents, settings and `.mcp.json` (`harnes-ctx`, `harnes-e2e`), and starts the proxy.
2. The first time, open a **+ Claude** pane and accept the folder trust prompt.
3. Talk to the orchestrator in the bottom box. It starts and supervises the crew. Follow the work in **Panes**, **Tous les agents** and the **Threads** tab.
4. When the Teacher's gate opens, read **Prêt pour toi** and the proposed commits. Test the result yourself, then approve.

## Notes

- `harnes-ctx` isolates output from the model context. It is not a security sandbox.
- Proxy compression is deterministic, so prompt caching keeps working.
- harnes-e2e and harnes-lookout are independent implementations inspired by TesterArmy e2e (Apache-2.0) and agent-lookout (MIT). No code from either project is included.
- libghostty-vt's API is still evolving. `crates/vt/shim/hv.c` is the only file to adapt when you update it.
