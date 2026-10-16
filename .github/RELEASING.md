# Release workflow operation

Prepared 2026-10-16. These definitions follow the manifest-version/tag comparison used by PII-Codex and Praxa, and cargo-dist's native artifact stages. Tests must precede publication. Existing tags and release assets are not overwritten.

## Start safely

Ordinary development does not publish by default. Leave repository variable `MIILO_RELEASE_ENABLED` unset while preparing accounts. Missing publication credentials/configuration produce the exact log/summary line **No release has been made**. Invalid supplied credentials, version drift, test failures and network/service errors remain real failures.

`version.yml` checks package versions on main. Once `MIILO_RELEASE_ENABLED=true`, a newer manifest version runs qualification, creates its matching annotated `vX.Y.Z` tag, and explicitly dispatches `release.yml` at that tag. A manual version dispatch with `publish=true` is an alternative. Default manual inputs are non-publishing. No timestamp-based tag sorting, forced tag moves or publishing on every commit.

Private repositories can tag and create private GitHub Releases with the automatically provided `GITHUB_TOKEN` and job-level contents write. Do not create a personal token just for same-repository releases. Explicit workflow dispatch uses actions write because token-created tags do not trigger ordinary tag-push workflows. Organization/repository policies may still restrict these permissions.

Commit the prepared workflow/tooling changes, then bump manifests for a new candidate if the current version tag already exists. Never move an old tag to pick up new workflows. Tags used for publication must contain the prepared workflow definitions. Configure release-environment protection and branch/tag rules in GitHub; YAML names do not create reviewer rules.

## Rehearse and activate

1. Run CI with publishing disabled. Inspect source tests, workflow lint and qualification's `ready` output. A private dependency skip is not a full source qualification pass.
2. Configure only the needed read-only private-dependency credential. Public checkouts need none. Never share a provider/account runtime credential with CI.
3. Review the candidate commit/version and required checks; bump all package manifests within this repository consistently. SDK language manifests share a version; CLI Cargo/npm/lock versions share a version.
4. Create a new matching tag through the version workflow, or push an owner-reviewed tag manually. For an existing prepared tag, run `release.yml` from that exact tag with `tag=vX.Y.Z` and `publish=true`. Do not run publication from main while passing an unrelated tag.
5. Review platform archive checks and release-environment approvals. Check the resulting private/public GitHub Release before enabling downstream registries.
6. Run optional publishing workflows deliberately after accounts are configured. They skip missing credentials rather than pretending success. Each channel can publish/fail independently; No release has been made is scoped to the job/channel producing it.

GitHub releases can remain private for rehearsal. Public registries, AUR and tap updates are held for public source/artifacts; otherwise their users cannot download the packages. Public release readiness still requires actual hosted platform evidence.

## Credentials and environments

| Configuration | Where/when |
| --- | --- |
| GITHUB_TOKEN | Provided by GitHub for its own repository; no setup secret needed |
| MIILO_RELEASE_ENABLED=true | Repository variable enabling automatic manifest-driven release on main; initially unset |
| MIILO_SDK_READ_TOKEN | Optional contents-read access to private miilo-ai/miilo-sdk for CLI/CE; no credential passed to PR-controlled code |
| MIILO_WORKSPACE_READ_TOKEN | Optional contents-read access to SDK, CLI and tap for CE cross-repository acceptance; also usable as SDK-read fallback |
| HOMEBREW_TAP_TOKEN | Optional restricted tap-write credential for CLI Homebrew publishing; migrate to a short-lived GitHub App token when configured |
| AUR_SSH_PRIVATE_KEY + AUR_KNOWN_HOSTS | Optional AUR publishing identity and operator-verified host keys; protected aur environment |
| MIILO_AUR_ARM64_QUALIFIED=true | Widget AUR publication additionally requires recorded native Arch ARM64 acceptance; no one-tap store support is implied |
| NPM_TOKEN or MIILO_NPM_TRUSTED_PUBLISHER=true | Optional scoped legacy credential or configured OIDC publisher for publish-packages.yml / npm environment |
| PYPI_TOKEN or MIILO_PYPI_TRUSTED_PUBLISHER=true | SDK only; pypi environment, exact workflow identity configured on PyPI |
| CARGO_REGISTRY_TOKEN or MIILO_CRATES_TRUSTED_PUBLISHER=true | SDK only; crates environment, exact crates.io publisher identity |

Trusted-publisher variables are confirmations of completed registry setup, not automatic account provisioning. Configure npm's publisher close to launch because a new configuration has a limited first-publish window. Do not supply both a static registry credential and an OIDC configuration unnecessarily. Runtime/provider/device tokens belong in user-managed credential files, not these secrets.

Local checks: `python -m pip install -r .github/release-tools/requirements.txt`, then `python -m unittest discover -s .github/release-tools -p 'test_*.py'`. `lint_workflows.py` runs pinned Actionlint for workflow syntax/context; ShellCheck/Pyflakes remain separate checks. Linux/Windows/Omarchy/MSI installation and directory approval are not established by local macOS tests.

Sources: [GitHub token scope/events](https://docs.github.com/en/actions/concepts/security/github_token), [cargo-dist configuration](https://axodotdev.github.io/cargo-dist/book/reference/config.html), [Actionlint](https://github.com/rhysd/actionlint), and the inspected PII-Codex/Praxa release workflows. No upstream plugin directory approval is automated by these definitions.
