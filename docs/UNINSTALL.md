# Remove Miilo software

Remove each installation using its original installation channel. Removing a UI
or binary does not purge private settings, revoke remote tokens, stop your coding
agents, or uninstall other Miilo clients.

## Before removing the collector binary

Close the Miilo integration you are removing. A terminal-owned `miilo local run`
can be stopped with Ctrl+C. For a desktop user service, inspect it first:

```sh
systemctl --user status miilo-local-desktop.service
systemctl --user cat miilo-local-desktop.service
```

Only if it is your Miilo observer and you want observation stopped for every
client using it, run:

```sh
systemctl --user disable --now miilo-local-desktop.service
```

This retains the unit definition and private configuration. It does not stop the
source coding agents. Do not remove shared binaries while another client still
uses them. Automatic unit-definition teardown is not provided by this release.

## Package channels

Homebrew on macOS/Linux:

```sh
brew uninstall miilo-ai/cli/miilo
```

Node package, only if installed globally through npm:

```sh
npm uninstall -g @miilo-ai/cli
```

For a custom npm prefix, use the same prefix as installation. Removing the Node
package leaves a separately installed native `miilo-local` engine alone.
Windows MSI users remove Miilo through Installed apps, or uninstall the original
MSI through Windows Installer. Check both miilo.exe and miilo-local.exe in that
installation, and distinguish other copies on PATH.

For a shell-installed binary or extracted archive, inspect its installation
receipt/files and remove only that installation's files. This version has no
universal shell/manual uninstaller; do not infer ownership from a basename or
recursively delete configuration directories. Report unknown leftovers for review.

## Launcher integrations

Use the removal instructions in the matching
[Omarchy plugin](https://github.com/miilo-ai/miilo-omarchy-plugin) or
[widget](https://github.com/miilo-ai/miilo-owl-widgets) checkout/release. Integration
removal preserves this CLI and shared observers unless separately stopped.

## Retained data and reinstall

Local configuration, ~/.miilo settings, legacy XDG configuration/cache stores,
credentials, migration originals, backups and copied setup skills are retained.
Use `miilo config paths` before removing the executable to inspect effective
paths, including overrides. Do not publish configuration or secret contents.
Reinstalling software should reuse retained settings. Purge is a separate,
currently manual operation requiring an explicit inventory and confirmation.
Removing a token file does not revoke its upstream credential.

The Node package CI exercises install/uninstall/reinstall in a disposable prefix,
checks its shim/package disappear, and preserves a separate native-engine file.
Homebrew/MSI checks are separate; none of these establish native desktop/phone
service teardown. Follow the native host acceptance instructions as well.
