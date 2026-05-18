use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use arc_swap::ArcSwap;
use async_trait::async_trait;
use rust_extensions::MyTimerTick;

type LabelsKey = Vec<(String, String)>;
type CountersMap = HashMap<LabelsKey, Arc<AtomicU64>>;

pub struct EventsPerSecondCounter {
    metric_name: String,
    counters: ArcSwap<CountersMap>,
}

impl EventsPerSecondCounter {
    pub(crate) fn new(metric_name: impl Into<String>) -> Self {
        Self {
            metric_name: metric_name.into(),
            counters: ArcSwap::from_pointee(HashMap::new()),
        }
    }

    pub fn increment(&self) {
        self.increment_by_with_labels(1, &[]);
    }

    pub fn increment_by(&self, n: u64) {
        self.increment_by_with_labels(n, &[]);
    }

    pub fn increment_with_labels(&self, labels: &[(&str, &str)]) {
        self.increment_by_with_labels(1, labels);
    }

    pub fn increment_by_with_labels(&self, n: u64, labels: &[(&str, &str)]) {
        let key = normalize_labels(labels);

        if let Some(atomic) = self.counters.load().get(&key) {
            atomic.fetch_add(n, Ordering::Relaxed);
            return;
        }

        self.counters.rcu(|prev| {
            if prev.contains_key(&key) {
                return prev.clone();
            }
            let mut new = (**prev).clone();
            new.insert(key.clone(), Arc::new(AtomicU64::new(0)));
            Arc::new(new)
        });

        self.counters
            .load()
            .get(&key)
            .expect("entry just inserted via rcu")
            .fetch_add(n, Ordering::Relaxed);
    }

    fn tick(&self) {
        let snapshot: Vec<(LabelsKey, u64)> = self
            .counters
            .load()
            .iter()
            .map(|(labels, atomic)| (labels.clone(), atomic.swap(0, Ordering::Relaxed)))
            .collect();

        for (labels, value) in snapshot {
            let name = self.metric_name.clone();
            if labels.is_empty() {
                metrics::gauge!(name).set(value as f64);
            } else {
                metrics::gauge!(name, &labels).set(value as f64);
            }
        }
    }
}

fn normalize_labels(labels: &[(&str, &str)]) -> LabelsKey {
    let mut owned: LabelsKey = labels
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect();
    owned.sort_by(|a, b| a.0.cmp(&b.0));
    owned
}

pub(crate) struct EventsPerSecondTimerTick {
    pub counters: Arc<ArcSwap<Vec<Arc<EventsPerSecondCounter>>>>,
}

#[async_trait]
impl MyTimerTick for EventsPerSecondTimerTick {
    async fn tick(&self) {
        let counters = self.counters.load_full();
        for counter in counters.iter() {
            counter.tick();
        }
    }
}
