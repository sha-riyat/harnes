# Control plane

harnes keeps the run's decisions and evidence in files and SQLite inside
`.harnes/`. This page describes the contracts, where each file lives and what
the cockpit can do with it. Everything is local and provider-independent.

## Model policy

The Planner is the only role on Opus 5.5, at `medium` effort. The orchestrator,
Teacher, Workers and reviewers (including the deep-review finders and
verifier) run on Sonnet 5.5, at `high` effort. Scouts, which only look things
up, run on Haiku 5.5, at `high` effort. This split is fixed: `light`, `full`,
`deep` and eco weeks do not move another role to Opus. Eco only skips deep
reviews.

Claude Code resolves the `opus` and `sonnet` aliases to the latest models,
currently Opus 5.5 and Sonnet 5.5. The engine passes them as `--model` and
`--effort`; interactive orchestrator panes use the same explicit flags so an
older project-local agent definition cannot silently keep an Opus session.
When `equip` runs on an existing project it rewrites only the `model:` and
`effort:` lines of the managed agent frontmatter and leaves the prompts intact.

## Event log and typed run view

- Schema: `harnes.event.v1`
- Storage: `.harnes/telemetry.db` (indexed, queryable) and
  `.harnes/telemetry/events.jsonl` (append-only audit trail)
- Secrets are redacted before an event is sealed. Event ids are content
  hashes, so replaying the same event is idempotent.
- Events carry `run`, `project`, `role`, `lot`, `kind`, `tool`, `path`,
  `command`, `status`, tokens, cost, `summary`, `detail`, `evidence` and a
  fidelity marker (`observed`, `inferred` or `unverified`).

The UI reads `harnes.view.v1` from `run_view`, built from
`.harnes/runs/<run>/` plus telemetry and memory. It contains:

| Card | Source |
|---|---|
| plan | `plan.json` |
| reviews | `review-<n>.json` |
| gate | `gate-<n>.json` |
| metrics | `metrics.json` |
| trace | telemetry query for the run |
| usage | `model.usage` events |
| findings | deterministic guard scan over the trace |
| decisions | `.harnes/runs/<run>/decisions.jsonl` |
| memory | memory candidates for the run |
| files | paths seen in telemetry |

The model never generates HTML for this view. It receives typed JSON only.

## HarnessSpec

`.harnes/harness.json` makes the harness inspectable and tunable:

```json
{
  "version": 1,
  "role": "worker",
  "provider": "claude",
  "components": {
    "system_prompt": ".claude/agents/worker.md",
    "guardrails": ".harnes/guards.json",
    "memory": "project",
    "tool_guidance": "mcp/harnes-ctx",
    "subagents": ["scout"],
    "skills": ["ecc-engineering-os", "builder-os"],
    "scripts": ["ctx_execute", "e2e"],
    "plugins": ["hooks"]
  },
  "budgets": { "always_on_tokens": 12000, "on_demand_tokens": 8000 },
  "provenance": "harnes default HarnessSpec v1"
}
```

`HarnessSpec::ensure` writes the default once; the cockpit's `harness_get` and
`harness_save` read and validate it. Paths must stay inside the project, budgets
must be positive, and the version must be `1`. The shape is informed by
Harness 2.0; no Python code or benchmark data is bundled (see
`THIRD_PARTY.md`).

## Guards

`.harnes/guards.json` is a small declarative policy:

```json
{
  "version": 1,
  "rules": [
    {
      "id": "deny-destructive-command",
      "title": "Commande destructive",
      "severity": "critical",
      "action": "deny",
      "tools": ["Bash", "PowerShell"],
      "paths": [],
      "pattern": "(?i)git\\s+reset\\s+--hard",
      "message": "Commande irreversible bloquée."
    }
  ]
}
```

- Actions: `allow`, `ask`, `deny`; severities: `info`, `warn`, `high`,
  `critical`.
