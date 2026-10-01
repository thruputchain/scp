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
returns the gzipped pprof protobuf produced by `dump_pprof`. Without activation,
the listener returns 404. Other paths and methods return 404.

The listener has no authentication; choose the bind address accordingly.
Profile dumping requires a writable temporary directory. Restart scpd after
changing these environment variables. Avoid overriding the inactive startup
default with `MALLOC_CONF=prof_active:true` when profiling should be off.

Run `cargo test -p scp_server --offline` to test listener opt-in and HTTP routing.
The real jemalloc allocator and profile conversion require Linux validation.
