# Third-party notices

harnes is MIT-licensed (see `LICENSE`). This file records the external work that
is bundled, copied or used as a method reference, and the rules for keeping
attribution correct.

## Bundled: ECC — Everything Claude Code

- Upstream: <https://github.com/affaan-m/ECC> (formerly
  `affaan-m/everything-claude-code`)
- Copyright: 2026 Affaan Mustafa
- License: MIT
- Pinned revision: `90606c8315ee09cafc303515c5f33024db6b9882` (2026-10-20)
- Location: `crates/os/packs/ecc-engineering-os-v0.1.0.zip`,
  `ecc-security-os-v0.1.0.zip`, `ecc-agent-os-v0.1.0.zip`
- Scope: 80 curated skills copied verbatim (48 engineering, 9 security,
  23 agent). The full ECC catalogue is not imported.

Each pack contains the upstream `LICENSE-ECC`, a `NOTICE`, and a
`PROVENANCE.md` with the exact list of copied skills. The pack wrapper files
(`manifest.yaml`, `OS.md`, `oracle/`, `routing/`, `playbooks/`, `rules/`,
`agents/`) are original harnes work and do not relicense the ECC content.

## Method reference: Beacon

- Upstream: <https://github.com/Asymptote-Labs/agent-beacon>
- License: MIT
- Use: the review-gated memory workflow (candidate → human review → promoted
  record) and the evidence/confidence model informed the native Rust
  implementation in `crates/memory` and the `memory_*` MCP tools.
- No Beacon source code or Go binary is bundled. harnes does not require
  Beacon, and there is no runtime dependency on it.

## Method reference: ECC security guide

- Upstream: <https://github.com/affaan-m/ECC> (`the-security-guide.md`), MIT
- Use: the idea of reading a project's own Claude Code config before running it
  (the two CVEs it cites), role isolation of judges, a heartbeat for stalled
  agents, and capped memory injection informed `harnes_guards::trust`, the
  built-in isolation rules, the stall check and `Memory::context`. No text or
  code from the guide is copied.

## Used, not bundled: OpenCode

- Upstream: <https://opencode.ai> (the `opencode` CLI, installed by the user)
- Use: harnes starts it as an external program for free-model steps
  (`crates/free`). Nothing of OpenCode is bundled, and harnes does not require
  it: without it every step runs on Claude.

## Method reference: Harness 2.0

- Upstream: <https://github.com/google-research/harness2>
- Copyright: Google LLC
- License: Apache License 2.0
- Use: the harness-specification shape, component/variant comparison and
  evaluation-before-tuning method informed `crates/crew/src/harness.rs`,
  `crates/eval` and the cockpit's `tune_plan`.
- No Harness 2.0 Python source, benchmark data or harness code is copied or
  bundled. harnes implements the method independently in Rust.

## Not used: OpenUI and CopilotKit

OpenUI and CopilotKit are MIT-licensed, but harnes does not add React,
CopilotKit or OpenUI to its Tauri/SolidJS app. The run view is a native,
typed `harnes.view.v1` SolidJS view. No OpenUI or CopilotKit code is bundled.

## Other dependencies

Rust crates and npm packages keep their own licenses. The authoritative lists
are `Cargo.lock`, `app/package-lock.json` and the license files shipped by those
packages. They are not copied into this repository.

## Attribution rules for future changes

1. A file copied from another project keeps its original copyright and license
   header and is listed in this file with source path, revision and license.
2. A file adapted from another project adds a header saying `Modified by
   harnes from <project> at <revision>` and the original license. Do not remove
   the original notice.
3. A method or schema borrowed without copying code is documented here as a
   method reference; do not imply endorsement by the upstream authors.
4. Apache-2.0 material keeps a `NOTICE` with the upstream attribution. MIT
   material keeps the copyright and permission notice. A bundled binary or
   archive carries its license inside the archive as well.
5. `LICENSE`, `NOTICE`, `THIRD_PARTY.md` and pack-level `LICENSE-*`/`NOTICE`
   files must ship with every release.