- Rules can match tool names (`Bash`, `Write`, `mcp__*`), path globs and a
  regex over the command or serialized input. Every condition present in a
  rule must match; an empty condition is ignored. Rules are evaluated in order
  and the first matching rule gives the decision; put `deny` rules before broad
  `allow` rules. Use `**/` globs when a tool reports absolute paths.
- Before every tool call, the agent's settings run the native guard:
  `harnes __hook --root <project> --forward http://127.0.0.1:<port>/hook/<key>`.
  It decides from the file, reports the event to the cockpit, and prints a
  Claude-compatible `PreToolUse` decision. If the cockpit is down, the guard
  still works. The standalone `harnes-hook` binary uses the same code.
- `PermissionRequest` remains blocking and is answered in the UI. The broker
  records `approval.requested`, `approval.granted` and `approval.denied`
  events, so approvals appear in the trace and the denied-then-retry finding.
- Guards are advisory for tools that do not support `PreToolUse`. The
  post-hoc scan correlates events (for example a secret read followed by
  network access) and emits findings instead of pretending it blocked them.
- `equip` creates the file once and never overwrites a user's rules. Reload it
  from the cockpit with `guards_reload`.

## Worktrees

`.harnes/worktrees.json` is optional:

```json
{ "enabled": true, "base": "HEAD" }
```

- Enabled by default when the project is a git repository whose root is the
  repository top level; non-git projects, repository subdirectories and tests
  run in the project root (no silent whole-repo checkout).
- One worktree and branch per lot: `.harnes/worktrees/<run>/<lot>` on
  `harnes/<run>/<lot>`.
- The worker edits the worktree; MCP data and threads stay in the project
  root. harnes injects the project's `.claude`/`CLAUDE.md` only when the base
  revision does not have them, points `.mcp.json --root` at the project, and
  removes those generated files before committing the lot.
- When a lot finishes, harnes commits leftovers, merges with `--no-ff`, and
  removes the worktree. On conflict it aborts the merge, keeps the worktree
  and branch, and leaves the conflict for a human. It never force-removes
  dirty work or rewrites history.
- Inspect with `worktree_list`; remove a clean one with `worktree_remove`.
  A conflicted worktree stays until a human resolves it.

## Review-gated memory

- Storage: `.harnes/memory.db`
- Pipeline: a run harvests evidence-backed candidates, the reviewer proposes
  lessons, and a **human** approves or rejects them. Approval promotes a
  candidate to a record; rejection keeps the reason.
- `memory_search`, `memory_context` and `memory_list` read approved records.
  `memory_candidates`, `memory_propose` and `memory_review` are the review
  tools. `memory_review` is refused unless `HARNES_MEMORY_REVIEW=1`; writes
  can be disabled with `HARNES_MEMORY_WRITE=0`.
- The confidence score is not an approval. It is one input to the human
  decision. The method is informed by Beacon (MIT); no Beacon binary is
  required or bundled.

## Evaluation

`.harnes/evals/*.yaml` holds deterministic suites:

```yaml
name: smoke
description: build and unit tests
tasks:
  - id: build
    title: Build
    prompt: the project compiles
    timeout_secs: 360
    checks:
      - name: cargo check
        command: cargo check --workspace --all-targets
        expect_exit: 0
        timeout_secs: 300
```

- A task passes only when it has at least one check and every check passes.
- `contains` adds an expected substring; `cwd` is relative to the project.
- On Windows checks run through `powershell.exe -NoProfile`; elsewhere `sh -c`.
- Results are saved to `.harnes/evals/results/<suite>-<started_at>.json`
  (`harnes.eval.v1`) and aggregated in `.harnes/eval-ledger.json`.
- `ensure_defaults` writes a `smoke` suite on first open if none exists. The
  run view's **lancer la suite** button calls the same runner on the first
  suite (normally `smoke`).
