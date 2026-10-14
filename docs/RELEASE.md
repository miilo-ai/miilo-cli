# Release process

## One-time repository setup

- Make `miilo-sdk`, `miilo-cli`, and `homebrew-tap` publicly readable before the
  first public release.
- Add a fine-grained `HOMEBREW_TAP_TOKEN` Actions secret to `miilo-cli`. Scope it
  only to `miilo-ai/homebrew-tap` with repository contents read/write access.
- Configure npm trusted publishing for `@miilo-ai/cli` before enabling registry
  publication. The checked-in workflow deliberately packs an artifact without
  publishing it.

## Every release

1. Confirm the SDK tag referenced by `Cargo.lock` is publicly readable.
2. Run `make test` and `make smoke` on a clean checkout.
3. Push a `vX.Y.Z` tag. `dist` builds native macOS, Linux, and Windows artifacts,
   installers, checksums, attestations, MSI, and the Homebrew formula.
4. Review the generated pull request or commit in `miilo-ai/homebrew-tap`.
5. Pack and inspect `packages/miilo-cli-node`; publish it separately with npm
   provenance after configuring a trusted publisher.
6. Substitute the release version and checksums into `packaging/aur/PKGBUILD.in`,
   run `makepkg --verifysource`, then test installation in the Omarchy VM.

Do not publish from a dirty tree. Do not put registry, GitHub, or Miilo tokens in
repository secrets when OIDC or a trusted publisher is available.
