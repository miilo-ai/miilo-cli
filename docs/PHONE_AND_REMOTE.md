# Use Miilo from a phone

Miilo observes work. Installing a widget does not install Codex, sign in to a
coding service, or grant access to another computer.

## Choose where the work runs

| Goal | Run the observer | Configure on the phone |
| --- | --- | --- |
| Observe sessions on this Linux/macOS computer | Beside the selected existing sessions | Nothing; use the local CLI or desktop plugin |
| Observe that computer from a phone | On the computer that owns the sessions | A pairing invitation, or a deployed observation endpoint and its credential |
| Run a coding agent inside the phone's Linux environment | In that environment, if supported | The agent's own installation/login, separately from Miilo |

`miilo local discover PROJECT` reads the filesystem of the machine executing it.
Running it on a phone does not discover sessions on your laptop over Tailscale.
Android/PRoot local collection is not qualified by the Linux/macOS support claim.

## Pair directly without CE

CLI 0.4.0 and widget helper 1.7.0 add explicit read-only computer pairing.
Keep the selected-session collector running, start `miilo local share` on the
computer, then create a private invitation with `miilo local pair invite`.
On the receiver, choose **Settings → Pair computer**, compare both codes and
approve on the host before selecting systems. No agent login or SSH key is copied.
See [pair computers](computer-pairing.md) for commands, private-network setup,
credential expiry, revocation and stale-state behavior. A physical phone still
needs its own tested network route.

## Use an existing deployed observation API

1. On the session-owning computer, install the matching CLI release and explicitly
   select existing sessions with `miilo local setup PROJECT`. Keep its collector
   running and verify `miilo local doctor` and `miilo local snapshot` there.
2. Obtain the observation API URL and a credential from that deployment's operator.
   The operator must already have connected the selected sessions or factory to
   that endpoint. A standalone local snapshot is a private file, not an HTTP API;
   `miilo local run` does not automatically publish it to a phone. Stop here if no
   compatible endpoint exists. Installing the SDK alone does not expose one.
3. If the endpoint is private, connect the physical phone to the appropriate
   Tailscale network or another approved route. Confirm that the phone's launcher
   Linux environment can reach it. A working browser route alone does not prove
   that environment has the same route.
4. In the Omalauncher widget, open **Set up**, add the observation API endpoint,
   enter its Miilo bearer token through the hidden prompt, and select returned
   providers. Begin read-only. Do not use the model's `/v1` endpoint here.
5. Open a selected system. Observe a natural timestamp/revision update. Empty
   work can be valid; do not create tasks just to make robots appear.

For the remote CLI, use the operator-approved `MIILO_URL` and `MIILO_TOKEN_FILE`,
then run `miilo doctor`, `miilo connections list`, and `miilo work list`.
Model chat is separate: `miilo-owl chat-setup` accepts a compatible model endpoint,
model name and optional private key file. Saving either configuration does not
prove connectivity or grant permission.

## Four different kinds of access

| Access | What it permits | What it does not permit |
| --- | --- | --- |
| Tailscale login/network policy | Reach approved devices and ports | Miilo login, model access or ordinary SSH authentication |
| Miilo endpoint credential | Read the observations that deployment allows | Codex/OpenAI sign-in or server administration |
| Model endpoint credential | Use that configured inference service | Read unrelated workflows or approve tasks |
| SSH identity | Log in as an authorized remote OS user | Automatically authorize Miilo or a coding service |

A passkey used for account sign-in is not interchangeable with an SSH private
key, a Miilo bearer token or a model API key. Tailscale SSH, when deliberately
configured by an operator, has its own access rules; it is not required for a
widget to read a reachable observation endpoint.

The phone does not need the computer's SSH private key or Codex `auth.json` to
observe through Miilo. Keep those credentials on their owning device. Local
transcript sources advertise no approval actions. Remote actions require the
provider's capability, permission and confirmation; a chat model cannot grant it.

## If you actually want Codex running on the phone

That is a separate agent installation and authentication task, not Miilo setup.
Consult [Codex authentication](https://learn.chatgpt.com/docs/auth) and verify the
CLI supports your environment. Device-code sign-in, when enabled for the account,
is completed by the user in a browser. Never ask a setup agent to search for or
copy another machine's login cache. Miilo does not guarantee Codex support on
Android/PRoot or sign users into Codex, Cursor or OpenCode.

## Why an emulator demonstration can look easier

An emulator may inherit a host network route or use an explicit forwarding rule.
A physical phone needs its own tested route; it cannot assume the host's localhost
or VPN is available. `localhost` always refers to the current environment.

Preview robots and curated provider fixtures can demonstrate layout, navigation
and isolated actions. Even a live model explanation does not make the sample
Codex-named systems authenticated Codex installations. Start a real install empty
and verify each actual source separately.

## If setup is missing something

Ask for the missing endpoint, permission or private credential-file reference.
Do not search credential stores, modify `/etc`, install a server, grant SSH access,
or start/resume agent work as an inferred fix. Read-only diagnosis is separate
from permission to alter the phone or server. Report unavailable/unauthorized
state accurately and stop rather than repeatedly retrying denied requests.