- Evaluation is deterministic and model-free. It does not claim to measure
  everything an agent can do; it is the baseline that tuning must not break.

## Tuning

`tune_plan` creates a one-component-at-a-time sweep for a suite:

| Variant | Change |
|---|---|
| `baseline` | reference model and effort |
| `effort-high` | higher reasoning effort |
| `context-tight` | smaller context budget |
| `review-deep` | one more reviewer round |

The plan is saved under `.harnes/experiments/tune-<timestamp>.json`. Run each
variant on the same suite, save both results, then compare with
`harnes_eval::compare` (pass rate and duration); keep a change only if it wins
or is neutral and simpler. Tuning comes after the eval and worktree machinery
because it needs a stable baseline. No model is called by `tune_plan` itself.

## Trust: what a project's own config can run

Claude Code reads `.claude/settings*.json` and `.mcp.json` from the folder it
opens in. A repository can use them to redirect the API (and the key with it),
run a command through a hook or helper, or start an MCP server unasked. Two
published Claude Code CVEs were of this kind (project code ran before the trust
dialog; a project overrode `ANTHROPIC_BASE_URL`), and harnes opens arbitrary
folders and routes every agent through its own proxy, so it reads that config
before anything starts (`harnes_guards::trust`).

| Finding | Severity | Why |
|---|---|---|
| `redirects-the-api` | critical (remote URL) / high (loopback) | `ANTHROPIC_BASE_URL` and the like in `env`: requests, and the key, go elsewhere or skip the proxy |
| `sets-credentials` | high | the project picks the account |
| `runs-a-helper` | high | `apiKeyHelper`, `awsAuthRefresh`… run a command at start |
| `runs-a-hook` | high | a project hook that is not harnes' own runs outside the guards |
| `auto-starts-mcp-servers` | high | `enableAllProjectMcpServers` is on and `.mcp.json` declares servers beyond `harnes-*` |
| `bypasses-permissions` | high | `defaultMode: bypassPermissions` |
| `other-mcp-server` | warn | informational |

High and critical findings stop every launch (a crew run, the orchestrator, a
Claude session) until the user has read them and accepted them in the usage
tab. Accepting is tied to a hash of the file: any later change asks again
(`.harnes/trust.json`). harnes' own template is clean, and since this version
it sets `enableAllProjectMcpServers` to `false`; its two servers are listed in
`enabledMcpjsonServers`.

## Role isolation

A judge must not be tuned to its candidate. `guards.json` rules can name
`roles`, taken from the agent's key (`worker`, `reviewer`, `reviewer-finder`,
`reviewer-verifier`, `teacher-bar`, `teacher-gate`). Four built-in rules come
after the user's own (turn them off with `"builtins": false`):

- workers cannot read `bar.json`, `review-*.json` or `gate-*.json` (Read, Grep,
  Glob, or a shell command that names them);
- the finders of a deep review cannot read each other's verdicts; the verifier
  can read them all.

Harness2 saw an agent fetch the rubric it was graded on. This closes the same
door.

## Liveness

- **Heartbeat.** Every hook event is a sign of life. An agent that reports
  `working` and sends nothing for a third of the agent timeout (20 minutes at
  most) is stopped and its step fails, instead of waiting out the whole hour.
  An agent waiting for the user is not stalled. Providers without hooks are
  only timed out.
- **Closing a pane** kills the agent and everything it started (`taskkill /T`
  on Windows, the process group elsewhere). On Windows the pseudo-console
  already ends most children: this covers the ones that detach.

## Turns

