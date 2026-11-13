# Automatic releases

Every PR runs the same read-only artifact workflow used by publication: native
archives for all configured targets, Windows MSI install/removal, global
installers, and Node/agent payloads. The existing qualification check fails if
any rehearsal build fails, is cancelled or is skipped. Main version bumps also
require this rehearsal before creating a tag. Build artifacts remain available
on the Actions run for inspection; rehearsals never publish releases, submit
Homebrew PRs or request attestation credentials.

Release publication consumes that shared build workflow and separately performs
attestation, upload and Homebrew submission under the protected release path.

A merge into main compares the package version with existing version tags. A new
version runs qualification, creates an immutable matching tag, and explicitly
dispatches the release workflow on that tag. A normal merge with an already tagged
version makes no release. Versions are not bumped automatically.

The repository variable MIILO_RELEASE_ENABLED=false pauses automatic publication.
No variable is required to enable it. Manual dispatch with publish=true can retry
explicitly. Missing required credentials/dependencies skip publication with
“No release has been made”; validation or authorization errors fail the run.
The release environment may still require its configured reviewer approval.

Check the published release and assets after a run; a green qualification or
skipped-publication run alone is not evidence of publication. Optional external
registries and Homebrew retain their separate credential requirements.
