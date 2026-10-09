//! Tracing is enabled by OTEL_EXPORTER_OTLP_ENDPOINT (OTLP over gRPC,
//! typically port 4317; http only). OTEL_EXPORTER_OTLP_PROTOCOL supports only grpc.
//! OTEL_SDK_DISABLED disables tracing. OTEL_TRACES_SAMPLER / OTEL_TRACES_SAMPLER_ARG
//! default to parentbased_traceidratio 0.05. OTEL_SPAN_FILTER defaults to info,
//! independently of RUST_LOG. OTEL_SERVICE_NAME and OTEL_RESOURCE_ATTRIBUTES
//! configure the service resource.

use std::ffi::OsStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use opentelemetry_sdk::Resource;
use opentelemetry_sdk::error::OTelSdkResult;
use opentelemetry_sdk::resource::EnvResourceDetector;
use opentelemetry_sdk::trace::{
    Sampler, SdkTracerProvider, SpanData, SpanExporter,
};

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
) -> (Option<Sampler>, Option<String>) {
    let value = value.filter(|value| !value.is_empty());
    match value.and_then(OsStr::to_str) {
        Some(name @ ("traceidratio" | "parentbased_traceidratio")) => {
            if let Some(ratio) = arg.and_then(OsStr::to_str)
                .and_then(|arg| arg.parse::<f64>().ok())
                .filter(|ratio| ratio.is_finite() && (0.0..=1.0).contains(ratio))
            {
                let sampler = Sampler::TraceIdRatioBased(ratio);
                return (Some(if name == "traceidratio" { sampler } else {
                    Sampler::ParentBased(Box::new(sampler))
                }), None);
            }
            (Some(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(0.05)))),
                Some(format!("Invalid or missing OTEL_TRACES_SAMPLER_ARG {arg:?}; using parentbased_traceidratio with ratio 0.05")))
        }
        Some("always_on" | "always_off"
            | "parentbased_always_on" | "parentbased_always_off") => (None, None),
        _ => (
            Some(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(0.05)))),
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
    // The SDK reads sampler env vars in the builder's default config. Suppress
    // its invalid-value warning: we report our different fallback after the
    // application's logging subscriber has been installed.
    let mut builder = tracing::subscriber::with_default(
        tracing::subscriber::NoSubscriber::default(),
        SdkTracerProvider::builder,
    );
    if let Some(sampler) = sampler {
        builder = builder.with_sampler(sampler);
    }
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
    if let Ok(name) = std::env::var("OTEL_SERVICE_NAME")
        && !name.is_empty()
    {
        resource = resource.with_service_name(name);
    }
    resource
        .with_detector(Box::new(EnvResourceDetector::new()))
        .build()
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
    match value {
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
    }
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
            assert_eq!(filter.to_string(), "info");
            assert_eq!(
                warning.is_some(),
                value == Some(OsStr::new("[invalid"))
            );
        }
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
            assert!(
                std::process::Command::new(std::env::current_exe().unwrap())
                    .args([
                        "--exact",
                        "telemetry::tests::service_resource_attributes"
                    ])
                    .env("SCP_RESOURCE_TEST", "1")
                    .env_remove("OTEL_RESOURCE_ATTRIBUTES")
                    .env_remove("OTEL_SERVICE_NAME")
                    .status()
                    .unwrap()
                    .success()
            );
            return;
        }
        let resource = resource(Some("node-id".into()));
        for (key, value) in [
            ("service.name", "scpd"),
            ("service.version", env!("CARGO_PKG_VERSION")),
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
                    .env_remove("OTEL_SDK_DISABLED")
                    .env_remove("OTEL_EXPORTER_OTLP_PROTOCOL")
                    .env_remove("OTEL_EXPORTER_OTLP_TRACES_PROTOCOL")
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
