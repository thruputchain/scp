//! Tracing is enabled by OTEL_EXPORTER_OTLP_ENDPOINT (OTLP over gRPC,
//! typically port 4317; http only). OTEL_EXPORTER_OTLP_PROTOCOL supports only grpc.
//! OTEL_SDK_DISABLED disables tracing. OTEL_TRACES_SAMPLER / OTEL_TRACES_SAMPLER_ARG
//! default to parentbased_traceidratio 0.05. OTEL_SPAN_FILTER defaults to info,
//! independently of RUST_LOG. Slot spans (target scpd::slot) are always exported
//! so correlation holds; OTEL_SPAN_FILTER controls the rest (subject to sampling).
//! OTEL_SERVICE_NAME and OTEL_RESOURCE_ATTRIBUTES
//! configure the service resource.
//! GROUNDHOG_NETWORK_HASH (set by chuckd, 64 hex characters) correlates slot
//! spans with block traces and samples their deterministic IDs at the configured
//! ratio. Missing or invalid hashes log one error and leave independent traces.

use opentelemetry::trace::{
    SpanContext, SpanKind, TraceContextExt, TraceFlags, TraceId, TraceState,
};
use opentelemetry_sdk::trace::{
    IdGenerator, RandomIdGenerator, SamplingDecision, ShouldSample,
};
use sha2::{Digest, Sha256};
use std::ffi::OsStr;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tracing_opentelemetry::OpenTelemetrySpanExt;

use opentelemetry_sdk::Resource;
use opentelemetry_sdk::error::OTelSdkResult;
use opentelemetry_sdk::resource::EnvResourceDetector;
use opentelemetry_sdk::trace::{
    Sampler, SdkTracerProvider, SpanData, SpanExporter,
};

struct SlotTraces {
    network_hash: [u8; 32],
    root_sampler: Box<dyn ShouldSample>,
}

static SLOT_TRACES: OnceLock<SlotTraces> = OnceLock::new();

pub fn block_trace_id(network_hash: &[u8; 32], height: u64) -> TraceId {
    let mut hash = Sha256::new();
    hash.update(b"groundhog.block-trace.v1\0");
    hash.update(network_hash);
    hash.update(height.to_be_bytes());
    trace_id_from_digest(hash.finalize().into())
}

fn trace_id_from_digest(digest: [u8; 32]) -> TraceId {
    let mut bytes: [u8; 16] = digest[..16].try_into().unwrap();
    if bytes == [0; 16] {
        bytes[15] = 1;
    }
    TraceId::from_bytes(bytes)
}

fn parse_network_hash(value: Option<&OsStr>) -> Result<[u8; 32], String> {
    let value = value.filter(|value| !value.is_empty()).ok_or_else(||
        "GROUNDHOG_NETWORK_HASH is not set; slot spans are not correlated with chuckd block traces".to_owned())?;
    let value = value
        .to_str()
        .filter(|value| value.len() == 64)
        .ok_or_else(|| {
            "GROUNDHOG_NETWORK_HASH must contain exactly 64 hex characters"
                .to_owned()
        })?;
    let mut hash = [0; 32];
    for (byte, pair) in hash.iter_mut().zip(value.as_bytes().chunks_exact(2)) {
        let digit = |byte: u8| (byte as char).to_digit(16);
        *byte = digit(pair[0])
            .zip(digit(pair[1]))
            .map(|(hi, lo)| ((hi << 4) | lo) as u8)
            .ok_or_else(|| {
                "GROUNDHOG_NETWORK_HASH contains non-hex characters".to_owned()
            })?;
    }
    Ok(hash)
}

fn configure_slot_traces(sampler: &Sampler) -> Result<(), String> {
    let network_hash = parse_network_hash(
        std::env::var_os("GROUNDHOG_NETWORK_HASH").as_deref(),
    )?;
    let root_sampler = match sampler {
        Sampler::ParentBased(root) => root.clone(),
        sampler => Box::new(sampler.clone()),
    };
    // A process builds its tracing provider once; retain the first configuration.
    let _ = SLOT_TRACES.set(SlotTraces {
        network_hash,
        root_sampler,
    });
    Ok(())
}

/// Link a freshly created span BEFORE its first enter consumes the OTel builder.
pub fn link_slot(span: &tracing::Span, slot_index: u64) {
    let Some(traces) = SLOT_TRACES.get().filter(|_| !span.is_disabled()) else {
        return;
    };
    let trace_id = block_trace_id(&traces.network_hash, slot_index);
    if opentelemetry::Context::current()
        .span()
        .span_context()
        .trace_id()
        == trace_id
    {
        return;
    }
    let sampled = traces
        .root_sampler
        .should_sample(None, trace_id, "slot", &SpanKind::Internal, &[], &[])
        .decision
        == SamplingDecision::RecordAndSample;
    let context = SpanContext::new(
        trace_id,
        RandomIdGenerator::default().new_span_id(),
        if sampled {
            TraceFlags::SAMPLED
        } else {
            TraceFlags::default()
        },
        true,
        TraceState::default(),
    );
    // A log-only subscriber can enable the span without an OTel layer. Report
    // failures at debug level; callers must still link before entering.
    if let Err(error) = span.set_parent(
        opentelemetry::Context::new().with_remote_span_context(context),
    ) {
        tracing::debug!(%error, "Could not link slot span to block trace");
    }
}

