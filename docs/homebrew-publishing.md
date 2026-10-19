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
the tap's GitHub Actions reviewer is a separate identity and its merge still follows native branch protection.

## Normal release

1. A qualified CLI tag publishes its GitHub assets.
2. The Homebrew job validates the stable tag and checks write access to a named
   review branch with a dry-run push before running installation tests.
3. It prepares the official formula, audits and installs it, exercises its tests,
   and opens/reuses `release/miilo-vVERSION` as a PR to the tap's `main`.
4. The publisher requests squash auto-merge for the exact proposed commit. The tap's
   trusted reviewer compares the formula with the prepared official release asset
   and approves only canonical, formula-only stable-version updates. It never
   checks out or executes PR code. Normal contributions still need maintainer review.
5. GitHub merges after its required checks and review pass. The publisher never
   pushes `main`, force-updates a branch or uses an admin bypass.
6. After merge, `brew update` and `brew upgrade miilo-ai/cli/miilo` receive the update.

A job reporting **auto-merge requested** has queued an update, not made the new
formula available yet. The formula becomes available when the PR reaches merged
state. Failed checks keep the PR open. A no-op run leaves an already-current formula unchanged.

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

A missing token now fails with an explicit Homebrew configuration error instead
of a successful publication skip. A present but unauthorized token fails the
preflight with the exact repository and required permissions. Neither failure
deletes or changes an existing GitHub release.

The tap's manual **Qualify CLI release import** workflow remains a read-only
qualification tool, not a second publisher.

## One-time tap settings

Enable repository auto-merge and allow GitHub Actions to create and approve pull
requests in `miilo-ai/homebrew-cli`. Keep default workflow permissions read-only.
The dedicated review job requests only Contents read and Pull requests write.
Keep required macOS/Linux tests, architecture/history checks, approving review,
stale-review dismissal and branch-update requirements intact. No administrator
bypass is used by this automation. If code-owner rules are added later, their
review requirement must also be satisfied; automation does not override them.

The reviewer accepts only `release/miilo-vVERSION` PRs within the canonical tap,
changing only `Formula/miilo.rb`, with a published stable official release and a
strictly newer version. Extra changes, forks, changed formula code or checksums,
and a changed head/base commit fail closed. Read-only historical qualification is
available through the tap's **Review official release formula** manual workflow.
