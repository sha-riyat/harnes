# Control plane

harnes keeps the run's decisions and evidence in files and SQLite inside
`.harnes/`. This page describes the contracts, where each file lives and what
the cockpit can do with it. Everything is local and provider-independent.

## Model policy

The Planner is the only role on Opus 5.5, at `medium` effort. The orchestrator,
Teacher, Workers, reviewers (including the deep-review finders and verifier)
and scouts run on Sonnet 5.5, at `high` effort. This split is fixed: `light`,
`full`, `deep` and eco weeks do not move another role to Opus. Eco only skips
deep reviews.

Claude Code resolves the `opus` and `sonnet` aliases to the latest models,
currently Opus 5.5 and Sonnet 5.5. The engine passes them as `--model` and
`--effort`; interactive orchestrator panes use the same explicit flags so an
older project-local agent definition cannot silently keep an Opus session.

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

`.harnes/.gitignore` contains `*`: run artefacts are local by default.
