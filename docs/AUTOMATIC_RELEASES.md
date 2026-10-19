# Automatic releases

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
