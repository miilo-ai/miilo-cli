---
name: miilo-connect
description: Set up or diagnose the read-only Miilo CLI against a user-selected Miilo backend. Use for connecting an existing installation or preparing first-time observation, not for launching agent tasks or approving provider work.
---

# Connect to Miilo

Miilo CLI observes a Miilo API. It does not itself launch a Codex/Cursor harness, ingest arbitrary local transcripts, or sign into every provider. An agent's own login does not authorize Miilo access.

## Find the shortest supported path

Use an existing `miilo` installation if present. Run `miilo --version` and inspect `miilo --help` before assuming commands exist. This release supports `doctor`, `connections list`, `work list`, `work show`, `work explain`, and `watch`; it has no `login`, `setup`, provider registration or approval command.

If installation is needed, inspect this repository's README and GitHub Releases. Select the user's OS/architecture and an actually published release. Do not invent an npm publication, Homebrew formula or installer URL. Only one implementation should own the `miilo` command. For a source checkout, follow its documented build instructions.

Ask for the Miilo API endpoint only when it cannot be resolved from the user's chosen configuration. `MIILO_URL` may point to CE, a private deployment or hosted Miilo. Never substitute a vendor cloud endpoint or a developer's personal factory. A tunnel provides network access, not application authorization.

Prefer `MIILO_TOKEN_FILE` pointing to a user-only credential file. Legacy `MIILO_TOKEN` remains available; never configure both. Resolve credential configuration from a user-approved environment/secret reference. Do not request secret values in chat, print them, put them in command arguments, commit them, or reuse a provider API key as a Miilo account token. Never read another agent's credential stores to extract a token. If credential provisioning is not available, stop at that explicit blocker; do not disable authentication.

## Verify observation

With configuration supplied securely, run:

```sh
miilo doctor
miilo connections list
miilo work list
```

Use returned provider/work IDs for `miilo work show CONNECTION/WORK_ID`; do not guess IDs. A healthy API with no connections is reachable but not onboarded. An empty work list may be legitimate and is not evidence of broken credentials.

To connect a new provider, use the selected backend's supported registration flow. Cloud and CE have different registration and credential contracts; inspect that backend's documentation. Only observe resources the user selects. Do not start/resume a task just to manufacture activity.

For a selected existing workflow, inspect one current item and use bounded `miilo watch` observation if resumable streaming is available. Stop watching after a meaningful real update or a stated timeout. Report “awaiting natural activity” if no update arrives; do not claim streaming was proved by a successful health check.

## Handoff

Report the implementation/version, selected endpoint without secrets, observed connection IDs, proof commands and outcomes, any unproved streaming/provider capability, and the exact remaining user step. Preserve user-owned setup and unrelated configuration. The first CLI release is read-only; supported control actions in other Miilo clients are not CLI features.
