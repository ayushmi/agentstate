use once_cell::sync::Lazy;
use prometheus::{
    register_counter_vec, register_gauge, register_gauge_vec, register_histogram,
    register_histogram_vec, CounterVec, Gauge, GaugeVec, Histogram, HistogramVec,
};

pub static WAL_ACTIVE_SEGMENTS: Lazy<Gauge> =
    Lazy::new(|| register_gauge!("wal_active_segments", "Current WAL segments").unwrap());

pub static STORAGE_BYTES_TOTAL: Lazy<GaugeVec> = Lazy::new(|| {
    register_gauge_vec!("storage_bytes_total", "Bytes by storage kind", &["kind"]).unwrap()
});

pub static WATCH_DROPS_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!("watch_drops_total", "Watch drops by reason", &["reason"]).unwrap()
});

pub static WATCH_BACKLOG_EVENTS: Lazy<GaugeVec> = Lazy::new(|| {
    register_gauge_vec!(
        "watch_backlog_events",
        "Max backlog (events) per namespace",
        &["ns"]
    )
    .unwrap()
});

pub static WATCH_EMIT_LAG_SEC: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "watch_emit_lag_seconds",
        "now - commit_ts for emitted events",
        vec![0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0]
    )
    .unwrap()
});

pub static SNAPSHOT_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!("snapshot_total", "Snapshots by result", &["result"]).unwrap()
});

pub static SNAPSHOT_DURATION_SEC: Lazy<Histogram> =
    Lazy::new(|| register_histogram!("snapshot_duration_seconds", "Snapshot duration").unwrap());

pub static RESTORE_RUNS_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!("restore_runs_total", "Restore runs by status", &["status"]).unwrap()
});

pub static VECTOR_QUERY_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!("vector_query_seconds", "ANN latency", &["field"]).unwrap()
});

pub static QUERY_PLANNER_MICROS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "query_planner_micros",
        "Explain query plan time (µs)",
        vec![50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0, 10000.0]
    )
    .unwrap()
});

pub static WATCH_CLIENTS: Lazy<GaugeVec> =
    Lazy::new(|| register_gauge_vec!("watch_clients", "Active watch clients", &["proto"]).unwrap());

pub static WATCH_EVENTS_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!("watch_events_total", "Watch events emitted", &["type"]).unwrap()
});

pub static WATCH_RESUMES_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!("watch_resumes_total", "Watch resumes", &["proto"]).unwrap()
});
