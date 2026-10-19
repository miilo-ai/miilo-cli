# Homebrew formula publication

The CLI GitHub release and the tap update are distinct deliverables. Native CLI
archives may already be published even if a later tap step fails. Never retag,
replace a release, weaken branch rules or bump the CLI version just to retry a
formula update.

## Credential

Create a fine-grained personal access token with resource owner `miilo-ai`, access
only to `homebrew-cli`, and repository permissions **Contents: Read and write**
and **Pull requests: Read and write**. Metadata read access is implicit. Ensure
any required organization approval is complete; pending approval does not grant
write access. Set an appropriate expiry and renew it before expiration.

Store it as `HOMEBREW_TAP_TOKEN` in the CLI repository's `homebrew` environment
secrets. A repository or organization secret of that name is also supported when
accessible, but an environment secret takes precedence. Do not paste token values
in issues, logs, chat or Git remote URLs. Do not reuse a broad developer token.
The CLI repository's ordinary `GITHUB_TOKEN` cannot write a separate tap repository.

The token needs no branch-rule bypass or permission to change workflow files.
A GitHub App installed only on the tap, with equivalent repository permissions,
is an alternative for a dedicated bot identity. The PR author is the token owner;
review/merge must follow the tap's rules and reviewer policy.

## Normal release

1. A qualified CLI tag publishes its GitHub assets.
2. The Homebrew job validates the stable tag and checks write access to a named
   review branch with a dry-run push before running installation tests.
3. It prepares the official formula, audits and installs it, exercises its tests,
   and opens/reuses `release/miilo-vVERSION` as a PR to the tap's `main`.
4. The tap's required macOS/Linux checks and review run normally. The workflow
   never pushes `main`, force-updates a branch, approves or merges its own PR.
5. After merge, `brew update` and `brew upgrade miilo-ai/cli/miilo` receive the update.

A job reporting **PR opened** has proposed an update, not made the new formula
available yet. A no-op run leaves an already-current formula unchanged.

## Retry an already-published version

After the tap has the PR-publishing helper and the CLI workflow change is merged,
run this from an authorized maintainer shell:

```sh
gh workflow run publish-homebrew.yml --repo miilo-ai/miilo-cli --ref main -f tag=v0.3.1
```

The manual entry downloads the existing release formula; it does not rebuild,
republish or move tags. Matching open PRs are reused. A differing existing branch
is refused for review instead of overwritten. Old release runs use their old
workflow revision; rerunning one does not pick up this fix.

If no token is configured, optional publication is skipped with the established
`No release has been made` receipt and a clarification that existing GitHub
releases are unchanged. A present but unauthorized token fails the preflight
with the exact repository and required permissions.

The tap's manual **Qualify CLI release import** workflow remains a read-only
qualification tool, not a second publisher.
