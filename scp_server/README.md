# scpd

## Heap profiles

Heap profiling is opt-in at build time and at run time:

- Build on Linux with `--features heap-profiling` to use jemalloc as the global
  allocator, with `prof:true,prof_active:false,lg_prof_sample:19` (512 KiB
  average sampling interval, inactive until enabled). Default builds, including
  release builds, keep the system allocator.
- Set `GROUNDHOG_HEAP_PROFILING=1` to activate sampling at startup. Only that
  exact value enables it. On a build without the feature, or off Linux, scpd
  fails startup rather than ignoring it.
- Set `SCPD_PPROF_LISTEN=127.0.0.1:9646` to open a separate HTTP listener.
  Without it no profiling socket or task exists. The listener has no
  authentication; use a private address.
- `GET /debug/pprof/heap` returns an uncompressed pprof protobuf converted from
  jemalloc `heap_v2`. Without activation, and for other paths and methods, the
  listener returns 404.
- Profiles are symbolized in process: function names, plus file and line when
  debuginfo is present, innermost inlined frame first. Mappings carry GNU build
  IDs.
- One dump runs at a time; a concurrent request gets 429. Dump and conversion
  run on a blocking worker without a time limit, and the job keeps its permit
  if the connection is cancelled. Request reads and response writes each have
  a 10 second timeout.
- Dumps need a writable temporary directory. Dump files are created with
  random names and mode 0600 and removed afterwards.

Restart scpd after changing these environment variables. Do not set
`MALLOC_CONF=prof_active:true` when profiling should be off.

Tests: `cargo test -p scp_server` covers the converter and HTTP routing on any
platform. On Linux, `cargo test -p scp_server --features heap-profiling` also
takes a real jemalloc dump and checks that a retained allocation resolves by
function name and that the test executable's mapping has a build ID.

## Tracing

scpd exports its `tracing` spans over OTLP when `OTEL_EXPORTER_OTLP_ENDPOINT`
is set. Restart scpd after changing these variables.

- `OTEL_EXPORTER_OTLP_ENDPOINT`: enables export. OTLP over gRPC only
  (collector port 4317), plain `http://` only; an `https://` endpoint disables
  tracing with a logged error.
- `OTEL_EXPORTER_OTLP_PROTOCOL`: only `grpc`. Any other value is ignored with a
  logged error.
- `OTEL_SDK_DISABLED=true`: no exporter or provider is built.
- `OTEL_TRACES_SAMPLER` / `OTEL_TRACES_SAMPLER_ARG`: default
  `parentbased_traceidratio` at `0.05`. An unknown sampler, or a ratio sampler
  with a missing or invalid argument, uses that default and logs an error.
- `GROUNDHOG_NETWORK_HASH`: set by chuckd; 64 hex characters representing the
  32-byte network hash. When set, slot spans join the block trace whose ID is
  derived from the network hash and slot index. Sampling uses that trace ID
  with the configured ratio so scpd and chuckd keep or drop the same blocks.
  Missing or invalid values log one error and leave independent traces.
- `OTEL_SPAN_FILTER`: which spans are exported, in `EnvFilter` syntax
  (for example `warn,scp_kernel=info`). Default `info`. It is independent of
  `RUST_LOG`, which only controls the log output.
- `OTEL_SERVICE_NAME` (default `scpd`) and `OTEL_RESOURCE_ATTRIBUTES`. The
  resource also carries `service.version` and `service.instance.id` (the
  node's hex ed25519 public key).

An unreachable collector logs one error and scpd carries on without exported
spans. On SIGINT or SIGTERM scpd flushes pending spans for up to 5 seconds; a
second signal exits immediately.
