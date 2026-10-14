# Miilo CLI

**See what your agents are doing without opening another dashboard.**

[![CI](https://github.com/miilo-ai/miilo-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/miilo-ai/miilo-cli/actions/workflows/ci.yml)
[![Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

Miilo CLI turns provider-neutral work, evidence, attention, and explanations into
a terminal interface. Point it at Miilo Cloud, Miilo CE, or a private deployment.
The endpoint is yours; there is no required call home.

```bash
export MIILO_URL=http://127.0.0.1:3700
export MIILO_TOKEN='short-lived-bearer-token'

miilo doctor
miilo connections list
miilo work list
miilo work show factory/task-1
miilo work explain factory/task-1 --question "Why is this blocked?"
miilo watch
```

The first release is deliberately read-only. Observation can be broadly useful.
Mutation needs stronger guarantees. Approve, reject, retry, and interrupt stay out
until revision binding, confirmation, and authorization have independent evidence.

## Install

The native Rust binary is the default:

```bash
# macOS and Linux
brew install miilo-ai/tap/miilo

# macOS or Linux without Homebrew
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/miilo-ai/miilo-cli/releases/latest/download/miilo-cli-installer.sh | sh
```

Windows releases include a PowerShell installer, MSI, and ZIP. A true Node.js
implementation is also maintained in [`packages/miilo-cli-node/`](packages/miilo-cli-node/)
for environments that standardize on Node 20.19 or newer. Both implementations
install the same `miilo` command, so install one globally.

## What it connects to

```text
miilo CLI ── HTTP + resumable SSE ── configured Miilo API ── provider adapters
```

`MIILO_URL` is authoritative. The target may live on a workstation, LAN,
Tailscale network, private cloud, or isolated environment. The CLI stores no
provider credentials and contains no analytics path. Authentication is a
caller-supplied bearer token.

The provider contract comes from [`miilo-sdk`](https://github.com/miilo-ai/miilo-sdk).
The CLI observes the normalized contract; it never executes an adapter directly.

## Repository map

| Path | Purpose |
| --- | --- |
| [`crates/miilo-client`](crates/miilo-client/) | Rust HTTP, authentication, snapshots, explanations, and resumable SSE |
| [`crates/miilo-cli`](crates/miilo-cli/) | Native Rust executable |
| [`packages/miilo-cli-node`](packages/miilo-cli-node/) | Portable Node.js implementation of the same read-only commands |
| [`packaging/aur`](packaging/aur/) | Arch Linux and Omarchy package template |

## Build it

```bash
make test
make smoke
```

Tests include both implementations, a packed npm installation, and an
authenticated user journey against a deterministic mock API. Local development
next to `miilo-sdk` uses the sibling checkout; clean releases resolve the tagged
SDK commit.

## Releases

A `v*` tag creates GitHub Release artifacts for Apple Silicon and Intel macOS,
ARM64 and x86-64 Linux, and x86-64 Windows. The release includes checksums,
provenance, shell and PowerShell installers, MSI, and a generated Homebrew
formula. The Node package is attached as a tested workflow artifact until npm
publishing is explicitly enabled.

See [`docs/RELEASE.md`](docs/RELEASE.md) before tagging.

Apache-2.0. The license does not grant rights to the Miilo name or marks.
