# GitHub release to Omarchy acceptance

Prepared 2026-10-16. These are release-candidate steps, not a statement that public packages exist. No publication, accounts, tags or deployment were performed by writing this runbook.

## Before the first candidate

- [ ] Decide public alpha versions and document their supported capabilities.
- [ ] Review licenses, package contents, secret scans and pending changes in each repository independently.
- [ ] Make the SDK readable and publish the reviewed SDK tag first. CLI and CE must resolve that exact public Git tag from a clean checkout without sibling overrides or personal credentials.
- [ ] Rerun SDK, CLI, CE and tap Actions. Local macOS checks are not the Linux/Windows release matrix.
- [ ] Rehearse the pinned cargo-dist version from the repository configuration: `dist plan`, generated workflow review and target builds. Preserve the custom Node package job. `make test-node` checks the CLI workflow's tap target against its manifest, but does not replace this rehearsal.
- [ ] Configure the CLI's `HOMEBREW_TAP_TOKEN` only when ready to enable cross-repository publishing. See the credential guide for least privilege. No registry accounts are needed for GitHub-only assets.
- [ ] Approve and publish reviewed CE/CLI candidate tags; verify checksums, provenance and exact release assets. The Node `.tgz` is included by the existing custom artifact/release job; verify it is actually attached before documenting it as downloadable.
- [ ] Inspect the generated `miilo` formula in `miilo-ai/miilo-homebrew-tap`; the pre-release tap currently contains no installable formula.

Publishing stays an explicit owner-controlled step. A prerelease is not a stable support claim for untested targets.

## First Omarchy journey: terminal, then integration

The current widget repository implements Android Omalauncher OWL widgets. It does not contain a Waybar/Hyprland desktop widget. The first Omarchy acceptance target is the Rust CLI and CE in a terminal. Desktop robots are additional implementation work, not something the Android widget installer provides.

1. Boot the Omarchy VM, record OS version and `uname -m`, and take a VM snapshot.
2. Download a pinned GitHub candidate's Linux archive and its checksum manifest through the official release page. Match x86_64 versus aarch64; use the exact filenames shown in that release rather than guessing an installer URL.
3. Verify SHA-256 against the published manifest before extracting. Where attestations are emitted, verify them with current GitHub CLI `gh release verify` / `gh release verify-asset`; a checksum fetched beside an archive is integrity evidence, not independent provenance. Do not silently accept a failed attestation.
4. Install the verified `miilo` and `miilo-ce` executables into a user-owned directory such as `~/.local/bin` and confirm that directory is on PATH. No root install is required for this path.
5. Follow [CE credentials](https://github.com/miilo-ai/miilo-ce/blob/main/docs/CREDENTIALS.md); run both programs inside the VM on `127.0.0.1:3700` initially. No Miilo Cloud account is required.
6. Run `miilo doctor`, `miilo connections list`, `miilo work list`, `miilo work show PROVIDER/WORK_ID`, `miilo work explain PROVIDER/WORK_ID`, and `miilo watch` once observations are ingested. Record actual provider/work IDs; placeholders are not seeded data.
7. Register two supported real adapters, observe an existing session and a natural update, and inspect real evidence. Synthetic fixtures prove protocol behavior only. CE needs adapters to push normalized observations; installing the CLI does not automatically discover every harness.
8. Interrupt networking, reconnect the stream, rotate its token, restart the client, and remove one provider while preserving the other. CE restart currently loses all connections/observations; demonstrate that explicitly rather than promising durability.

Remote Tower access comes after loopback acceptance. Verify guest routing and TLS/encrypted tunnel independently; host Tailscale membership alone does not enroll the guest. Do not put tokens in endpoint URLs.

## Alternative distribution paths

Homebrew uses a custom repository name, so the explicit URL matters:

```sh
brew tap miilo-ai/tap https://github.com/miilo-ai/miilo-homebrew-tap.git
brew install miilo-ai/tap/miilo
```

Run only after the generated formula exists and points at verified release assets. Test Linuxbrew separately from macOS Homebrew. Homebrew is not a Windows installer; Windows needs its own ZIP/MSI/PowerShell artifact acceptance.

For Node, download the actual CLI `.tgz` release asset and install that local file with npm on a supported Node version from `packages/miilo-cli-node/package.json`. No npm registry publication is implied. Choose Rust or Node as the active `miilo` command; avoid two global installations silently competing on PATH.

For Arch packaging, render `miilo-cli/packaging/aur/PKGBUILD.in` using the actual version and published Linux checksums, then inspect it, run `makepkg --verifysource`, build and install inside the disposable VM. There is no published AUR package yet; do not advertise `yay -S` as working until it exists and passes this journey.

Omalauncher acceptance is a separate Android/Linux environment using the OWL package and its own verified setup. `~/.local/bin` is an executable install location; `/root/.config` in that environment is launcher configuration. They serve different purposes.

## Evidence to capture

| Proof | Required record | Current state |
| --- | --- | --- |
| Local SDK contracts + package payloads | Commands, platform, working-source diff | Locally passed; not a public install |
| CE credential lifecycle and disconnect | Rotation/rejection/stream close; two-provider isolation | Locally passed with disposable credentials/fixtures |
| Rust/Node client parity | Installed package command, HTTP/SSE journey | Local macOS source/package proof |
| Public dependency resolution | Clean CI checkout and exact readable SDK tag | Pending public availability |
| Linux/Omarchy release install | Archive version/digest, OS/arch, terminal journey | Pending release and VM acceptance |
| Homebrew | Generated formula, audit, macOS + Linux install | Pending release |
| Windows | Artifact + ACL checks + client journey | Pending platform acceptance |
| Real providers | Existing-session updates and actual evidence from two providers | Pending CE live-provider journey |
| Actions | Confirmed revision and exactly one provider receipt | CE has no action proxy; advertise observation-only |

Record the checkout SHA and any dirty diff for local evidence; record the release tag/digest for installed artifacts. Never publish tokens, private payloads or identifying hostnames in demo evidence.

References: [Homebrew taps](https://docs.brew.sh/Taps.html), [cargo-dist configuration](https://axodotdev.github.io/cargo-dist/book/reference/config.html), [GitHub release download](https://cli.github.com/manual/gh_release_download), [release asset verification](https://cli.github.com/manual/gh_release_verify-asset).

Research update — 2026-10-16: Omarchy Quattro has a native plugin contract, and the upstream widget store now includes an OWL/Quickshell Linux runtime. Neither is implemented or qualified by this Miilo terminal runbook. The hosted Omarchy registry still documents client/deployment launch work as incomplete. Treat desktop UI reuse and marketplace publication as separate acceptance tracks.
