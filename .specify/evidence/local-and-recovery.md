# Local and recovery evidence — macOS arm64, 2026-10-16/17

> Historical validation record. Results apply only to the revision and environment recorded below; they do not describe current release readiness.


Rust workspace: 16 tests passed, including command-level isolated import, natural
fixture append, private-data exclusion, exact-subject explanation, stale/private
file rejection, public Codex partial parsing, replacement/truncation/symlink rejection,
and actual SSE subscribe-before-bootstrap/reconnect/lost-history recovery.
Node: 12 tests passed including actual socket recovery and response bounds.
Rust workspace clippy passed with warnings denied. Packed npm installation/help,
agent plugin packaging, 15 release-policy tests and workflow lint passed.
Cargo-dist plan includes miilo and miilo-local in the Apple Silicon archive.
Pinned remote SDK source was fetched through the existing SSH identity and built;
all runtime dependencies are explicitly declared.

Live current-chat Codex read-only discovery/import/snapshot passed. No natural
update arrived during 10 seconds; do not claim that live update path was observed.
Actual Linux/systemd transactions and native Omarchy interaction remain delegated.
Windows local mode is explicitly unsupported. No release has been made.

Committed candidate bbeafb1 passed all tests after extraction with git archive into
a clean temporary directory, without private sibling repositories. Later consent
hardening is requalified in the final source checkpoint. Actual optimized macOS
archive installation passed CLI doctor and both executable versions, plus the
CLI-to-widget update/explanation/stale-source journey with CE absent. The final
digest/source checkpoint is recorded after the final rebuild. Future enrollment
now requires adapter creation metadata after server-bound consent, not merely an
unknown ID in a bounded recent-session list. Native Linux/systemd still needs
the delegated acceptance run. MSI content/gate is prepared and XML-validated;
Windows installation has not been executed on this Mac.
