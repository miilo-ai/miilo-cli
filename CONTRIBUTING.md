# Contributing

Miilo CLI is a read-only client of the public Miilo HTTP and event-stream
contract. Keep provider orchestration, credentials, and product-specific policy
out of this repository.

## Before opening a pull request

1. Keep Rust and Node command behavior aligned.
2. Add a focused test for transport, parsing, or output changes.
3. Run `make test` and `make smoke`.
4. Explain compatibility and privacy effects.

New mutating commands require a separate design review covering current-revision
binding, explicit confirmation, authorization scopes, idempotency, audit receipts,
and reconciliation after conflicts.

By contributing, you agree that your contribution is licensed under Apache-2.0.
