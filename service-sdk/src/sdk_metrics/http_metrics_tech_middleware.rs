use async_trait::async_trait;
use my_http_server::*;
use rust_extensions::date_time::DateTimeAsMicroseconds;

pub struct MetricsTechMiddleware;

#[async_trait]
impl HttpServerTechMiddleware for MetricsTechMiddleware {
    async fn got_result(&self, request: &HttpRequestData, http_result: &ResponseData) {
        let now = DateTimeAsMicroseconds::now();
        let duration = now.duration_since(request.started).as_positive_or_zero();

        let common_labels = &[
            ("method", request.method.to_string()),
            ("path", request.path.to_string()),
        ];

        if http_result.has_error {
            if http_result.status_code == 404 {
                metrics::histogram!("http_request_duration_sec", common_labels)
                    .record(duration.as_secs_f64());
                metrics::counter!("http_request_milis_duration_sum", common_labels)
                    .increment(duration.as_millis() as u64);
                metrics::counter!("http_request_count", common_labels).increment(1);
            } else {
                let failed_labels = &[
                    ("method", request.method.to_string()),
                    ("path", request.path.to_string()),
                    ("status_code", http_result.status_code.to_string()),
                ];

                metrics::counter!("http_failed_request_count", failed_labels).increment(1);
                metrics::counter!("http_failed_request_milis_duration_sum", failed_labels)
                    .increment(duration.as_millis() as u64);
                metrics::histogram!("http_failed_request_duration_sec", failed_labels)
                    .record(duration.as_secs_f64());
            }
        }
    }
}
