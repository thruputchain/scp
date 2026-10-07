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
) -> (Option<Sampler>, Option<String>) {
    match value.and_then(OsStr::to_str) {
        Some("always_on" | "always_off" | "traceidratio"
            | "parentbased_always_on" | "parentbased_always_off"
            | "parentbased_traceidratio") => (None, None),
        _ => (
            Some(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(0.05)))),
            value.map(|value| format!(
                "Invalid OTEL_TRACES_SAMPLER {value:?}; using parentbased_traceidratio with ratio 0.05"
            )),
        ),
    }
}

pub fn provider() -> (Option<SdkTracerProvider>, Option<String>) {
    if !enabled(
        std::env::var_os("OTEL_EXPORTER_OTLP_ENDPOINT").as_deref(),
        std::env::var_os("OTEL_SDK_DISABLED").as_deref(),
    ) {
        return (None, None);
    }
    // The exporter is built without TLS (devnet sends to the host Alloy over
    // loopback), so an https endpoint would fail silently; say so instead.
    if let Some(warning) = unsupported_endpoint(
        std::env::var_os("OTEL_EXPORTER_OTLP_ENDPOINT").as_deref(),
    ) {
        return (None, Some(warning));
    }
    let (sampler, warning) =
        sampler_override(std::env::var_os("OTEL_TRACES_SAMPLER").as_deref());
    // The SDK reads sampler env vars in the builder's default config. Suppress
    // its invalid-value warning: we report our different fallback after the
    // application's logging subscriber has been installed.
    let mut builder = if warning.is_some() {
        tracing::subscriber::with_default(
            tracing::subscriber::NoSubscriber::default(),
            SdkTracerProvider::builder,
        )
    } else {
        SdkTracerProvider::builder()
    };
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
        Err(error) => return (None, Some(error.to_string())),
    };
    let mut resource = Resource::builder_empty()
        .with_service_name("scpd")
        .with_detector(Box::new(EnvResourceDetector::new()));
    if let Ok(name) = std::env::var("OTEL_SERVICE_NAME")
        && !name.is_empty()
    {
        resource = resource.with_service_name(name);
    }
    let resource = resource.build();
    let provider = builder
        .with_resource(resource)
        .with_batch_exporter(WarnOnceExporter {
            inner: exporter,
            warned: AtomicBool::new(false),
        })
        .build();
    (Some(provider), warning)
}

#[derive(Debug)]
struct WarnOnceExporter {
    inner: opentelemetry_otlp::SpanExporter,
    warned: AtomicBool,
}

impl SpanExporter for WarnOnceExporter {
    async fn export(&self, batch: Vec<SpanData>) -> OTelSdkResult {
        let result = self.inner.export(batch).await;
        if let Err(error) = &result
            && !self.warned.swap(true, Ordering::Relaxed)
        {
            tracing::warn!(%error, "OpenTelemetry span export failed; collector may be unreachable");
        }
        result
    }

    fn set_resource(&mut self, resource: &Resource) {
        self.inner.set_resource(resource);
    }

    fn shutdown_with_timeout(&self, timeout: Duration) -> OTelSdkResult {
        self.inner.shutdown_with_timeout(timeout)
    }
}

pub fn span_filter() -> tracing_subscriber::EnvFilter {
    tracing_subscriber::EnvFilter::from_default_env()
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
            assert!(
                std::process::Command::new(std::env::current_exe().unwrap())
                    .args([
                        "--exact",
                        "telemetry::tests::exported_spans_obey_log_filter"
                    ])
                    .env("SCP_FILTER_TEST", "1")
                    .env("RUST_LOG", "warn,scp_kernel=info")
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
        let subscriber = tracing_subscriber::registry().with(
            tracing_opentelemetry::layer()
                .with_tracer(provider.tracer("test"))
                .with_filter(span_filter()),
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
        assert_eq!(names, ["consensus", "warning"]);
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
        for (key, value) in [("slot_index", 42), ("entry_round_number", 0)] {
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

    #[test]
    fn sampler_environment() {
        // Child processes isolate environment changes from concurrent tests.
        if let Ok(case) = std::env::var("SCP_SAMPLER_TEST_CASE") {
            if case == "disabled" {
                assert!(provider().0.is_none());
                return;
            }
            let (override_sampler, warning) = sampler_override(
                std::env::var_os("OTEL_TRACES_SAMPLER").as_deref(),
            );
            let sampler = override_sampler
                .map(|s| format!("{s:?}"))
                .unwrap_or_else(|| {
                    format!(
                        "{:?}",
                        opentelemetry_sdk::trace::Config::default().sampler
                    )
                });
            assert_eq!(sampler, case);
            assert_eq!(
                warning.is_some(),
                std::env::var("OTEL_TRACES_SAMPLER").as_deref()
                    == Ok("invalid")
            );
            return;
        }
        for (value, expected) in [
            (None, "ParentBased(TraceIdRatioBased(0.05))"),
            (Some("always_on"), "AlwaysOn"),
            (Some("always_off"), "AlwaysOff"),
            (Some("traceidratio"), "TraceIdRatioBased(0.25)"),
            (Some("parentbased_always_on"), "ParentBased(AlwaysOn)"),
            (Some("parentbased_always_off"), "ParentBased(AlwaysOff)"),
            (
                Some("parentbased_traceidratio"),
                "ParentBased(TraceIdRatioBased(0.25))",
            ),
            (Some("invalid"), "ParentBased(TraceIdRatioBased(0.05))"),
            (None, "disabled"),
        ] {
            let mut command =
                std::process::Command::new(std::env::current_exe().unwrap());
            command
                .args([
                    "--exact",
                    "telemetry::tests::sampler_environment",
                    "--nocapture",
                ])
                .env("SCP_SAMPLER_TEST_CASE", expected)
                .env("OTEL_TRACES_SAMPLER_ARG", "0.25")
                .env("OTEL_EXPORTER_OTLP_ENDPOINT", "http://127.0.0.1:4317")
                .env(
                    "OTEL_SDK_DISABLED",
                    if expected == "disabled" {
                        "true"
                    } else {
                        "false"
                    },
                )
                .env_remove("OTEL_TRACES_SAMPLER");
            if let Some(value) = value {
                command.env("OTEL_TRACES_SAMPLER", value);
            }
            assert!(
                command.status().unwrap().success(),
                "case {value:?}: {expected}"
            );
        }
    }
}
