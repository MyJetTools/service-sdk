use async_trait::async_trait;
use my_http_server::*;

pub struct MetricsMiddleware;

impl MetricsMiddleware {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl HttpServerMiddleware for MetricsMiddleware {
    async fn handle_request(
        &self,
        ctx: &mut HttpContext,
    ) -> Option<Result<HttpOkResult, HttpFailResult>> {
        let path = ctx.request.http_path.as_str().to_string();

        if path == "/metrics" {
            let report = prometheus::TextEncoder::new()
                .encode_to_string(&prometheus::default_registry().gather());

            match report {
                Ok(report) => {
                    let response = HttpOutput::as_text(report).into_ok_result(false);
                    return Some(response);
                }
                Err(err) => {
                    let response = HttpOutput::from_builder()
                        .set_content_as_text(err.to_string())
                        .set_status_code(502)
                        .into_fail_result(false, false);

                    return Some(response);
                }
            }
        }
        None

        /*
        let mut sw = Stopwatch::start_new();
        let result = get_next.next(ctx).await;
        sw.stop();

        let duration = sw.elapsed();
        let method = ctx.request.method.as_str().to_string();
        let path = ctx.request.http_path.as_str().to_string();
        let common_labels = &[("method", method.clone()), ("path", path.clone())];

        let mut has_to_write_metrics = false;

        if let Err(result) = &result {
            if result.status_code == 404 {
                has_to_write_metrics = true
            } else {
                let failed_labels = &[
                    ("method", method.clone()),
                    ("path", path.clone()),
                    ("status_code", result.status_code.to_string()),
                ];

                metrics::counter!("http_failed_request_count", failed_labels).increment(1);
                metrics::counter!("http_failed_request_milis_duration_sum", failed_labels)
                    .increment(duration.as_millis() as u64);
                metrics::histogram!("http_failed_request_duration_sec", failed_labels)
                    .record(duration.as_secs_f64());
            }
        }

        if has_to_write_metrics {
            return result;
        }

        metrics::histogram!("http_request_duration_sec", common_labels)
            .record(duration.as_secs_f64());
        metrics::counter!("http_request_milis_duration_sum", common_labels)
            .increment(duration.as_millis() as u64);
        metrics::counter!("http_request_count", common_labels).increment(1);

        return result;
         */
    }
}