Every event of an agent session carries a `turn` id, opened by each user prompt
(Beacon's `prompt.id`). `Filter.turn` returns one whole turn: the prompt, every
tool call, every file and every approval until the next prompt.

## Harness fingerprint and budgets

- `harnes_crew::harness::fingerprint` hashes the spec, the role prompts, the
  guard rules and `CLAUDE.md`. Every run stores it in `metrics.json` and in
  `harness.fp`. If the harness changes while a run is interrupted, the resumed
  run says so and is marked `harness_changed`: its numbers mix two harnesses and
  should not be compared.
- `always_on_tokens` in `harness.json` is checked at each agent start against
  what the agent loads before it does anything (its prompt, `CLAUDE.md` and what
  that imports, about 4 bytes to a token). Going over it is a `budget.exceeded`
  event; `harness_get` returns both numbers.

## Typed questions (`ask_user`)

An agent that needs a decision that is the user's no longer prints a menu for
harnes to read off the screen. It calls the `ask_user` MCP tool with one of three
registered kinds, and the cockpit draws a card:

| Kind | Fields | Answer |
|---|---|---|
| `choice` | `question`, 2 to 8 `options` (`id`, `label`, `detail?`) | the option's `id` |
| `confirm` | `question` | `yes` or `no` |
| `text` | `question` | free text, 2,000 characters at most |

Common fields: `default` (what stands if nobody answers) and `timeout_secs` (5 to
540, 300 by default). The result is `{answered: true, choice, text}` or
`{answered: false, reason: timeout | declined | cancelled, default}`.

The model describes what it needs, never the interface:

- a kind that is not registered, an unknown field, too many options, a duplicate
  id or an over-long text is refused with a message the agent can read and fix;
- the card is drawn as text, never as markup;
- the answer is checked against the card (an option that does not exist, a blank
  text, a `maybe` for a confirm) and a bad one leaves the question open;
- an agent has one question open at a time, and 20 may be open over every agent;
- while it waits, the agent shows as blocked on the user, not as working;
- closing the agent's pane cancels its question; the user can also skip it.

Events: `question.asked`, `question.answered` (a choice is kept, a free-text
answer is not, only its length) and `question.declined`. The route is
`POST /ask/<agent key>` on the hook server; every agent now gets `HARNES_API` in
its environment, and the tool only appears for agents launched by harnes.

Screen reading stays as the fallback for questions that come from an agent's own
terminal UI (a Claude Code menu, a `(y/n)`). The idea of a closed registry of
typed components is the one OpenUI and CopilotKit share; harnes takes the idea
and none of their code or React.

## Free models (OpenCode)

A free model proposes, a paid one checks. The profile, per project in
`.harnes/profile.json` (the usage tab sets it), decides who may see the code:

| Profile | What goes to a free model |
|---|---|
| `claude` (default) | nothing: no third party sees the project |
| `mixed` | the finders of a deep review; the first try of small tasks (light runs) |
| `max-eco` | also the first try of every worker whose lot is not sensitive |

`mixed` becomes `max-eco` when the week runs ahead of its limits and
`auto_escalate` is on; a project is never moved off `claude` without the user.

What is never routed: the planner, the Teacher, the verifier, the orchestrator,
a retry after a review, a run the Teacher rated high risk, and any lot whose
files look sensitive (credentials, `.github/`, auth, payment, migrations, keys).
A free worker needs a worktree, so its mistakes stay on a branch a Claude
reviewer reads. A free finder only reads, and harnes proves it: the working tree
(git status and diff) must be the same after as before, or its answer is
discarded.

Fallback is the rule: no OpenCode, a model that is not offered, an error, a
silence of 15 minutes, a bad edit: the step runs again on Claude, so a free
model can cost time but never a result. Each case is a `route.free` or
`route.fallback` event.

Only models whose provider keeps nothing and does not train are allowed by
default (`glm-5.3-flash-free`, `deepseek-v4.1-flash-free`, `space-bunny-free`,
`longcat-2.5-preview-free`, per OpenCode Zen's own page). Others may keep or
reuse what they get; they run only if the user adds them to `models`. harnes
asks `opencode models opencode` and uses the intersection, so a withdrawn model
is simply skipped.

`ctx_digest` (MCP, harnes-ctx) gives any agent a short summary of a large
output, stored id or file, written by a free model when the profile allows it
(`digest: true`); the full text stays one `ctx_retrieve` away. Secrets are
masked before anything leaves, sensitive files are refused, and any failure
returns the deterministic head, tail and error lines. Under `claude` it never
sends anything out.

The prompt goes to OpenCode in a file, never on a command line (an npm shim runs
through `cmd.exe`, where quoting cannot be trusted). `equip` writes the project's
`opencode.json` with the two harnes MCP servers and refuses `webfetch`, keeping
the user's own keys; lot worktrees get a copy, never committed.

Limits, said plainly: the project's guards run through Claude Code hooks, which
OpenCode does not have, so a free agent runs without them. That is why it only
gets work that is isolated (a worktree, or read-only and proven so) and verified
afterwards. This path was built against OpenCode's documented CLI and tested
with a stand-in binary; it has not run against a real OpenCode install.

## Experience library

`harnes_eval::passk` makes a harness change measurable:

- `run_k` runs a suite `k` times (1 to 10): pass@1 (the average), pass^k (all `k`
  pass) and the flaky tasks. One run says little about a flaky check.
- `decide` keeps a change only if it wins, or ties and is simpler, and never if
  a task that always passed now fails.
- `record` appends the verdict, with both fingerprints and the numbers, to
  `.harnes/experiments/library.jsonl`. `tune_plan` lists the variants already
  rejected on the same model, so a sweep does not pay for them twice.

harnes does not run Harness2's Propose-Probe-Compose loop: each probe is a real
agent run on a real task, and that cost should be a deliberate act, not a
default. These pieces are what make its results trustworthy when you do.

## ECC packs

`crates/os/packs/ecc-*.zip` adds three curated method packs:

| Pack | Skills | Role |
|---|---|---|
| `ecc-engineering-os` | 48 | workers and reviewers |
| `ecc-security-os` | 9 | reviewers and supply-chain work |
| `ecc-agent-os` | 23 | orchestrator and planner |

Each pack contains `OS.md`, `oracle/ORACLE.md`, `routing/intents.yaml`,
`playbooks/`, `rules/`, `agents/`, `PROVENANCE.md`, `NOTICE`, `LICENSE-ECC`
and the selected upstream skill directories. The skills are copied verbatim
from ECC at commit `90606c8315ee09cafc303515c5f33024db6b9882`; the wrappers are
harnes originals. Not all 311 ECC skills are imported.

## Files

| Path | Purpose |
|---|---|
| `.harnes/harness.json` | HarnessSpec v1 |
| `.harnes/guards.json` | runtime guard rules |
| `.harnes/worktrees.json` | worktree enable/base |
| `.harnes/worktrees/<run>/<lot>` | isolated lot checkout |
| `.harnes/memory.db` | candidates and approved memory |
| `.harnes/telemetry.db` | indexed event log |
| `.harnes/telemetry/events.jsonl` | append-only event trail |
| `.harnes/runs/<run>/` | plan, bar, reports, reviews, gate, decisions |
| `.harnes/evals/*.yaml` | eval suites |
| `.harnes/evals/results/` | eval results |
| `.harnes/eval-ledger.json` | cumulative eval counters |
| `.harnes/experiments/` | tune plans and comparisons |
| `.harnes/context/` | provider hand-offs |
| `.harnes/profile.json` | models profile: `claude`, `mixed` or `max-eco`, allowed free models |
| `.harnes/trust.json` | project-config findings the user accepted, by file hash |
| `.harnes/experiments/library.jsonl` | what was tried, with fingerprints and numbers |
| `.harnes/runs/<run>/harness.fp` | the harness the run started with |
| `opencode.json` | harnes MCP servers for OpenCode, written when a profile uses it |
| `question.*` events in `telemetry.db` | typed questions and what became of them |

`.harnes/.gitignore` contains `*`: run artefacts are local by default.
