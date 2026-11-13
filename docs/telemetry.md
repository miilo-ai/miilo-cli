# Optional operation tracing

Telemetry is off unless you supply an endpoint for an explanation request:

```sh
miilo work explain connection/work-id \
  --telemetry-endpoint http://127.0.0.1:4318/v1/traces
```

Supply the full OTLP/HTTP **protobuf** traces URL. HTTPS is required except for
numeric loopback addresses. Collector authentication is optional:

```sh
miilo work explain connection/work-id \
  --telemetry-endpoint https://collector.example/v1/traces \
  --telemetry-token-file /path/to/private-collector-token
```

The token file is separate from the Miilo API credential, limited to 4096 bytes
and restricted to its owner on Unix. Keep it in your private configuration
directory, outside project source. Collector credentials are sent only as an
authorization header, never as trace attributes. Configuration is per invocation;
Miilo does not discover OTEL environment settings, use system HTTP proxies,
follow redirects, initialize a global tracer or persist telemetry consent.

## Recorded operation

The CLI emits one `execute_tool request_explanation` span around its actual API
request and response decoding. It does not claim to perform the provider's model
inference. Model names, token usage and downstream inference durations are unknown
at this boundary and omitted. Snapshot lookup, polling and local transcript
observation do not become fabricated GenAI calls. This CLI trace is a root;
downstream trace-context propagation is not enabled by this command.

No question, explanation, work title, provider/work/session ID, endpoint address,
audio, transcript, tool payload or raw exception is included. Errors use fixed
classifications; Ctrl+C marks the in-flight request cancelled. Cancellation drops
the local request future; it does not promise to cancel remote provider work.
The resource contains only the fixed service name `miilo`. Operation timing and
fixed operation names still reveal activity to the selected collector.

## Delivery behavior

The Rust host uses an in-memory queue of 256 spans, batches of at most 64 and a
200 ms scheduling interval. Recording does not wait for network delivery. Queue
overflow drops telemetry, not work. Nothing is spooled to disk.

Each HTTP attempt has a 500 ms timeout, including a 250 ms connection timeout.
A transport failure or HTTP 429/502/503/504 gets at most one retry after 25 ms.
Permanent errors, redirects and partial rejection are not retried. Responses are
bounded to 64 KiB. Retries can duplicate spans after an ambiguous network failure;
delivery is best effort, not exactly once. A successful collector response means
receiver acceptance, not durable storage by a downstream analytics service.

Shutdown has a two-second waiting budget on a blocking host thread. The CLI warns
about incomplete telemetry without changing the explanation's exit status or JSON
output. Revoking tracing suppresses future records and unfinished guards; it
cannot recall spans already queued or submitted. Immediate process termination
can lose queued spans.

## Embedding

The `miilo-telemetry` workspace crate owns transport and provider lifetime. The
SDK owns the typed semantic mapping; observation domains remain independent of
exporter configuration. Construct and shut down `Telemetry` on blocking threads.
Call `record` with an approved SDK operation and explicit parent context for
linked agent, tool and inference spans. `stats()` reports submitted/delivered
span counts and failed batches; it carries no payload content. Finish producers
before shutdown and handle its result. The helper and voice lifecycle must wire
their own real boundaries before claiming coverage.

Run `cargo test -p miilo-telemetry` and `cargo test -p miilo-cli --test telemetry
--test telemetry_lifecycle` for decoded OTLP receiver, CLI opt-in/privacy,
credential separation, cancellation, retries, queue saturation and failure tests.
