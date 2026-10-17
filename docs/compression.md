# Local compression and verified restore

No server/account is required. Both `miilo` and `miilo-local archive` use the
SDK's optional gzip/Brotli/zstd adapters. The Node CLI delegates to `miilo-local`
through its existing explicit native-helper boundary; set MIILO_LOCAL_EXECUTABLE
when it is not on PATH. Node-only remote commands remain independent of it.

```sh
miilo archive export selected-history.jsonl history.miilo --codec zstd
miilo archive inspect history.miilo
miilo archive restore history.miilo restored-history.jsonl
miilo archive compress gzip response.json response.json.gz
miilo archive decompress gzip response.json.gz restored-response.json
```

`export` defaults to zstd level 3; gzip level 6 and Brotli quality 5 are explicit
alternatives. `inspect` fully decodes and verifies both hashes/lengths. Bundle
commands hold bounded payloads in memory; raw compress/decompress commands stream
through fixed copy buffers and codec windows. The default input/output cap is
64 MiB, configurable with `--max-bytes N` (1 to 268435456). Bundle header overhead
is separate from its encoded payload cap. All output uses a private temporary file
(0600 on Unix), flushed before atomic no-clobber publication. Existing paths and
symlinks are not overwritten. Failed restore never publishes a partial output.

Select a regular input file explicitly. No command discovers/enrolls transcripts,
embeds original paths, changes provider state, uploads data or executes restored
content. Raw gzip/Brotli/zstd outputs are interoperable standard streams, but use
the Miilo bundle when you need SHA-256 checked exact restore. Hashes do not prove
who sent a bundle. Compression is not encryption, redaction or token compaction.

Remote Rust clients now negotiate/decode gzip/Brotli/zstd responses while applying
size limits to decoded JSON. Node/browser clients negotiate encodings supported
by their fetch runtime. Request-body compression is not enabled. Live event/speech
streams retain the server's conservative identity policy.

Validation: compiled local CLI tests (all three codecs, exact restore, corruption,
no-clobber, output limits), Rust HTTP decoder tests, and `npm run test:native` in
the Node package after `cargo build --workspace`. CI runs the native integration
after building the helper, and skips SDK-dependent qualification honestly when
private SDK access is unavailable. Linux/macOS/Windows release artifact execution
remains part of the native release matrix, not evidence from one host.
