use http::HeaderMap;
use http::HeaderName;
use http::HeaderValue;
use opentelemetry::Context;
use opentelemetry::global;
use opentelemetry::propagation::Extractor;
use opentelemetry::propagation::Injector;
use opentelemetry::trace::TraceContextExt;

/// Convenience methods for OpenTelemetry propagation on [`HeaderMap`].
pub trait HeaderMapOtelExt {
    /// Extract a remote context if a valid trace context is present.
    fn extract_otel_context(&self) -> Option<Context>;

    /// Inject the provided context into the header map.
    fn inject_otel_context(&mut self, context: &Context);
}

impl HeaderMapOtelExt for HeaderMap {
    fn extract_otel_context(&self) -> Option<Context> {
        if self.is_empty() {
            return None;
        }

        global::get_text_map_propagator(|propagator| {
            let context = propagator.extract(&HeaderExtractor(self));
            if context.span().span_context().is_valid() {
                Some(context)
            } else {
                None
            }
        })
    }

    fn inject_otel_context(&mut self, context: &Context) {
        global::get_text_map_propagator(|propagator| {
            propagator.inject_context(context, &mut HeaderInjector(self));
        });
    }
}

struct HeaderExtractor<'a>(&'a HeaderMap);

impl Extractor for HeaderExtractor<'_> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|value| value.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(HeaderName::as_str).collect()
    }
}

struct HeaderInjector<'a>(&'a mut HeaderMap);

impl Injector for HeaderInjector<'_> {
    fn set(&mut self, key: &str, value: String) {
        let Ok(name) = HeaderName::try_from(key) else {
            return;
        };
        let Ok(value) = HeaderValue::from_str(&value) else {
            return;
        };

        self.0.insert(name, value);
    }
}

#[cfg(test)]
mod tests {
    use super::HeaderMapOtelExt;
    use http::HeaderMap;
    use opentelemetry::Context;
    use opentelemetry::global;
    use opentelemetry::trace::SpanContext;
    use opentelemetry::trace::SpanId;
    use opentelemetry::trace::TraceContextExt;
    use opentelemetry::trace::TraceFlags;
    use opentelemetry::trace::TraceId;
    use opentelemetry::trace::TraceState;
    use opentelemetry_sdk::propagation::TraceContextPropagator;

    #[test]
    fn inject_and_extract_round_trip() {
        global::set_text_map_propagator(TraceContextPropagator::new());

        let span_context = SpanContext::new(
            TraceId::from_hex("0123456789abcdef0123456789abcdef").expect("valid trace id"),
            SpanId::from_hex("0123456789abcdef").expect("valid span id"),
            TraceFlags::SAMPLED,
            false,
            TraceState::default(),
        );
        let context = Context::new().with_remote_span_context(span_context.clone());

        let mut headers = HeaderMap::new();
        headers.inject_otel_context(&context);

        let extracted = headers
            .extract_otel_context()
            .expect("expected trace context to round-trip");
        let extracted_span = extracted.span().span_context().clone();

        assert_eq!(extracted_span.trace_id(), span_context.trace_id());
        assert_eq!(extracted_span.span_id(), span_context.span_id());
        assert_eq!(extracted_span.trace_flags(), span_context.trace_flags());
        assert!(extracted_span.is_remote());
    }
}