pub fn slot_span(slot_index: u64) -> tracing::Span {
    let height = i64::try_from(slot_index).unwrap_or(i64::MAX);
    let span = tracing::info_span!(
        target: "scpd::slot",
        "slot",
        slot_index = height,
        "groundhog.block.height" = height
    );
    link_slot(&span, slot_index);
    span
}

fn enabled(endpoint: Option<&OsStr>, disabled: Option<&OsStr>) -> bool {
    !disabled.is_some_and(|value| value.eq_ignore_ascii_case("true"))
        && endpoint.is_some_and(|value| !value.is_empty())
}

fn unsupported_endpoint(endpoint: Option<&OsStr>) -> Option<String> {
    let endpoint = endpoint.and_then(OsStr::to_str)?;
    endpoint.to_ascii_lowercase().starts_with("https://").then(|| {
        format!("OTEL_EXPORTER_OTLP_ENDPOINT {endpoint:?} uses https, which this build does not support; tracing is disabled")
    })
}

fn sampler_override(
    value: Option<&OsStr>,
    arg: Option<&OsStr>,
) -> (Sampler, Option<String>) {
    let value = value.filter(|value| !value.is_empty());
    match value.and_then(OsStr::to_str) {
        Some(name @ ("traceidratio" | "parentbased_traceidratio")) => {
            if let Some(ratio) = arg.and_then(OsStr::to_str)
                .and_then(|arg| arg.parse::<f64>().ok())
                .filter(|ratio| ratio.is_finite() && (0.0..=1.0).contains(ratio))
            {
                let sampler = Sampler::TraceIdRatioBased(ratio);
                return (if name == "traceidratio" { sampler } else {
                    Sampler::ParentBased(Box::new(sampler))
                }, None);
            }
            (Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(0.05))),
                Some(format!("Invalid or missing OTEL_TRACES_SAMPLER_ARG {arg:?}; using parentbased_traceidratio with ratio 0.05")))
        }
        Some("always_on") => (Sampler::AlwaysOn, None),
        Some("always_off") => (Sampler::AlwaysOff, None),
        Some("parentbased_always_on") => (Sampler::ParentBased(Box::new(Sampler::AlwaysOn)), None),
        Some("parentbased_always_off") => (Sampler::ParentBased(Box::new(Sampler::AlwaysOff)), None),
        _ => (
            Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(0.05))),
            value.map(|value| format!(
                "Invalid OTEL_TRACES_SAMPLER {value:?}; using parentbased_traceidratio with ratio 0.05"
            )),
        ),
    }
}

pub fn provider(
    instance_id: Option<String>,
) -> (Option<SdkTracerProvider>, Vec<String>) {
    let mut warnings = Vec::new();
    if !enabled(
        std::env::var_os("OTEL_EXPORTER_OTLP_ENDPOINT").as_deref(),
        std::env::var_os("OTEL_SDK_DISABLED").as_deref(),
    ) {
        return (None, warnings);
    }
    // This build has no TLS features, so refuse https here instead of failing inside tonic.
    if let Some(warning) = unsupported_endpoint(
        std::env::var_os("OTEL_EXPORTER_OTLP_ENDPOINT").as_deref(),
    ) {
        return (None, vec![warning]);
    }
    let (sampler, warning) = sampler_override(
        std::env::var_os("OTEL_TRACES_SAMPLER").as_deref(),
        std::env::var_os("OTEL_TRACES_SAMPLER_ARG").as_deref(),
    );
    warnings.extend(warning);
    for name in [
        "OTEL_EXPORTER_OTLP_PROTOCOL",
        "OTEL_EXPORTER_OTLP_TRACES_PROTOCOL",
    ] {
        warnings
            .extend(protocol_warning(name, std::env::var_os(name).as_deref()));
    }
    // The builder's default config also reads the sampler env vars; its
    // warnings are compiled out with the SDK's internal-logs feature.
    let builder = SdkTracerProvider::builder().with_sampler(sampler.clone());
    opentelemetry::global::set_text_map_propagator(
        opentelemetry_sdk::propagation::TraceContextPropagator::new(),
    );
    let exporter = match opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .build()
    {
        Ok(exporter) => exporter,
        Err(error) => {
            warnings.push(error.to_string());
            return (None, warnings);
        }
    };
    let provider = builder
        .with_resource(resource(instance_id))
        .with_batch_exporter(BestEffortExporter {
            inner: exporter,
            warned: AtomicBool::new(false),
        })
        .build();
    warnings.extend(configure_slot_traces(&sampler).err());
    (Some(provider), warnings)
}

fn protocol_warning(name: &str, value: Option<&OsStr>) -> Option<String> {
    value
        .filter(|value| !value.is_empty() && *value != OsStr::new("grpc"))
        .map(|value| {
            format!(
                "{name} {value:?} is ignored; gRPC (default port 4317) is used"
            )
        })
}

fn resource(instance_id: Option<String>) -> Resource {
    let mut resource = Resource::builder_empty()
        .with_service_name("scpd")
        .with_attribute(opentelemetry::KeyValue::new(
            "service.version",
            env!("CARGO_PKG_VERSION"),
        ));
    if let Some(instance_id) = instance_id.filter(|value| !value.is_empty()) {
        resource = resource.with_attribute(opentelemetry::KeyValue::new(
            "service.instance.id",
            instance_id,
        ));
    }
    // Detected attributes override earlier ones; OTEL_SERVICE_NAME goes last
    // because the OTel spec gives it precedence over OTEL_RESOURCE_ATTRIBUTES.
    resource = resource.with_detector(Box::new(EnvResourceDetector::new()));
    if let Ok(name) = std::env::var("OTEL_SERVICE_NAME")
        && !name.is_empty()
    {
        resource = resource.with_service_name(name);
    }
    resource.build()
}

