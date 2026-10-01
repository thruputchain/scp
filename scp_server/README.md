# scpd heap profiles

On Linux, scpd uses jemalloc as its global allocator, built with profiling
support and `prof:true,prof_active:false,lg_prof_sample:19`. This allocator
replacement and profiling-capable allocator bookkeeping are unavoidable
default changes; allocation sampling remains inactive unless enabled.
Non-Linux builds retain their existing allocator and have no profiling listener.

Set `SCPD_PPROF_LISTEN=127.0.0.1:9646` to create a separate HTTP listener.
Without this variable, no profiling socket or background listener task is
created. Set `GROUNDHOG_HEAP_PROFILING=1` to activate allocation sampling
(average interval 512 KiB), even if no listener is configured. Only that exact
value enables profiling. With both variables set, `GET /debug/pprof/heap`
returns an uncompressed pprof protobuf converted from jemalloc heap_v2. Without activation,
the listener returns 404. Other paths and methods return 404.

The listener has no authentication; choose the bind address accordingly.
Profile dumping requires a writable temporary directory. Restart scpd after
changing these environment variables. Avoid overriding the inactive startup
default with `MALLOC_CONF=prof_active:true` when profiling should be off.

Run `cargo test -p scp_server --offline` to test listener opt-in and HTTP routing.
The pure Rust converter and its fixture tests run on macOS too. The real
jemalloc allocator and /proc/self/maps collection require Linux validation.

Only one dump may run at a time; concurrent heap requests receive 429 immediately.
Dumping and conversion run in spawn_blocking without a time limit. The permit
stays with the blocking job even if the connection task is cancelled. Request
reads and response writes each have a separate 10 second timeout; neither
timeout cuts off a dump. Dump files are created exclusively with random names
and mode 0600 in the temporary directory and removed on success or failure.
