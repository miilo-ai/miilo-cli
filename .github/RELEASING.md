# Release workflow operation

See [automatic releases](../docs/AUTOMATIC_RELEASES.md) for the current workflow.
A new manifest version on main triggers qualification and publication; an unchanged
version does not. `MIILO_RELEASE_ENABLED=false` is an emergency pause, not a required opt-in.
Never move published tags or replace release assets. Validate the actual release
and its assets after a run. Registry credentials and store approvals are separate.

See [release status](../docs/release-status.md) for known distribution limitations.
