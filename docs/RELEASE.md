# Release process

Follow [automatic releases](AUTOMATIC_RELEASES.md). Bump consistent package manifests
on a reviewed feature branch for a new release. Once merged, qualification creates
and publishes the matching immutable tag. Documentation-only changes need no bump.

GitHub Releases distribute native binaries and the Node package archive. npm registry
publication is separate and must not be implied by a GitHub release. Verify actual
assets and checksums before advertising a new installation target.

Homebrew currently needs a reviewed formula PR into the protected tap; automatic
release success does not prove that the formula was updated. Test macOS and Linux
formula installation separately. AUR publication also remains a separate process.

See [release status](release-status.md) for remaining validation scope.
