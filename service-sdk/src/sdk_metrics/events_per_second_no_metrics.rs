/// No-op twin of [`crate::EventsPerSecondCounter`], compiled when
/// `with-prometheus-metrics` is off.
///
/// The signatures match the real counter exactly, so a service registers and
/// increments counters the same way regardless of the feature - the values are
/// simply never accumulated and never reported anywhere. Nothing here touches
/// prometheus, and no metrics crate is linked.
///
/// The metric name is not even stored: without a reporter there is nobody to
/// name.
pub struct EventsPerSecondCounter;

impl EventsPerSecondCounter {
    pub(crate) fn new(_metric_name: impl Into<String>) -> Self {
        Self
    }

    pub fn increment(&self) {}

    pub fn increment_by(&self, _n: u64) {}

    pub fn increment_with_labels(&self, _labels: &[(&str, &str)]) {}

    pub fn increment_by_with_labels(&self, _n: u64, _labels: &[(&str, &str)]) {}
}
