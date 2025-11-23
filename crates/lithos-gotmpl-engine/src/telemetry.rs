// SPDX-License-Identifier: Apache-2.0 OR MIT
#![cfg_attr(not(feature = "telemetry"), allow(dead_code))]

#[cfg(feature = "telemetry")]
mod otel {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::OnceLock;
    use std::time::Duration;

    use opentelemetry::global;
    use opentelemetry::metrics::{Counter, Histogram};
    use opentelemetry::trace::SpanKind;
    use opentelemetry::{trace::Tracer, KeyValue};

    const METER_NAME: &str = "lithos_gotmpl_engine";
    const TRACER_NAME: &str = "lithos_gotmpl_engine";

    static ENABLED: AtomicBool = AtomicBool::new(false);
    static HANDLES: OnceLock<Handles> = OnceLock::new();

    struct Handles {
        tracer: opentelemetry::global::BoxedTracer,
        render_hist: Histogram<f64>,
        analyze_hist: Histogram<f64>,
        render_counter: Counter<u64>,
        analyze_counter: Counter<u64>,
        helper_counter: Counter<u64>,
    }

    impl Handles {
        fn new() -> Self {
            let meter = global::meter(METER_NAME);
            let render_hist = meter
                .f64_histogram("lithos.render.duration_ms")
                .with_description("Render duration in milliseconds")
                .init();
            let analyze_hist = meter
                .f64_histogram("lithos.analyze.duration_ms")
                .with_description("Analyze duration in milliseconds")
                .init();
            let render_counter = meter
                .u64_counter("lithos.render.count")
                .with_description("Number of template renders")
                .init();
            let analyze_counter = meter
                .u64_counter("lithos.analyze.count")
                .with_description("Number of template analyses")
                .init();
            let helper_counter = meter
                .u64_counter("lithos.helper.count")
                .with_description("Number of helper invocations")
                .init();
            let tracer = global::tracer(TRACER_NAME);
            Self {
                tracer,
                render_hist,
                analyze_hist,
                render_counter,
                analyze_counter,
                helper_counter,
            }
        }
    }

    fn handles() -> &'static Handles {
        HANDLES.get_or_init(Handles::new)
    }

    pub fn enable() {
        ENABLED.store(true, Ordering::Relaxed);
    }

    pub fn disable() {
        ENABLED.store(false, Ordering::Relaxed);
    }

    fn enabled() -> bool {
        ENABLED.load(Ordering::Relaxed)
    }

    pub fn record_render(template: &str, template_len: usize, duration: Duration, success: bool) {
        if !enabled() {
            return;
        }
        let hs = handles();
        let duration_ms = duration.as_secs_f64() * 1_000.0;
        let attrs = [
            KeyValue::new("template.name", template.to_string()),
            KeyValue::new("template.length", template_len as i64),
            KeyValue::new("render.success", success),
        ];
        hs.render_counter.add(1, &attrs);
        hs.render_hist.record(duration_ms, &attrs);
        let mut span = hs
            .tracer
            .span_builder("Template::render")
            .with_kind(SpanKind::Internal)
            .start(&hs.tracer);
        span.set_attribute(KeyValue::new("template.name", template.to_string()));
        span.set_attribute(KeyValue::new("template.length", template_len as i64));
        span.set_attribute(KeyValue::new("render.duration_ms", duration_ms));
        span.set_attribute(KeyValue::new("render.success", success));
        span.end();
    }

    pub fn record_analyze(template: &str, template_len: usize, duration: Duration, success: bool) {
        if !enabled() {
            return;
        }
        let hs = handles();
        let duration_ms = duration.as_secs_f64() * 1_000.0;
        let attrs = [
            KeyValue::new("template.name", template.to_string()),
            KeyValue::new("template.length", template_len as i64),
            KeyValue::new("analyze.success", success),
        ];
        hs.analyze_counter.add(1, &attrs);
        hs.analyze_hist.record(duration_ms, &attrs);
        let mut span = hs
            .tracer
            .span_builder("Template::analyze")
            .with_kind(SpanKind::Internal)
            .start(&hs.tracer);
        span.set_attribute(KeyValue::new("template.name", template.to_string()));
        span.set_attribute(KeyValue::new("template.length", template_len as i64));
        span.set_attribute(KeyValue::new("analyze.duration_ms", duration_ms));
        span.set_attribute(KeyValue::new("analyze.success", success));
        span.end();
    }

    pub fn record_helper_invocation(name: &str, kind: &'static str, success: bool) {
        if !enabled() {
            return;
        }
        let hs = handles();
        let attrs = [
            KeyValue::new("helper.name", name.to_string()),
            KeyValue::new("helper.kind", kind.to_string()),
            KeyValue::new("helper.success", success),
        ];
        hs.helper_counter.add(1, &attrs);
    }
}

#[cfg(not(feature = "telemetry"))]
mod otel {
    use std::time::Duration;

    pub fn enable() {}
    pub fn disable() {}
    pub fn record_render(
        _template: &str,
        _template_len: usize,
        _duration: Duration,
        _success: bool,
    ) {
    }
    pub fn record_analyze(
        _template: &str,
        _template_len: usize,
        _duration: Duration,
        _success: bool,
    ) {
    }

    pub fn record_helper_invocation(_name: &str, _kind: &'static str, _success: bool) {}
}

pub use otel::{disable, enable, record_analyze, record_helper_invocation, record_render};
