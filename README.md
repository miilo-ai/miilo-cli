# Miilo CLI

**See what your agents are doing without opening another dashboard.**

[![CI](https://github.com/miilo-ai/miilo-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/miilo-ai/miilo-cli/actions/workflows/ci.yml)
[![Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

Miilo CLI observes selected existing sessions locally, or connects to an optional
self-hosted/managed Miilo API. Standalone observation needs no account, CE process,
Miilo URL, or listening HTTP service.

```bash
miilo local discover /path/to/project
miilo local setup /path/to/project
miilo local run
# In another terminal:
miilo local doctor
miilo local snapshot
miilo local explain PROVIDER/WORK_ID
```

Candidate 0.2 supports local Codex/Cursor observation on Linux/macOS. Discovery
does not start or resume work; import is explicit. Bootstrap history appears idle.
Only sanitized assistant summaries/tool names are projected, not user messages,
reasoning or tool inputs/outputs. This is activity observation, not authoritative
execution or approval detection. The CLI remains read-only. Windows supports the
remote command set; native local file observation is not yet qualified there.

For remote CE/managed observation, explicitly configure `MIILO_URL` and a private
`MIILO_TOKEN_FILE`, then use `doctor`, `connections list`, `work list/show/explain`
and `watch`. Watching subscribes before its fresh snapshot; events invalidate
state and reconnects always obtain a fresh baseline.

## Install

The native Rust binary is the default:

```bash
# macOS and Linux
brew tap miilo-ai/tap https://github.com/miilo-ai/miilo-homebrew-tap.git
brew install miilo-ai/tap/miilo

# macOS or Linux without Homebrew
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/miilo-ai/miilo-cli/releases/latest/download/miilo-cli-installer.sh | sh
```

Windows releases include a PowerShell installer, MSI, and ZIP. A true Node.js
implementation is also maintained in [`packages/miilo-cli-node/`](packages/miilo-cli-node/)
for environments that standardize on Node 20.19 or newer. Both implementations
install the same `miilo` command, so install one globally.

## Let your agent help

The [miilo-connect skill](skills/miilo-connect/SKILL.md) guides a terminal-capable
assistant through version checks, explicit local selection or an optional
authenticated remote connection. It never extracts another agent's credentials
or starts work to populate a demonstration. Candidate source is not a published
release; installation URLs below work only after owner-approved publication.

## Runtime and state

The public `miilo-local` library contains small read-only discovery/transcript
adapters. `miilo` invokes it directly; `miilo-local` is a compatibility executable
for native widgets. The Node CLI delegates local commands to that same executable
(PATH or `MIILO_LOCAL_EXECUTABLE`); remote commands remain implemented in Node.

Selected configuration lives at `$XDG_CONFIG_HOME/miilo/local.json` (or
`~/.config/miilo/local.json`), overridable with `MIILO_LOCAL_CONFIG`. Version 2
stores `mode: local`, `snapshotFile`, sessions and groups, with no URL/token.
The atomic owner-only runtime cache defaults to `$XDG_STATE_HOME/miilo/snapshot.json`
or `~/.local/state/miilo/snapshot.json`. It is not durable history. A cache older
than 15 seconds is rejected. One collector per configuration is allowed.
Native Linux import owns `miilo-local-desktop.service`; terminal `run` stops on
Ctrl+C without touching agents. Existing version-1 CE configurations stay explicit
remote publication mode and are not silently migrated or overwritten.

The [SDK runtime](https://github.com/miilo-ai/miilo-sdk) owns provider isolation,
validation, aggregation, deduplication and bounded event history. CE consumes the
same core. No private private-application checkout is required. The committed SDK
dependency pins a full source revision; private repository access is required
until the owner makes these repositories public.

## Repository map

| Path | Purpose |
| --- | --- |
| [`crates/miilo-client`](crates/miilo-client/) | Rust HTTP, authentication, snapshots, explanations, and resumable SSE |
| [`crates/miilo-cli`](crates/miilo-cli/) | Native `miilo` and `miilo-local` executables |
| [`crates/miilo-local`](crates/miilo-local/) | Standalone local session adapters and composition |
| [`packages/miilo-cli-node`](packages/miilo-cli-node/) | Portable Node.js implementation of the same read-only commands |
| [`packaging/aur`](packaging/aur/) | Arch Linux and Omarchy package template |

## Build it

```bash
make test
make smoke
```

Tests include both implementations, a packed npm installation, and an
authenticated user journey against a deterministic mock API. Local development
next to `miilo-sdk` uses the sibling checkout; clean releases resolve the pinned
SDK commit.

## Releases

A `v*` tag creates GitHub Release artifacts for Apple Silicon and Intel macOS,
ARM64 and x86-64 Linux, and x86-64 Windows. The release includes checksums,
provenance, shell and PowerShell installers, MSI, and a generated Homebrew
formula. The Node package is attached as a tested workflow artifact until npm
publishing is explicitly enabled.

See [`docs/RELEASE.md`](docs/RELEASE.md) before tagging.

Apache-2.0. The license does not grant rights to the Miilo name or marks.

## Credential files

Prefer `MIILO_TOKEN_FILE=/path/to/private/token` or `--token-file /path/to/private/token` over token contents in arguments. Rust and Node accept the same protected-file configuration and reject credential-bearing endpoint URLs and redirects. Legacy `MIILO_TOKEN` remains supported; do not set both methods. Restart a running client after token rotation. See [CE credential operations](https://github.com/miilo-ai/miilo-ce/blob/main/docs/CREDENTIALS.md) for provisioning, rotation, transport and current single-operator limits.

For the release-to-VM checklist, see [Omarchy acceptance](docs/OMARCHY.md). Commands that depend on public assets remain gated until those assets exist.

## Release automation

See [.github/RELEASING.md](.github/RELEASING.md) for private-repository rehearsals, manifest-driven tags, credential-missing skips and activation. Publication stays disabled until explicitly requested/configured.

`watch --json` in candidate 0.2 emits NDJSON records with `type: snapshot`,
`reason: bootstrap|event`, and an authoritative `snapshot`. This replaces the
old raw-event watch output. Library consumers needing raw resumable events can
still call `MiiloClient.stream`; `observe` provides reconciled snapshots.
