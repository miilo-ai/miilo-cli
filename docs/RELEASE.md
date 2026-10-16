# Release process

## One-time repository setup

- Make `miilo-sdk`, `miilo-cli`, and `miilo-homebrew-tap` publicly readable before the
  first public release.
- Add a fine-grained `HOMEBREW_TAP_TOKEN` Actions secret to `miilo-cli`. Scope it
  only to `miilo-ai/miilo-homebrew-tap` with repository contents read/write access.
- Configure npm trusted publishing for `@miilo-ai/cli` before enabling registry
  publication. The checked-in workflow deliberately packs an artifact without
  publishing it.

## Every release

1. Confirm the SDK tag referenced by `Cargo.lock` is publicly readable.
2. Run `make test` and `make smoke` on a clean checkout.
3. Push a `vX.Y.Z` tag. `dist` builds native macOS, Linux, and Windows artifacts,
   installers, checksums, attestations, MSI, and the Homebrew formula.
4. Review the generated pull request or commit in `miilo-ai/miilo-homebrew-tap`.
5. Pack and inspect `packages/miilo-cli-node`; publish it separately with npm
   provenance after configuring a trusted publisher.
6. Substitute the release version and checksums into `packaging/aur/PKGBUILD.in`,
   run `makepkg --verifysource`, then test installation in the Omarchy VM.

Do not publish from a dirty tree. Do not put registry, GitHub, or Miilo tokens in
repository secrets when OIDC or a trusted publisher is available.

## Current release gate — 2026-10-16

`make test-node` checks that the generated/custom release workflow and Cargo metadata both target `miilo-ai/miilo-homebrew-tap` and that the formula name is `miilo`. This catches the stale repository destination found during local qualification. It does not replace running the pinned cargo-dist `dist plan`, reviewing regenerated workflows (including the custom Node packaging job), or installing an actual published formula.

Local Rust/Node tests and installed Node package checks passed. Public SDK-tag resolution, GitHub-hosted release jobs, generated formula publication and Linux/Windows install acceptance remain open. No npm account is required for GitHub `.tgz` distribution; the current host job attaches the custom Node artifact alongside the other release artifacts. Verify the actual candidate release assets before advertising installation commands.

Credential-file setup, rotation and the tap publishing secret are documented in [CE credential operations](https://github.com/miilo-ai/miilo-ce/blob/main/docs/CREDENTIALS.md). Windows ACL qualification and per-user/scoped CE identity are separate future gates.
