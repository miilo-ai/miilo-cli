# Install on Omarchy

Install the native CLI from GitHub Releases or Homebrew, then follow
[local observation](../README.md) to select existing sessions. No CE server or
Miilo account is required for local observation.

```sh
brew tap miilo-ai/cli
brew install miilo-ai/cli/miilo
miilo --version
```

For manual installation, select the release archive matching the host architecture,
verify its published checksums and install in a user-owned PATH directory.
The separate [Omarchy plugin](https://github.com/miilo-ai/miilo-omarchy-plugin)
provides the native desktop robots. The widgets repository supplies its helper
and the separate Android Omalauncher experience.

For phone viewing and remote factories, read [phone and remote setup](PHONE_AND_REMOTE.md).
Tailscale routing, provider credentials and local session selection are separate.
Do not expose a local collector publicly or assume phone installation imports another device's sessions.

AUR installation is not advertised until an actual package is published and tested.
See [release status](release-status.md) and [automatic releases](AUTOMATIC_RELEASES.md).
