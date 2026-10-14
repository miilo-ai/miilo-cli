# Miilo CLI

Miilo CLI is a private-by-default observer for automated work. It connects to a
managed or self-hosted Miilo control plane and makes agent activity, evidence,
attention, and grounded explanations available from a terminal without moving
provider credentials into the CLI.

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

The first release is deliberately read-only. Approve, reject, retry, interrupt,
and other mutating commands remain outside the CLI until supervised-action gates
and revision binding have independent evidence.

## Install

The native Rust binary is the recommended distribution:

```bash
# macOS and Linux after the first release
brew install miilo-ai/tap/miilo

# Any supported macOS or Linux host
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/miilo-ai/miilo-cli/releases/latest/download/miilo-cli-installer.sh | sh
```

Windows receives a PowerShell installer, MSI, and ZIP from GitHub Releases. A
portable Node.js implementation provides the same command surface on Node 20.19+
and can be installed from `@miilo-ai/cli` once the npm package is published.

## Private deployments

`MIILO_URL` is the authority. The CLI has no required connection to Miilo Cloud
and contains no analytics or call-home path. It can connect to Miilo CE on a
workstation, LAN, Tailscale network, private cloud, or air-gapped environment.
Authentication is a caller-supplied short-lived bearer token.

```text
miilo CLI -> configured Miilo API -> customer providers and models
```

Provider contracts come from the separately versioned
[`miilo-sdk`](https://github.com/miilo-ai/miilo-sdk). The CLI never executes a
provider adapter directly.

## Implementations

| Path | Purpose |
| --- | --- |
| `crates/miilo-client` | Rust HTTP, authentication, snapshot, explanation, and resumable SSE transport |
| `crates/miilo-cli` | Native Rust `miilo` executable |
| `packages/miilo-cli-node` | Portable Node.js `miilo` executable with the same read-only commands |

Install only one implementation globally because both intentionally expose the
same `miilo` command.

## Development

```bash
make test
make smoke
```

During local development next to the SDK repository, `make test-rust-local`
uses `../miilo-sdk/rust`. Normal clean checkouts resolve the tagged SDK from
GitHub.

## Release channels

A `v*` tag drives native archives for Apple Silicon and Intel macOS, x86-64 and
ARM64 Linux, and x86-64 Windows. The release also produces shell, PowerShell,
MSI, checksum, and Homebrew artifacts. The Node implementation is built and
packed independently so it remains a real JavaScript client rather than a binary
downloader.

Licensed under Apache-2.0. The license does not grant trademark rights.