#[derive(Debug)]
struct BestEffortExporter {
    inner: opentelemetry_otlp::SpanExporter,
    warned: AtomicBool,
}

impl SpanExporter for BestEffortExporter {
    async fn export(&self, batch: Vec<SpanData>) -> OTelSdkResult {
        let result = self.inner.export(batch).await;
        if let Err(error) = &result
            && !self.warned.swap(true, Ordering::Relaxed)
        {
            tracing::error!(%error, "OpenTelemetry span export failed; collector may be unreachable, continuing without exported spans");
        }
        Ok(())
    }

    fn set_resource(&mut self, resource: &Resource) {
        self.inner.set_resource(resource);
    }

    fn shutdown_with_timeout(&self, timeout: Duration) -> OTelSdkResult {
        self.inner.shutdown_with_timeout(timeout)
    }
}

pub fn span_filter() -> (tracing_subscriber::EnvFilter, Option<String>) {
    parse_span_filter(std::env::var_os("OTEL_SPAN_FILTER").as_deref())
}

fn parse_span_filter(
    value: Option<&OsStr>,
) -> (tracing_subscriber::EnvFilter, Option<String>) {
    use tracing_subscriber::EnvFilter;
    let value = value.filter(|value| !value.is_empty());
    let (filter, warning) = match value {
        None => (EnvFilter::new("info"), None),
        Some(value) => match value
            .to_str()
            .and_then(|value| EnvFilter::try_new(value).ok())
        {
            Some(filter) => (filter, None),
            None => (
                EnvFilter::new("info"),
                Some(format!("Invalid OTEL_SPAN_FILTER {value:?}; using info")),
            ),
        },
    };
    // Linking spans need OTel data even when the user's filter excludes scpd.
    (
        filter.add_directive("scpd::slot=info".parse().unwrap()),
        warning,
    )
}

pub async fn flush_unless_interrupted(
    flush: impl std::future::Future<Output = ()>,
    termination: impl std::future::Future<Output = ()>,
) -> bool {
    tokio::select! {
        _ = flush => true,
        _ = termination => false,
    }
}

pub async fn run_until_termination(
    startup: impl std::future::Future<Output = ()>,
    termination: impl std::future::Future<Output = ()>,
) {
    tokio::pin!(termination);
    tokio::select! {
        _ = startup => termination.await,
        _ = &mut termination => {},
    }
}

