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
    if let Ok(name) = std::env::var("OTEL_SERVICE_NAME") {
        if !name.is_empty() {
            resource = resource.with_service_name(name);
        }
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
        if let Err(error) = &result {
            if !self.warned.swap(true, Ordering::Relaxed) {
                tracing::warn!(%error, "OpenTelemetry span export failed; collector may be unreachable");
            }
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

pub async fn termination_signal() {
    #[cfg(unix)]
    {
        let mut terminate = tokio::signal::unix::signal(
            tokio::signal::unix::SignalKind::terminate(),
        )
        .expect("install SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = terminate.recv() => {},
        }
    }
    #[cfg(not(unix))]
    let _ = tokio::signal::ctrl_c().await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracing_disabled_without_endpoint() {
        assert!(!enabled(None, None));
        assert!(!enabled(Some(OsStr::new("")), None));
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
