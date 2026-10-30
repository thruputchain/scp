//! The kernel's spans record `slot_index`, `thruput.block.height` and the
//! round and ballot counters as integers, so a tracing exporter (scpd's
//! OpenTelemetry layer, for one) emits them as numeric attributes rather than
//! strings.

use crate::testkernel::TestKernel;
use crate::testutil::TestNode;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id};
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::prelude::*;
use xdr_common::VecM;

#[derive(Default)]
struct RecordedSpan {
    name: &'static str,
    ints: HashMap<String, i64>,
    /// Fields recorded through any other `Visit` method.
    non_ints: Vec<String>,
}

impl Visit for RecordedSpan {
    fn record_i64(&mut self, field: &Field, value: i64) {
        self.ints.insert(field.name().to_owned(), value);
    }

    fn record_debug(&mut self, field: &Field, _: &dyn std::fmt::Debug) {
        self.non_ints.push(field.name().to_owned());
    }
}

struct CaptureSpans(Arc<Mutex<Vec<RecordedSpan>>>);

impl<S: tracing::Subscriber> Layer<S> for CaptureSpans {
    fn on_new_span(&self, attrs: &Attributes<'_>, _: &Id, _: Context<'_, S>) {
        let mut span = RecordedSpan {
            name: attrs.metadata().name(),
            ..Default::default()
        };
        attrs.record(&mut span);
        self.0.lock().unwrap().push(span);
    }
}

#[test]
fn nomination_span_records_integer_fields() {
    // A subscriber installed with `with_default` is thread-local, but the
    // interest it registers for callsites is process-wide, so with it in
    // place every `debug!` in concurrently running tests evaluates its
    // arguments, some of which the test drivers deliberately leave
    // unimplemented. Run the body in a process of its own.
    if std::env::var_os("SCP_SPAN_TEST").is_none() {
        let status =
            std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "span_test::nomination_span_records_integer_fields",
                ])
                .env("SCP_SPAN_TEST", "1")
                .status()
                .unwrap();
        assert!(status.success(), "span test failed in the child process");
        return;
    }

    let spans = Arc::new(Mutex::new(Vec::new()));
    // INFO and above: the kernel's debug lines format node IDs through
    // `Driver::to_short_string`, which the test driver does not implement.
    let subscriber = tracing_subscriber::registry()
        .with(CaptureSpans(spans.clone()))
        .with(tracing_subscriber::filter::LevelFilter::INFO);
    tracing::subscriber::with_default(subscriber, || {
        let nodes = TestNode::new_batch(2);
        let qset = thruput_scp::SCPQuorumSet {
            threshold: 2,
            validators: VecM(vec![nodes[0].node_id(), nodes[1].node_id()]),
            inner_sets: VecM(vec![]),
        };
        let mut kernel = TestKernel::new(nodes[0].node_id(), qset, true);
        kernel.driver().priority_lookup = Box::new(|_| 1000);
        kernel.nominate(42, &thruput_scp::SCPValue::default(), false);
    });

    let spans = spans.lock().unwrap();
    let span = spans
        .iter()
        .find(|span| span.name == "nominate")
        .expect("nominate span");
    for (key, value) in [
        ("slot_index", 42),
        ("thruput.block.height", 42),
        ("entry_round_number", 0),
    ] {
        assert_eq!(
            span.ints.get(key),
            Some(&value),
            "{key} should be an integer field; non-integer fields: {:?}",
            span.non_ints
        );
    }
}