pub fn termination_signal() -> impl std::future::Future<Output = ()> {
    #[cfg(unix)]
    {
        let mut terminate = tokio::signal::unix::signal(
            tokio::signal::unix::SignalKind::terminate(),
        )
        .expect("install SIGTERM handler");
        let mut interrupt = tokio::signal::unix::signal(
            tokio::signal::unix::SignalKind::interrupt(),
        )
        .expect("install SIGINT handler");
        async move {
            tokio::select! {
                _ = interrupt.recv() => {},
                _ = terminate.recv() => {},
            }
        }
    }
    #[cfg(not(unix))]
    async {
        let _ = tokio::signal::ctrl_c().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_trace_id_vector_and_zero_fixup() {
        assert_eq!(
            block_trace_id(&[0x11; 32], 42).to_string(),
            "553a3192399806a83a6f64004c4cab48"
        );
        assert_eq!(trace_id_from_digest([0; 32]), TraceId::from(1));
        let mut digest = [0; 32];
        digest[31] = 7;
        assert_eq!(trace_id_from_digest(digest), TraceId::from(1));
        digest[0] = 2;
        assert_eq!(trace_id_from_digest(digest).to_bytes()[0], 2);
    }

    #[test]
    fn network_hash_parsing() {
        for value in ["ab".repeat(32), "AB".repeat(32)] {
            assert_eq!(
                parse_network_hash(Some(OsStr::new(&value))).unwrap(),
                [0xab; 32]
            );
        }
        for value in [None, Some(OsStr::new(""))] {
            assert_eq!(
                parse_network_hash(value).unwrap_err(),
                "GROUNDHOG_NETWORK_HASH is not set; slot spans are not correlated with chuckd block traces"
            );
        }
        for value in ["a".repeat(63), "a".repeat(65), "a".repeat(100_000)] {
            let error =
                parse_network_hash(Some(OsStr::new(&value))).unwrap_err();
            assert!(error.contains("64 hex") && error.len() < 200);
        }
        assert!(
            parse_network_hash(Some(OsStr::new(&"zz".repeat(32))))
                .unwrap_err()
                .contains("non-hex")
        );
        assert!(parse_network_hash(Some(OsStr::new(&"é".repeat(32)))).is_err());
    }

    #[test]
    fn explicit_root_samplers() {
        for (name, sampled) in [
            ("always_on", true),
            ("always_off", false),
            ("parentbased_always_on", true),
            ("parentbased_always_off", false),
        ] {
            let (sampler, warning) =
                sampler_override(Some(OsStr::new(name)), None);
            assert!(warning.is_none());
            let root: Box<dyn ShouldSample> = match sampler {
                Sampler::ParentBased(root) => root,
                sampler => Box::new(sampler),
            };
            assert_eq!(
                root.should_sample(
                    None,
                    block_trace_id(&[0x11; 32], 42),
                    "slot",
                    &SpanKind::Internal,
                    &[],
                    &[]
                )
                .decision
                    == SamplingDecision::RecordAndSample,
                sampled
            );
        }
    }

    #[tokio::test]
    async fn correlated_slot_spans() {
        use opentelemetry::trace::TracerProvider;
        use tracing_subscriber::prelude::*;

        let Ok(case) = std::env::var("SCP_SLOT_TRACE_TEST") else {
            for case in [
                "on",
                "off",
                "missing",
                "invalid",
                "ratio_drop",
                "ratio_sample",
                "filter",
            ] {
                let mut command = std::process::Command::new(
                    std::env::current_exe().unwrap(),
                );
                command
                    .args([
                        "--exact",
                        "telemetry::tests::correlated_slot_spans",
                    ])
                    .env("SCP_SLOT_TRACE_TEST", case)
                    .env(
                        "OTEL_TRACES_SAMPLER",
                        if case.starts_with("ratio_") {
                            "traceidratio"
                        } else {
                            "parentbased_traceidratio"
                        },
                    )
                    .env(
                        "OTEL_TRACES_SAMPLER_ARG",
                        match case {
                            "off" => "0.0",
                            "ratio_drop" => "0.22",
                            "ratio_sample" => "0.23",
                            _ => "1.0",
                        },
                    )
                    .env("OTEL_EXPORTER_OTLP_ENDPOINT", "http://127.0.0.1:4317")
                    .env_remove("OTEL_SDK_DISABLED")
                    .env_remove("OTEL_EXPORTER_OTLP_PROTOCOL")
                    .env_remove("OTEL_EXPORTER_OTLP_TRACES_PROTOCOL")
                    .env_remove("GROUNDHOG_NETWORK_HASH");
                command.env(
                    "OTEL_SPAN_FILTER",
                    if case == "filter" {
                        "warn,scp_kernel=info"
                    } else {
                        "info"
                    },
                );
                if case != "missing" {
                    command.env(
                        "GROUNDHOG_NETWORK_HASH",
                        if case == "invalid" {
                            "bad".to_owned()
                        } else {
                            "11".repeat(32)
                        },
                    );
                }
                assert!(command.status().unwrap().success(), "{case}");
            }
            return;
        };
        let (sampler, warning) = sampler_override(
            std::env::var_os("OTEL_TRACES_SAMPLER").as_deref(),
            std::env::var_os("OTEL_TRACES_SAMPLER_ARG").as_deref(),
        );
        assert!(warning.is_none());
        // Exercise production setup without exporting any spans to the OTLP
        // provider; use an in-memory exporter for all assertions below.
        let (otel_provider, warnings) = provider(None);
        assert!(otel_provider.is_some());
        let correlated = !matches!(case.as_str(), "missing" | "invalid");
        assert_eq!(warnings.len(), usize::from(!correlated));
        if !correlated {
            assert!(warnings[0].contains("GROUNDHOG_NETWORK_HASH"));
            assert!(SLOT_TRACES.get().is_none());
        }
        otel_provider.unwrap().shutdown().unwrap();
        let spans = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let provider = SdkTracerProvider::builder()
            .with_sampler(sampler)
            .with_simple_exporter(Exporter(spans.clone()))
            .build();
        let subscriber = tracing_subscriber::registry().with(
            tracing_opentelemetry::layer()
                .with_tracer(provider.tracer("test"))
                .with_filter(span_filter().0),
        );
        if case.starts_with("ratio_") || case == "filter" {
            let expected = block_trace_id(&[0x11; 32], 42);
            let bytes = expected.to_bytes();
            let fraction = (u64::from_be_bytes(bytes[8..].try_into().unwrap())
                >> 1) as f64
                / (1_u64 << 63) as f64;
            assert!(0.22 < fraction && fraction < 0.23, "{fraction}");
            tracing::subscriber::with_default(subscriber, || {
                slot_span(42).in_scope(|| {
                    tracing::info_span!(target: "scp_kernel", "filtered_child").in_scope(|| {});
                });
            });
            provider.shutdown().unwrap();
            let spans = spans.lock().unwrap();
            if case == "ratio_drop" {
                assert!(spans.is_empty(), "{spans:?}");
            } else {
                assert_eq!(spans.len(), 2, "{spans:?}");
                assert_eq!(
                    expected.to_string(),
                    "553a3192399806a83a6f64004c4cab48"
                );
                assert!(
                    spans.iter().all(|s| s.span_context.trace_id() == expected)
                );
                let slot = spans.iter().find(|s| s.name == "slot").unwrap();
                let child =
                    spans.iter().find(|s| s.name == "filtered_child").unwrap();
                assert_eq!(child.parent_span_id, slot.span_context.span_id());
            }
            return;
        }
        tracing::subscriber::with_default(subscriber, || {
            let disabled = tracing::Span::none();
            link_slot(&disabled, 42);
            slot_span(42).in_scope(|| {
                tracing::info_span!("child").in_scope(|| {});
                // Same block keeps its natural parent even when linked again.
                slot_span(42).in_scope(|| {
                    tracing::info_span!("grandchild").in_scope(|| {})
                });
                slot_span(43).in_scope(|| {});
            });
            tracing::info_span!("independent").in_scope(|| {});
        });
        provider.shutdown().unwrap();
        let spans = spans.lock().unwrap();
        if case == "off" {
            assert!(spans.is_empty(), "{spans:?}");
            return;
        }
        let child = spans.iter().find(|s| s.name == "child").unwrap();
        let slot = spans
            .iter()
            .find(|s| {
                s.name == "slot"
                    && s.span_context.span_id() == child.parent_span_id
                    && s.attributes.iter().any(|a| {
                        a.key.as_str() == "groundhog.block.height"
                            && a.value == opentelemetry::Value::I64(42)
                    })
            })
            .unwrap();
        assert_eq!(child.parent_span_id, slot.span_context.span_id());
        assert_eq!(child.span_context.trace_id(), slot.span_context.trace_id());
        let nested = spans.iter().find(|s| s.name == "grandchild").unwrap();
        let nested_slot = spans
            .iter()
            .find(|s| s.span_context.span_id() == nested.parent_span_id)
            .unwrap();
        assert_eq!(nested_slot.parent_span_id, slot.span_context.span_id());
        let independent =
            spans.iter().find(|s| s.name == "independent").unwrap();
        assert_ne!(
            independent.span_context.trace_id(),
            slot.span_context.trace_id()
        );
        let expected =
            TraceId::from_hex("553a3192399806a83a6f64004c4cab48").unwrap();
        if correlated {
            assert_eq!(slot.span_context.trace_id(), expected);
            let other = spans
                .iter()
                .find(|s| {
                    s.name == "slot"
                        && s.attributes.iter().any(|a| {
                            a.key.as_str() == "groundhog.block.height"
                                && a.value == opentelemetry::Value::I64(43)
                        })
                })
                .unwrap();
            assert_eq!(
                other.span_context.trace_id(),
                block_trace_id(&[0x11; 32], 43)
            );
        } else {
            assert_ne!(slot.span_context.trace_id(), expected);
        }
    }

    #[derive(Debug, Clone)]
    struct Exporter(std::sync::Arc<std::sync::Mutex<Vec<SpanData>>>);

    impl SpanExporter for Exporter {
        async fn export(&self, batch: Vec<SpanData>) -> OTelSdkResult {
            self.0.lock().unwrap().extend(batch);
            Ok(())
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn sigterm_during_startup_reaches_shutdown() {
        use opentelemetry::trace::TracerProvider;
        use tracing_subscriber::prelude::*;

        if std::env::var_os("SCP_SIGNAL_TEST").is_none() {
            assert!(std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "telemetry::tests::sigterm_during_startup_reaches_shutdown"])
                .env("SCP_SIGNAL_TEST", "1")
                .status().unwrap().success());
            return;
        }
        let termination = termination_signal();
        let spans = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let provider = SdkTracerProvider::builder()
            .with_sampler(Sampler::AlwaysOn)
            .with_batch_exporter(Exporter(spans.clone()))
            .build();
        let subscriber = tracing_subscriber::registry().with(
            tracing_opentelemetry::layer().with_tracer(provider.tracer("test")),
        );
        tracing::subscriber::with_default(subscriber, || {
            tracing::info_span!("startup").in_scope(|| {});
        });
        let startup = async {
            assert!(
                std::process::Command::new("kill")
                    .args(["-TERM", &std::process::id().to_string()])
                    .status()
                    .unwrap()
                    .success()
            );
            std::future::pending::<()>().await;
        };
        tokio::time::timeout(
            Duration::from_secs(5),
            run_until_termination(startup, termination),
        )
        .await
        .unwrap();
        tokio::task::spawn_blocking(move || provider.shutdown())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(spans.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn termination_cancels_pending_startup() {
        let (send, receive) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(run_until_termination(
            std::future::pending(),
            async {
                receive.await.unwrap();
            },
        ));
        send.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(1), task)
            .await
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    async fn termination_waits_after_startup() {
        let (send, receive) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(run_until_termination(async {}, async {
            receive.await.unwrap();
        }));
        tokio::task::yield_now().await;
        assert!(!task.is_finished());
        send.send(()).unwrap();
        task.await.unwrap();
    }

    #[test]
    fn exported_spans_obey_log_filter() {
        use opentelemetry::trace::TracerProvider;
        use tracing_subscriber::prelude::*;

        if std::env::var_os("SCP_FILTER_TEST").is_none() {
            for case in ["custom", "default", "empty"] {
                let mut command = std::process::Command::new(
                    std::env::current_exe().unwrap(),
                );
                command
                    .args([
                        "--exact",
                        "telemetry::tests::exported_spans_obey_log_filter",
                    ])
                    .env("SCP_FILTER_TEST", case)
                    .env(
                        "RUST_LOG",
                        if case == "custom" { "off" } else { "error" },
                    )
                    .env_remove("OTEL_SPAN_FILTER");
                if case != "default" {
                    command.env(
                        "OTEL_SPAN_FILTER",
                        if case == "custom" {
                            "warn,scp_kernel=info"
                        } else {
                            ""
                        },
                    );
                }
                assert!(command.status().unwrap().success());
            }
            return;
        }
        let spans = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let provider = SdkTracerProvider::builder()
            .with_sampler(Sampler::AlwaysOn)
            .with_simple_exporter(Exporter(spans.clone()))
            .build();
        let subscriber = tracing_subscriber::registry().with(
            tracing_opentelemetry::layer()
                .with_tracer(provider.tracer("test"))
                .with_filter(span_filter().0),
        );
        tracing::subscriber::with_default(subscriber, || {
            tracing::info_span!(target: "unrelated", "excluded")
                .in_scope(|| {});
            tracing::debug_span!(target: "scp_kernel", "excluded_debug")
                .in_scope(|| {});
            tracing::info_span!(target: "scp_kernel", "consensus")
                .in_scope(|| {});
            tracing::warn_span!(target: "unrelated", "warning").in_scope(|| {});
        });
        provider.shutdown().unwrap();
        let names: Vec<_> = spans
            .lock()
            .unwrap()
            .iter()
            .map(|s| s.name.to_string())
            .collect();
        if std::env::var("SCP_FILTER_TEST").unwrap() == "custom" {
            assert_eq!(names, ["consensus", "warning"]);
        } else {
            assert_eq!(names, ["excluded", "consensus", "warning"]);
        }
    }

    #[test]
    fn nomination_span_exports_integer_attributes() {
        use opentelemetry::trace::TracerProvider;
        use scp_kernel::{TestKernel, TestNode};
        use scp_proto::{SCPQuorumSet, SCPValue};
        use tracing_subscriber::prelude::*;
        use xdr_common::VecM;

        // Isolate tracing callsite interest from concurrent tests.
        if std::env::var_os("SCP_OTEL_INT_TEST").is_none() {
            assert!(
                std::process::Command::new(std::env::current_exe().unwrap())
                    .args([
                        "--exact",
                        "telemetry::tests::nomination_span_exports_integer_attributes"
                    ])
                    .env("SCP_OTEL_INT_TEST", "1")
                    .status()
                    .unwrap()
                    .success()
            );
            return;
        }
        let spans = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let provider = SdkTracerProvider::builder()
            .with_sampler(Sampler::AlwaysOn)
            .with_simple_exporter(Exporter(spans.clone()))
            .build();
        let subscriber = tracing_subscriber::registry()
            .with(
                tracing_opentelemetry::layer()
                    .with_tracer(provider.tracer("test")),
            )
            .with(tracing_subscriber::filter::LevelFilter::INFO);
        tracing::subscriber::with_default(subscriber, || {
            let nodes = TestNode::new_batch(2);
            let qset = SCPQuorumSet {
                threshold: 2,
                validators: VecM(vec![
                    nodes[0].node_id().clone(),
                    nodes[1].node_id().clone(),
                ]),
                inner_sets: VecM(vec![]),
            };
            let mut kernel =
                TestKernel::new(nodes[0].node_id().clone(), qset, true);
            kernel.driver().priority_lookup = Box::new(|_| 1000);
            kernel.nominate(42, &SCPValue::default(), false);
        });
        provider.shutdown().unwrap();
        let spans = spans.lock().unwrap();
        let span = spans.iter().find(|span| span.name == "nominate").unwrap();
        for (key, value) in [
            ("slot_index", 42),
            ("groundhog.block.height", 42),
            ("entry_round_number", 0),
        ] {
            assert!(span.attributes.iter().any(|attribute| {
                attribute.key.as_str() == key
                    && attribute.value == opentelemetry::Value::I64(value)
            }));
        }
    }

    #[test]
    fn tracing_disabled_without_endpoint() {
        assert!(!enabled(None, None));
        assert!(!enabled(Some(OsStr::new("")), None));
    }

    #[test]
    fn invalid_span_filter_falls_back() {
        for value in [None, Some(OsStr::new("")), Some(OsStr::new("[invalid"))]
        {
            let (filter, warning) = parse_span_filter(value);
            assert_eq!(filter.to_string(), "scpd::slot=info,info");
            assert_eq!(
                warning.is_some(),
                value == Some(OsStr::new("[invalid"))
            );
        }
    }

    #[test]
    fn slot_filter_overrides_target_directive() {
        let (filter, warning) =
            parse_span_filter(Some(OsStr::new("warn,scpd::slot=off")));
        assert!(warning.is_none());
        assert_eq!(filter.to_string(), "scpd::slot=info,warn");
    }

    #[test]
    fn protocol_warnings() {
        for name in [
            "OTEL_EXPORTER_OTLP_PROTOCOL",
            "OTEL_EXPORTER_OTLP_TRACES_PROTOCOL",
        ] {
            for value in [None, Some(OsStr::new("")), Some(OsStr::new("grpc"))]
            {
                assert!(protocol_warning(name, value).is_none());
            }
            let warning =
                protocol_warning(name, Some(OsStr::new("http/protobuf")))
                    .unwrap();
            assert!(
                warning.contains("ignored")
                    && warning.contains("gRPC")
                    && warning.contains("4317")
            );
        }
    }

    #[test]
    fn service_resource_attributes() {
        if std::env::var_os("SCP_RESOURCE_TEST").is_none() {
            for (attributes, service_name) in [
                (None, None),
                (
                    Some("service.name=from-attributes,service.version=9.9.9"),
                    Some("from-env"),
                ),
            ] {
                let mut command = std::process::Command::new(
                    std::env::current_exe().unwrap(),
                );
                command
                    .args([
                        "--exact",
                        "telemetry::tests::service_resource_attributes",
                    ])
                    .env("SCP_RESOURCE_TEST", "1")
                    .env_remove("OTEL_RESOURCE_ATTRIBUTES")
                    .env_remove("OTEL_SERVICE_NAME");
                if let Some(attributes) = attributes {
                    command.env("OTEL_RESOURCE_ATTRIBUTES", attributes);
                }
                if let Some(service_name) = service_name {
                    command.env("OTEL_SERVICE_NAME", service_name);
                }
                assert!(command.status().unwrap().success());
            }
            return;
        }
        let overridden = std::env::var_os("OTEL_SERVICE_NAME").is_some();
        let resource = resource(Some("node-id".into()));
        for (key, value) in [
            ("service.name", if overridden { "from-env" } else { "scpd" }),
            (
                "service.version",
                if overridden {
                    "9.9.9"
                } else {
                    env!("CARGO_PKG_VERSION")
                },
            ),
            ("service.instance.id", "node-id"),
        ] {
            assert_eq!(
                resource.get(&key.into()),
                Some(opentelemetry::Value::from(value))
            );
        }
    }

    #[tokio::test]
    async fn flush_finishes_first() {
        let (send, receive) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(flush_unless_interrupted(
            async {
                receive.await.unwrap();
            },
            std::future::pending(),
        ));
        send.send(()).unwrap();
        assert!(
            tokio::time::timeout(Duration::from_secs(1), task)
                .await
                .unwrap()
                .unwrap()
        );
    }

    #[tokio::test]
    async fn termination_interrupts_flush() {
        let (send, receive) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(flush_unless_interrupted(
            std::future::pending(),
            async {
                receive.await.unwrap();
            },
        ));
        send.send(()).unwrap();
        assert!(
            !tokio::time::timeout(Duration::from_secs(1), task)
                .await
                .unwrap()
                .unwrap()
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn second_sigterm_interrupts_flush() {
        if std::env::var_os("SCP_SECOND_SIGNAL_TEST").is_none() {
            assert!(
                std::process::Command::new(std::env::current_exe().unwrap())
                    .args([
                        "--exact",
                        "telemetry::tests::second_sigterm_interrupts_flush"
                    ])
                    .env("SCP_SECOND_SIGNAL_TEST", "1")
                    .status()
                    .unwrap()
                    .success()
            );
            return;
        }
        let signal = || {
            assert!(
                std::process::Command::new("kill")
                    .args(["-TERM", &std::process::id().to_string()])
                    .status()
                    .unwrap()
                    .success()
            );
        };
        let termination = termination_signal();
        signal();
        tokio::time::timeout(
            Duration::from_secs(1),
            run_until_termination(std::future::pending(), termination),
        )
        .await
        .unwrap();
        let termination = termination_signal();
        let flush = async {
            signal();
            std::future::pending::<()>().await;
        };
        assert!(
            !tokio::time::timeout(
                Duration::from_secs(1),
                flush_unless_interrupted(flush, termination)
            )
            .await
            .unwrap()
        );
    }

    #[derive(Clone)]
    struct LogBuffer(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

    impl std::io::Write for LogBuffer {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn collector_failure_logged_once() {
        use opentelemetry::trace::{Span, Tracer, TracerProvider};
        if std::env::var_os("SCP_COLLECTOR_TEST").is_none() {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let port = listener.local_addr().unwrap().port();
            drop(listener);
            assert!(
                std::process::Command::new(std::env::current_exe().unwrap())
                    .args([
                        "--exact",
                        "telemetry::tests::collector_failure_logged_once"
                    ])
                    .env("SCP_COLLECTOR_TEST", "1")
                    .env(
                        "OTEL_EXPORTER_OTLP_ENDPOINT",
                        format!("http://127.0.0.1:{port}")
                    )
                    .env("OTEL_BSP_SCHEDULE_DELAY", "100")
                    .env("OTEL_EXPORTER_OTLP_TIMEOUT", "200")
                    .env("OTEL_TRACES_SAMPLER", "always_on")
                    .env("GROUNDHOG_NETWORK_HASH", "11".repeat(32))
                    .env_remove("OTEL_SDK_DISABLED")
                    .env_remove("OTEL_EXPORTER_OTLP_PROTOCOL")
                    .env_remove("OTEL_EXPORTER_OTLP_TRACES_PROTOCOL")
                    .env_remove("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT")
                    .env_remove("OTEL_EXPORTER_OTLP_TRACES_TIMEOUT")
                    .env_remove("OTEL_EXPORTER_OTLP_HEADERS")
                    .env_remove("OTEL_EXPORTER_OTLP_TRACES_HEADERS")
                    .status()
                    .unwrap()
                    .success()
            );
            return;
        }
        let output =
            LogBuffer(std::sync::Arc::new(std::sync::Mutex::new(Vec::new())));
        let writer = output.clone();
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::TRACE)
            .with_ansi(false)
            .with_writer(move || writer.clone())
            .init();
        let (provider, warnings) = provider(None);
        assert!(warnings.is_empty(), "{warnings:?}");
        let provider = provider.unwrap();
        let tracer = provider.tracer("test");
        for _ in 0..6 {
            tracer.start("export").end();
            tokio::time::sleep(Duration::from_millis(150)).await;
        }
        tokio::task::spawn_blocking(move || provider.shutdown())
            .await
            .unwrap()
            .unwrap();
        let captured =
            String::from_utf8(output.0.lock().unwrap().clone()).unwrap();
        assert_eq!(
            captured
                .lines()
                .filter(|line| line.contains("span export failed"))
                .count(),
            1,
            "{captured}"
        );
        assert_eq!(
            captured
                .lines()
                .filter(|line| line.contains("ERROR") || line.contains("WARN"))
                .count(),
            1,
            "{captured}"
        );
    }

    #[test]
    fn https_endpoint_is_reported_not_silently_dropped() {
        for endpoint in ["https://collector:4317", "HTTPS://collector:4317"] {
            let warning = unsupported_endpoint(Some(OsStr::new(endpoint)))
                .expect(endpoint);
            assert!(warning.contains("https"), "{warning}");
        }
        assert_eq!(
            unsupported_endpoint(Some(OsStr::new("http://127.0.0.1:4317"))),
            None
        );
        assert_eq!(unsupported_endpoint(None), None);
    }

    #[test]
    fn tracing_enabled_with_endpoint() {
        assert!(enabled(Some(OsStr::new("http://127.0.0.1:4317")), None));
    }

    #[tokio::test]
    async fn sampler_environment() {
        use opentelemetry::trace::{
            Span, SpanContext, SpanId, SpanKind, TraceContextExt, TraceFlags,
            TraceId, TraceState, Tracer, TracerProvider,
        };
        use opentelemetry_sdk::trace::{SamplingDecision, ShouldSample};
        if let Ok(case) = std::env::var("SCP_SAMPLER_TEST_CASE") {
            let (provider, warnings) = provider(None);
            if case == "disabled" {
                assert!(provider.is_none());
                return;
            }
            let provider = provider.unwrap();
            let ratio = if case.starts_with("quarter") {
                0.25
            } else {
                0.05
            };
            let parentbased = case != "quarter_plain";
            assert_eq!(
                warnings.is_empty(),
                !case.ends_with("warning"),
                "{warnings:?}"
            );
            let tracer = provider.tracer("test");
            let expected = Sampler::TraceIdRatioBased(ratio);
            let mut sampled = 0;
            // Root spans expose their generated trace IDs, allowing exact decision checks.
            for _ in 0..2048 {
                let mut span = tracer
                    .start_with_context("root", &opentelemetry::Context::new());
                let id = span.span_context().trace_id();
                let decision = expected
                    .should_sample(
                        None,
                        id,
                        "root",
                        &SpanKind::Internal,
                        &[],
                        &[],
                    )
                    .decision;
                assert_eq!(
                    span.span_context().is_sampled(),
                    decision == SamplingDecision::RecordAndSample
                );
                sampled += usize::from(span.span_context().is_sampled());
                span.end();
            }
            assert!(sampled > 0 && sampled < 2048);
            // Crafted IDs distinguish ratios and parent-based decisions.
            for fraction in [0.01, 0.10, 0.50] {
                for parent_sampled in [false, true] {
                    let low = (fraction * u64::MAX as f64) as u64;
                    let id = TraceId::from(u128::from(low));
                    let context = opentelemetry::Context::new()
                        .with_remote_span_context(SpanContext::new(
                            id,
                            SpanId::from(1),
                            if parent_sampled {
                                TraceFlags::SAMPLED
                            } else {
                                TraceFlags::default()
                            },
                            true,
                            TraceState::default(),
                        ));
                    let mut span = tracer.start_with_context("child", &context);
                    assert_eq!(
                        span.span_context().is_sampled(),
                        if parentbased {
                            parent_sampled
                        } else {
                            fraction < ratio
                        }
                    );
                    span.end();
                }
            }
            let _ = tokio::task::spawn_blocking(move || provider.shutdown())
                .await
                .unwrap();
            return;
        }
        for (name, arg, expected) in [
            (Some("traceidratio"), Some("0.25"), "quarter_plain"),
            (
                Some("parentbased_traceidratio"),
                Some("0.25"),
                "quarter_parent",
            ),
            (Some("parentbased_traceidratio"), None, "fallback_warning"),
            (Some("traceidratio"), Some("abc"), "fallback_warning"),
            (
                Some("parentbased_traceidratio"),
                Some("1.5"),
                "fallback_warning",
            ),
            (Some("traceidratio"), Some("NaN"), "fallback_warning"),
            (Some("traceidratio"), Some("inf"), "fallback_warning"),
            (Some("traceidratio"), Some("-0.1"), "fallback_warning"),
            (None, None, "fallback"),
            (Some(""), Some(""), "fallback"),
            (Some("invalid"), None, "fallback_warning"),
            (None, None, "disabled"),
        ] {
            let mut command =
                std::process::Command::new(std::env::current_exe().unwrap());
            command
                .args(["--exact", "telemetry::tests::sampler_environment"])
                .env("SCP_SAMPLER_TEST_CASE", expected)
                .env("GROUNDHOG_NETWORK_HASH", "11".repeat(32))
                .env("OTEL_EXPORTER_OTLP_ENDPOINT", "http://127.0.0.1:4317")
                .env(
                    "OTEL_SDK_DISABLED",
                    if expected == "disabled" {
                        "true"
                    } else {
                        "false"
                    },
                )
                .env_remove("OTEL_TRACES_SAMPLER")
                .env_remove("OTEL_TRACES_SAMPLER_ARG")
                .env_remove("OTEL_EXPORTER_OTLP_PROTOCOL")
                .env_remove("OTEL_EXPORTER_OTLP_TRACES_PROTOCOL");
            if let Some(name) = name {
                command.env("OTEL_TRACES_SAMPLER", name);
            }
            if let Some(arg) = arg {
                command.env("OTEL_TRACES_SAMPLER_ARG", arg);
            }
            assert!(
                command.status().unwrap().success(),
                "{name:?} {arg:?} {expected}"
            );
        }
    }
}
