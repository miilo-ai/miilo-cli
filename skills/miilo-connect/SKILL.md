---
name: miilo-connect
description: Set up or diagnose read-only Miilo observation of explicitly selected existing local sessions, or connect to a user-chosen remote Miilo API. Does not launch agent work or approve provider tasks.
---

# Connect to Miilo

Inspect `miilo --version` and `miilo --help`. CLI 0.2 provides local observation
and a `miilo-local` compatibility executable. Use only commands actually present
in the installation. Use an existing binary or an available release matching the
OS/architecture; never invent a package publication or installer URL.

## Choose the device and access boundary first

Ask where the existing sessions run and where the user wants to view them.
Local discovery reads only the executing device's filesystem; running discovery
on a phone cannot inspect a laptop's sessions through Tailscale. For a remote
phone view, obtain the operator's compatible observation endpoint and credential;
keep the collector beside the original sessions. Local snapshot files are not
remotely served by default. Stop if the required transport is not provisioned.

Tailscale reachability, Miilo authentication, model authentication and SSH login
are separate. Viewing remote work does not require copying SSH private keys or
Codex auth caches, installing/signing into Codex on the phone, or modifying
`/etc`. Do not scan credential stores, probe system directories for missing
credentials, change server configuration or grant new access as an inferred fix.
Ask the user to complete any coding-service login themselves, only if they
explicitly want that agent running in this environment. Android/PRoot local
collection is not implied by qualified Linux/macOS support.

An emulator route, preview robots or Codex-named fixtures do not prove physical
phone connectivity or real agent login. Missing credentials/permissions are a
stop condition, not permission for repeated retries or disabling authentication.

## Local observation

On Linux/macOS, `miilo local discover PROJECT` lists existing Codex/Cursor session
metadata through read-only adapters. Discovery is not permission to import every
session. Let the user select sessions using `miilo local setup PROJECT`, or use
the native desktop import screen. Group folder/pattern rules and inclusion of
future matches require explicit selection. Do not start/resume agent work to
manufacture activity, scan credential stores, or substitute guessed transcript paths.

Local mode requires no CE process, Miilo API URL, Miilo account, or bearer token.
Version-2 configuration contains `mode: local`, selected sessions/groups and a
private snapshot cache path. `miilo local run` continuously observes the selection;
`run --once` performs one bounded read. `snapshot` reports current normalized
observations; `explain PROVIDER/WORK` reports source facts and missing evidence.
Stop continuous observation after a useful natural update or a bounded test.
Stopping this observer must not stop the source agent.

Native Linux import can install a user-owned observation service. Inspect its
status before starting another collector. Do not replace existing CE/remote
configuration: legacy version-1 files remain explicit remote mode. Use a separate
`MIILO_LOCAL_CONFIG` file when testing another mode. A stale cache is unavailable,
not proof that an agent is sleeping. Transcript activity does not prove execution,
task completion, permission requests or approval authority.

Node local commands delegate to the same native `miilo-local` engine; install the
native engine or set `MIILO_LOCAL_EXECUTABLE`. Windows currently supports remote
commands; do not promise native local transcript observation there.

## Optional remote observation

Only ask for an API URL when the user chooses CE, managed Miilo or another remote
deployment. Configure `MIILO_URL` and preferably `MIILO_TOKEN_FILE` from approved
environment/secret references. Do not put secret values in chat or arguments,
extract another agent's credentials, reuse provider keys as Miilo identity tokens,
or disable authentication to bypass provisioning. Legacy `MIILO_TOKEN` is supported;
never configure both methods. A tunnel grants reachability, not authorization.

Run `miilo doctor`, `connections list`, `work list`, and selected `work show` or
`work explain PROVIDER/WORK`. Use returned IDs. Empty work may be legitimate.
`miilo watch` subscribes before a fresh snapshot and reads authoritative state
again after events/reconnects. It does not blindly apply deltas to an old cache.
Remote provider registration follows that deployment's documented workflow.

## Handoff

Report binary/version, effective local/remote mode, selected sources, freshness,
proof commands and outcomes, and the exact remaining step. Separate fixture
evidence from live updates and unsupported integrations. Public CLI local sources
are read-only; control features in other Miilo clients are not CLI approval commands.
