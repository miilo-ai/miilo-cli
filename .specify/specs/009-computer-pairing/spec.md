# Read-only computer pairing

Status: implemented and validated on native clients and an Android emulator; awaiting merge and release.

## Behavior

A computer can explicitly share its already-selected local observations with another
Miilo client. Local observation continues to work without a listener or account.
Starting sharing requires a current selected snapshot and an explicit bind address,
HTTPS origin and display name. Pairing does not enroll transcripts or start agents.

Invitations contain a pinned TLS certificate and a single-use secret, expire after
five minutes and must be transferred privately. A client requests access, displays
a six-digit comparison code, and waits for the owner to approve the matching code
on the host. Approval is available only through an owner-only local Unix socket.
Each approved device receives a separate credential, expiring after 30 days.

Readers can request snapshots only. Remote actions, terminal input, model calls,
filesystem browsing and remote approvals are unavailable. Revocation blocks the
next read. A stale collector returns an unavailable response rather than a healthy
cached snapshot. Previously authorized devices survive a host restart.

## Boundaries

`miilo-pairing` owns protocol DTOs, pairing policy and ports. Files, TLS, clocks,
randomness and HTTP are adapters. `miilo-local` composes the selected snapshot
source. Widget consumers reuse the same crate's client and certificate validation.
Default TLS roots, redirects and proxies are disabled for paired connections.
Private state uses owner-only directories/files with symlink checks. This is local
file protection, not encryption at rest. No transcript or credential is logged.

## Validation

Exercise expiry, reuse, denial, wrong comparison codes, persistence failures,
independent credentials, revocation, stale-state recovery, certificate rejection,
request limits, private-file policy and unsupported remote mutations. Validate
native clients on macOS, Linux and the Android launcher environment. Report each
platform's actual evidence separately; protocol tests do not prove GUI behavior.
