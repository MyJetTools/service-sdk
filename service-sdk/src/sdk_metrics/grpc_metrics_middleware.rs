use std::pin::Pin;
use std::task::{Context, Poll};

use my_grpc_extensions::hyper;
use my_grpc_extensions::tonic::body::Body;
use tower::{Layer, Service};

#[derive(Debug, Clone, Default)]
pub struct GrpcMetricsMiddlewareLayer;

impl<S> Layer<S> for GrpcMetricsMiddlewareLayer {
    type Service = GrpcMetricsMiddleware<S>;

    fn layer(&self, service: S) -> Self::Service {
        GrpcMetricsMiddleware { inner: service }
    }
}

#[derive(Debug, Clone)]
pub struct GrpcMetricsMiddleware<S> {
    inner: S,
}

type BoxFuture<'a, T> = Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

impl<S> Service<hyper::Request<Body>> for GrpcMetricsMiddleware<S>
where
    S: Service<hyper::Request<Body>, Response = hyper::Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: hyper::Request<Body>) -> Self::Future {
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);
        let method = req.method().to_string();
        let path = req.uri().path().to_string();

        Box::pin(async move {
            let mut sw = stopwatch::Stopwatch::start_new();
            let response = inner.call(req).await?;
            sw.stop();
            let duration = sw.elapsed();
            let common_labels = &[
                ("method", method),
                ("path", path),
                ("status_code", response.status().to_string()),
            ];

            metrics::histogram!("grpc_request_duration_sec", common_labels)
                .record(duration.as_secs_f64());
            metrics::counter!("grpc_request_duration_milis_sum", common_labels)
                .increment(duration.as_millis() as u64);
            metrics::counter!("grpc_request_count", common_labels).increment(1);

            Ok(response)
        })
    }
}
