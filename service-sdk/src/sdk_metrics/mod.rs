// Everything that actually talks to prometheus lives behind
// `with-prometheus-metrics`. The EventsPerSecond API is the exception: it is
// exported in both modes so service code compiles unchanged, and simply does
// nothing when the feature is off.

#[cfg(all(feature = "grpc", feature = "with-prometheus-metrics"))]
mod grpc_metrics_middleware;
#[cfg(all(feature = "grpc", feature = "with-prometheus-metrics"))]
pub use grpc_metrics_middleware::*;

#[cfg(feature = "with-prometheus-metrics")]
mod http_metrics_middleware;
#[cfg(feature = "with-prometheus-metrics")]
pub use http_metrics_middleware::*;

#[cfg(feature = "with-prometheus-metrics")]
mod http_metrics_tech_middleware;
#[cfg(feature = "with-prometheus-metrics")]
pub use http_metrics_tech_middleware::*;

#[cfg(feature = "with-prometheus-metrics")]
mod events_per_second;
#[cfg(feature = "with-prometheus-metrics")]
pub use events_per_second::*;

#[cfg(not(feature = "with-prometheus-metrics"))]
mod events_per_second_no_metrics;
#[cfg(not(feature = "with-prometheus-metrics"))]
pub use events_per_second_no_metrics::*;
