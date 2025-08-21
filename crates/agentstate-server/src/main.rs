use agentstate_core::{PutRequest, QueryRequest};
use agentstate_storage::{InMemoryStore, PersistentStore, Storage};
use axum::http::StatusCode;
use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use once_cell::sync::Lazy;
use opentelemetry_sdk;
use opentelemetry_otlp::WithExportConfig;
use tracing_subscriber::prelude::*;
use prometheus::{
    Encoder, HistogramVec, IntCounter, IntCounterVec, IntGaugeVec, Registry, TextEncoder,
};
use serde_json::json;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, Level};
mod metrics;
use metrics::{WATCH_CLIENTS, WATCH_EVENTS_TOTAL, WATCH_RESUMES_TOTAL};
use futures::Stream;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use axum::body::Bytes;
use std::path::Path as StdPath;
use std::pin::Pin;
use tonic::{transport::Server as GrpcServer, Request, Response as TonicResponse, Status};

#[derive(Clone)]
struct AppState {
    store: Arc<dyn Storage>,
    // simple in-memory idempotency cache: (ns, key)-> (ts, response json)
    idem: Arc<
        parking_lot::RwLock<
            std::collections::HashMap<(String, String), (std::time::Instant, serde_json::Value)>,
        >,
    >,
    // rate limiters keyed by cap token identity (kid+jti)
    qps:
        Arc<parking_lot::RwLock<std::collections::HashMap<String, (f64, std::time::Instant, u64)>>>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Tracing + optional OTLP
    let otlp = std::env::var("OTLP_ENDPOINT").ok();
    if let Some(endpoint) = otlp {
        let tracer = opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(
                opentelemetry_otlp::new_exporter()
                    .tonic()
                    .with_endpoint(endpoint),
            )
            .install_batch(opentelemetry_sdk::runtime::Tokio)
            .ok();
        if let Some(tracer) = tracer {
            let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
            let subscriber = tracing_subscriber::registry()
                .with(tracing_subscriber::fmt::layer())
                .with(telemetry);
            tracing::subscriber::set_global_default(subscriber).ok();
        } else {
            tracing_subscriber::fmt()
                .with_max_level(Level::INFO)
                .with_env_filter("info")
                .init();
        }
    } else {
        tracing_subscriber::fmt()
            .with_max_level(Level::INFO)
            .with_env_filter("info")
            .init();
    }

    let store: Arc<dyn Storage> = if let Ok(dir) = std::env::var("DATA_DIR") {
        match PersistentStore::open(std::path::PathBuf::from(dir)) {
            Ok(p) => Arc::new(p),
            Err(e) => {
                tracing::warn!("persistent open failed: {} — falling back to memory", e);
                Arc::new(InMemoryStore::new())
            }
        }
    } else {
        Arc::new(InMemoryStore::new())
    };
    let state = AppState {
        store,
        idem: Arc::new(parking_lot::RwLock::new(std::collections::HashMap::new())),
        qps: Arc::new(parking_lot::RwLock::new(std::collections::HashMap::new())),
    };
    
    let store_for_backlog = state.store.clone();
    let sweeper_state = state.clone();
    let snapshot_state = state.clone();
    let grpc_state = state.clone();

    // Metrics registry (MVP)
    static REGISTRY: Lazy<Registry> = Lazy::new(Registry::new);
    static OPS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
        IntCounterVec::new(prometheus::opts!("agentstate_ops_total", "ops"), &["op"]).unwrap()
    });
    static OP_DURATION: Lazy<HistogramVec> = Lazy::new(|| {
        HistogramVec::new(
            prometheus::opts!("op_duration_seconds", "op durations").into(),
            &["op"],
        )
        .unwrap()
    });
    static WAL_FSYNC_TOTAL: Lazy<IntCounter> =
        Lazy::new(|| IntCounter::new("wal_fsync_total", "wal fsyncs").unwrap());
    static WATCH_CLIENTS: Lazy<IntGaugeVec> = Lazy::new(|| {
        IntGaugeVec::new(
            prometheus::opts!("watch_clients", "active watch clients"),
            &["proto"],
        )
        .unwrap()
    });
    static WATCH_EVENTS_TOTAL: Lazy<IntCounter> =
        Lazy::new(|| IntCounter::new("watch_events_total", "watch events emitted").unwrap());
    static WATCH_RESUMES_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
        IntCounterVec::new(
            prometheus::opts!("watch_resumes_total", "watch resumes"),
            &["proto"],
        )
        .unwrap()
    });
    let _ = REGISTRY.register(Box::new(OPS_TOTAL.clone()));
    let _ = REGISTRY.register(Box::new(OP_DURATION.clone()));
    let _ = REGISTRY.register(Box::new(WAL_FSYNC_TOTAL.clone()));
    let _ = REGISTRY.register(Box::new(WATCH_CLIENTS.clone()));
    let _ = REGISTRY.register(Box::new(WATCH_EVENTS_TOTAL.clone()));
    let _ = REGISTRY.register(Box::new(WATCH_RESUMES_TOTAL.clone()));
    // Register dashboard metrics
    let _ = prometheus::default_registry().register(Box::new(metrics::WAL_ACTIVE_SEGMENTS.clone()));
    let _ = prometheus::default_registry().register(Box::new(metrics::STORAGE_BYTES_TOTAL.clone()));
    let _ = prometheus::default_registry().register(Box::new(metrics::WATCH_DROPS_TOTAL.clone()));
    let _ =
        prometheus::default_registry().register(Box::new(metrics::WATCH_BACKLOG_EVENTS.clone()));
    let _ = prometheus::default_registry().register(Box::new(metrics::WATCH_EMIT_LAG_SEC.clone()));
    let _ = prometheus::default_registry().register(Box::new(metrics::SNAPSHOT_TOTAL.clone()));
    let _ =
        prometheus::default_registry().register(Box::new(metrics::SNAPSHOT_DURATION_SEC.clone()));
    let _ =
        prometheus::default_registry().register(Box::new(metrics::VECTOR_QUERY_SECONDS.clone()));
    let _ =
        prometheus::default_registry().register(Box::new(metrics::QUERY_PLANNER_MICROS.clone()));

    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/:ns/objects", post(put_objects))
        .route("/v1/:ns/objects/:id", get(get_object).delete(delete_object))
        .route("/v1/:ns/query", post(query))
        .route("/v1/:ns/watch", get(watch_sse))
        .route("/v1/:ns/lease/acquire", post(lease_acquire))
        .route("/v1/:ns/lease/renew", post(lease_renew))
        .route("/v1/:ns/lease/release", post(lease_release))
        .route("/admin/snapshot", post(admin_snapshot))
        .route("/admin/manifest", get(admin_manifest))
        .route("/admin/trim-wal", post(admin_trim_wal))
        .route("/admin/explain-query", post(admin_explain_query))
        .route("/admin/dump", get(admin_dump))
        .route("/metrics", get(metrics))
        .with_state(state)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );

    let http_addr: SocketAddr = "0.0.0.0:8080".parse().unwrap();
    let grpc_addr: SocketAddr = "0.0.0.0:9090".parse().unwrap();
    info!("http listening on {}", http_addr);
    info!("grpc listening on {}", grpc_addr);

    // TTL sweeper
    tokio::spawn(async move {
        loop {
            let _ = sweeper_state.store.sweep_expired(0).await; // retention window unused in mem engine
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        }
    });

    // Snapshotter (if persistent store)
    if std::env::var("DATA_DIR").is_ok() {
        tokio::spawn(async move {
            loop {
                // downcast to PersistentStore via Any is non-trivial; call via HTTP admin in future.
                // For now, do nothing; placeholder for snapshot scheduling.
                tokio::time::sleep(std::time::Duration::from_secs(300)).await;
            }
        });
        // Data dir scanner for storage_bytes_total
        let data_dir = std::env::var("DATA_DIR").unwrap();
        tokio::spawn(async move {
            loop {
                let wal = dir_size(std::path::Path::new(&data_dir).join("wal"));
                let snaps = dir_size(std::path::Path::new(&data_dir).join("snapshots"));
                metrics::STORAGE_BYTES_TOTAL
                    .with_label_values(&["wal"])
                    .set(wal as f64);
                metrics::STORAGE_BYTES_TOTAL
                    .with_label_values(&["snapshots"])
                    .set(snaps as f64);
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            }
        });
        // Backlog gauge updater (for InMemoryStore)
        tokio::spawn(async move {
            loop {
                let map = store_for_backlog.backlog_map();
                for (ns, v) in &map {
                    metrics::WATCH_BACKLOG_EVENTS
                        .with_label_values(&[ns])
                        .set(*v as f64);
                }
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        });
    }

    let use_tls = std::env::var("TLS_CERT_PATH").is_ok() && std::env::var("TLS_KEY_PATH").is_ok();
    let http = if use_tls {
        let cert = std::fs::read(std::env::var("TLS_CERT_PATH").unwrap()).expect("read cert");
        let key = std::fs::read(std::env::var("TLS_KEY_PATH").unwrap()).expect("read key");
        let config = axum_server::tls_rustls::RustlsConfig::from_pem(cert, key)
            .await
            .expect("tls");
        tokio::spawn(async move {
            axum_server::bind_rustls(http_addr, config)
                .serve(app.into_make_service())
                .await
                .unwrap();
        })
    } else {
        tokio::spawn(async move {
            axum_server::bind(http_addr)
                .serve(app.into_make_service())
                .await
                .unwrap();
        })
    };
    let grpc = {
        let svc = AgentStateGrpc {
            state: grpc_state,
        };
        let mut builder = GrpcServer::builder();
        if use_tls {
            let cert = std::fs::read(std::env::var("TLS_CERT_PATH").unwrap()).expect("read cert");
            let key = std::fs::read(std::env::var("TLS_KEY_PATH").unwrap()).expect("read key");
            let identity = tonic::transport::Identity::from_pem(cert, key);
            let mut tls = tonic::transport::ServerTlsConfig::new().identity(identity);
            if let Ok(ca) = std::env::var("TLS_CLIENT_CA_PATH").map(std::fs::read) {
                if let Ok(ca_bytes) = ca {
                    let ca = tonic::transport::Certificate::from_pem(ca_bytes);
                    tls = tls.client_ca_root(ca);
                }
            }
            builder = builder.tls_config(tls).expect("tls config");
        }
        tokio::spawn(async move {
            builder
                .add_service(agentstate_v1::agent_state_server::AgentStateServer::new(
                    svc,
                ))
                .serve(grpc_addr)
                .await
                .unwrap();
        })
    };

    tokio::try_join!(http, grpc).map(|_| ())?;
    Ok(())
}

fn dir_size(path: impl AsRef<StdPath>) -> u64 {
    fn walk(p: &StdPath, acc: &mut u64) {
        if let Ok(md) = std::fs::metadata(p) {
            if md.is_file() {
                *acc += md.len();
                return;
            }
        }
        if let Ok(rd) = std::fs::read_dir(p) {
            for e in rd.flatten() {
                walk(&e.path(), acc);
            }
        }
    }
    let mut total = 0u64;
    walk(path.as_ref(), &mut total);
    total
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

async fn put_objects(
    State(app): State<AppState>,
    Path(ns): Path<String>,
    headers: HeaderMap,
    Json(req): Json<PutRequest>,
) -> impl IntoResponse {
    let claims = match enforce_caps(&headers, &ns, "put") {
        Ok(c) => c,
        Err(resp) => return resp.into_response(),
    };
    if let Err(resp) = rate_limit(&app, &claims) {
        return resp.into_response();
    }
    // Region pin
    if let Some(reg) = claims.get("region").and_then(|v| v.as_str()) {
        if let Ok(srv) = std::env::var("REGION") {
            if !srv.is_empty() && srv != reg {
                return (
                    StatusCode::UNAVAILABLE_FOR_LEGAL_REASONS,
                    Json(json!({"error":"region_mismatch"})),
                )
                    .into_response();
            }
        }
    }
    // Size limits
    if let Some(maxb) = claims.get("max_bytes").and_then(|v| v.as_u64()) {
        if let Some(cl) = headers
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
        {
            if cl > maxb {
                return (
                    StatusCode::PAYLOAD_TOO_LARGE,
                    Json(json!({"error":"too_large"})),
                )
                    .into_response();
            }
        }
    }
    // Optional lease fencing
    if let Some(resource) = headers.get("If-Resource").and_then(|v| v.to_str().ok()) {
        match headers
            .get("If-Fence")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
        {
            Some(f) => {
                if let Err(e) = app.store.validate_fence(&ns, resource, f).await {
                    return (StatusCode::CONFLICT, Json(json!({"error": e.to_string()})))
                        .into_response();
                }
            }
            None => {
                return (StatusCode::CONFLICT, Json(json!({"error":"missing fence"})))
                    .into_response()
            }
        }
    }
    let _timer = {
        // local static
        static OP_DURATION: Lazy<HistogramVec> = Lazy::new(|| {
            HistogramVec::new(
                prometheus::opts!("op_duration_seconds", "op durations").into(),
                &["op"],
            )
            .unwrap()
        });
        OP_DURATION.with_label_values(&["put"]).start_timer()
    };
    // Idempotency key support
    if let Some(key) = headers.get("Idempotency-Key").and_then(|v| v.to_str().ok()) {
        // persisted idempotency
        let body_hash =
            agentstate_core::util::blake3_hex(serde_json::to_vec(&req).unwrap().as_slice());
        if let Ok(Some(rec)) = app.store.idempotency_lookup(&ns, key, &body_hash).await {
            return (StatusCode::OK, Json(rec.response)).into_response();
        }
        match app.store.put(&ns, req).await {
            Ok(obj) => {
                let val = serde_json::to_value(&obj).unwrap_or(json!({"id": obj.id}));
                let _ = app
                    .store
                    .idempotency_commit(
                        &ns,
                        key,
                        &body_hash,
                        val.clone(),
                        obj.commit_seq,
                        chrono::Utc::now() + chrono::Duration::minutes(10),
                    )
                    .await;
                {
                    static OPS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
                        IntCounterVec::new(
                            prometheus::opts!("agentstate_ops_total", "ops"),
                            &["op"],
                        )
                        .unwrap()
                    });
                    OPS_TOTAL.with_label_values(&["put"]).inc();
                }
                (StatusCode::OK, Json(val)).into_response()
            }
            Err(e) => (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": e.to_string()})),
            )
                .into_response(),
        }
    } else {
        match app.store.put(&ns, req).await {
            Ok(obj) => {
                static OPS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
                    IntCounterVec::new(prometheus::opts!("agentstate_ops_total", "ops"), &["op"])
                        .unwrap()
                });
                OPS_TOTAL.with_label_values(&["put"]).inc();
                (StatusCode::OK, Json(obj)).into_response()
            }
            Err(e) => (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": e.to_string()})),
            )
                .into_response(),
        }
    }
}

#[derive(serde::Deserialize)]
struct GetOpts {
    at: Option<String>,
}

async fn get_object(
    State(app): State<AppState>,
    Path((ns, id)): Path<(String, String)>,
    q: Option<Query<GetOpts>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(resp) = enforce_caps(&headers, &ns, "get") {
        return resp.into_response();
    }
    let _timer = {
        static OP_DURATION: Lazy<HistogramVec> = Lazy::new(|| {
            HistogramVec::new(
                prometheus::opts!("op_duration_seconds", "op durations").into(),
                &["op"],
            )
            .unwrap()
        });
        OP_DURATION.with_label_values(&["get"]).start_timer()
    };
    let at_ts = q
        .and_then(|Query(g)| g.at)
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc));
    match app
        .store
        .get(&ns, &id, agentstate_storage::traits::GetOptions { at_ts })
        .await
    {
        Ok(obj) => {
            static OPS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
                IntCounterVec::new(prometheus::opts!("agentstate_ops_total", "ops"), &["op"])
                    .unwrap()
            });
            OPS_TOTAL.with_label_values(&["get"]).inc();
            (StatusCode::OK, Json(obj)).into_response()
        }
        Err(e) => (StatusCode::NOT_FOUND, Json(json!({"error": e.to_string()}))).into_response(),
    }
}

async fn delete_object(
    State(app): State<AppState>,
    Path((ns, id)): Path<(String, String)>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(resp) = enforce_caps(&headers, &ns, "delete") {
        return resp.into_response();
    }
    match app.store.delete(&ns, &id).await {
        Ok(_) => (StatusCode::NO_CONTENT).into_response(),
        Err(e) => (StatusCode::NOT_FOUND, Json(json!({"error": e.to_string()}))).into_response(),
    }
}

async fn query(
    State(app): State<AppState>,
    Path(ns): Path<String>,
    headers: HeaderMap,
    Json(req): Json<QueryRequest>,
) -> impl IntoResponse {
    if let Err(resp) = enforce_caps(&headers, &ns, "query") {
        return resp.into_response();
    }
    let _timer = {
        static OP_DURATION: Lazy<HistogramVec> = Lazy::new(|| {
            HistogramVec::new(
                prometheus::opts!("op_duration_seconds", "op durations").into(),
                &["op"],
            )
            .unwrap()
        });
        OP_DURATION.with_label_values(&["query"]).start_timer()
    };
    match app.store.query(&ns, req).await {
        Ok(list) => {
            static OPS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
                IntCounterVec::new(prometheus::opts!("agentstate_ops_total", "ops"), &["op"])
                    .unwrap()
            });
            OPS_TOTAL.with_label_values(&["query"]).inc();
            (StatusCode::OK, Json(list)).into_response()
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn watch_sse(
    State(app): State<AppState>,
    Path(ns): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(resp) = enforce_caps(&headers, &ns, "watch") {
        return resp.into_response();
    }
    // Manual SSE stream with decrement on drop
    struct ClientGuard(&'static str);
    impl ClientGuard {
        fn inc(proto: &'static str) -> Self {
            WATCH_CLIENTS.with_label_values(&[proto]).inc();
            ClientGuard(proto)
        }
    }
    impl Drop for ClientGuard {
        fn drop(&mut self) {
            WATCH_CLIENTS.with_label_values(&[self.0]).dec();
        }
    }

    let mut handle = app.store.subscribe(
        agentstate_storage::traits::WatchFilter { ns: ns.clone() },
        None,
    );
    let guard = ClientGuard::inc("sse");
    let s = async_stream::stream! {
        let _g = guard;
        loop {
            if let Some((last, _retry)) = handle.overflow_meta() {
                metrics::WATCH_DROPS_TOTAL.with_label_values(&["overflow"]).inc();
                let payload = serde_json::to_string(&json!({"error":"overflow","last_commit":last})).unwrap();
                let chunk = format!("id: {}\ndata: {}\n\n", last, payload);
                yield Ok::<Bytes, std::io::Error>(Bytes::from(chunk));
                break;
            } else if let Some(ev) = handle.try_next() {
                match ev {
                    agentstate_storage::traits::WatchEvent::Put(o) => {
                        WATCH_EVENTS_TOTAL.with_label_values(&["put"]).inc();
                        let lag = (chrono::Utc::now() - o.ts).num_milliseconds() as f64 / 1000.0;
                        metrics::WATCH_EMIT_LAG_SEC.observe(lag.max(0.0));
                        let payload = serde_json::to_string(&json!({"type":"put","obj":o,"commit_seq":o.commit_seq})).unwrap();
                        let chunk = format!("id: {}\ndata: {}\n\n", o.commit_seq, payload);
                        yield Ok::<Bytes, std::io::Error>(Bytes::from(chunk));
                    }
                    agentstate_storage::traits::WatchEvent::Delete{ns,id,commit_seq} => {
                        WATCH_EVENTS_TOTAL.with_label_values(&["put"]).inc();
                        let payload = serde_json::to_string(&json!({"type":"delete","ns":ns,"id":id,"commit_seq":commit_seq})).unwrap();
                        let chunk = format!("id: {}\ndata: {}\n\n", commit_seq, payload);
                        yield Ok::<Bytes, std::io::Error>(Bytes::from(chunk));
                    }
                }
            } else {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }
    };
    axum::http::Response::builder()
        .header(axum::http::header::CONTENT_TYPE, "text/event-stream")
        .header(axum::http::header::CACHE_CONTROL, "no-cache")
        .body(axum::body::Body::from_stream(s))
        .unwrap()
}

async fn metrics() -> impl IntoResponse {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buf = Vec::new();
    let _ = encoder.encode(&metric_families, &mut buf);
    (StatusCode::OK, String::from_utf8(buf).unwrap_or_default())
}

// Admin endpoints
async fn admin_snapshot(State(app): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    if let Err(resp) = enforce_caps(&headers, "admin://global", "admin") {
        return resp.into_response();
    }
    let t0 = std::time::Instant::now();
    match app.store.admin_snapshot().await {
        Ok((id, last_seq)) => {
            metrics::SNAPSHOT_TOTAL.with_label_values(&["ok"]).inc();
            metrics::SNAPSHOT_DURATION_SEC.observe(t0.elapsed().as_secs_f64());
            (
                StatusCode::OK,
                Json(json!({"snapshot_id": id, "last_seq": last_seq})),
            )
                .into_response()
        }
        Err(e) => {
            metrics::SNAPSHOT_TOTAL.with_label_values(&["error"]).inc();
            metrics::SNAPSHOT_DURATION_SEC.observe(t0.elapsed().as_secs_f64());
            (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }
}
async fn admin_manifest(State(app): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    if let Err(resp) = enforce_caps(&headers, "admin://global", "admin") {
        return resp.into_response();
    }
    match app.store.admin_manifest().await {
        Ok(m) => {
            if let Some(segs) = m.get("segments").and_then(|v| v.as_array()) {
                metrics::WAL_ACTIVE_SEGMENTS.set(segs.len() as f64);
            }
            (StatusCode::OK, Json(m)).into_response()
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn admin_dump(State(app): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    if let Err(resp) = enforce_caps(&headers, "admin://global", "admin") {
        return resp.into_response();
    }
    match app.store.admin_snapshot().await {
        Ok((snapshot_id, _)) => {
            let mut response = String::new();
            for obj in app.store.all_objects() {
                if let Ok(json) = serde_json::to_string(&obj) {
                    response.push_str(&json);
                    response.push('\n');
                }
            }
            (StatusCode::OK, response).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()}))
        ).into_response(),
    }
}

async fn admin_trim_wal(
    State(app): State<AppState>,
    q: Option<Query<std::collections::HashMap<String, String>>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(resp) = enforce_caps(&headers, "admin://global", "admin") {
        return resp.into_response();
    }
    let sid = q
        .and_then(|Query(m)| m.get("snapshot_id").cloned())
        .unwrap_or_default();
    if sid.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":"missing snapshot_id"})),
        )
            .into_response();
    }
    match app.store.admin_trim_wal(&sid).await {
        Ok(deleted) => {
            if let Ok(m) = app.store.admin_manifest().await {
                if let Some(segs) = m.get("segments").and_then(|v| v.as_array()) {
                    metrics::WAL_ACTIVE_SEGMENTS.set(segs.len() as f64);
                }
            }
            (StatusCode::OK, Json(json!({"deleted": deleted}))).into_response()
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

#[derive(serde::Deserialize)]
struct ExplainReq {
    ns: String,
    #[serde(default)]
    filter: Option<serde_json::Value>,
    #[serde(default)]
    top_k_vec: Option<serde_json::Value>,
    #[serde(default)]
    fields: Option<Vec<String>>,
}

async fn admin_explain_query(
    State(_app): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<ExplainReq>,
) -> impl IntoResponse {
    if let Err(resp) = enforce_caps(&headers, &req.ns, "admin") {
        return resp.into_response();
    }
    let t0 = std::time::Instant::now();
    let mut plan: Vec<serde_json::Value> = Vec::new();
    if let Some(f) = &req.filter {
        if let Some(tags) = f.get("tags").and_then(|t| t.as_object()) {
            for (k, _v) in tags {
                plan.push(json!({"op":"filter","on": format!("tags.{}", k), "index":"tags","selectivity":0.5}));
            }
        }
        if let Some(jp) = f.get("jsonpath").and_then(|j| j.as_str()) {
            plan.push(json!({"op":"filter","on": format!("jsonpath:{}", jp), "index":"jsonpath","selectivity":0.5}));
        }
    }
    if let Some(vq) = &req.top_k_vec {
        if let Some(field) = vq.get("field").and_then(|v| v.as_str()) {
            plan.push(json!({"op":"ann","field":field,"algo":"hnsw","candidates":5000,"k": vq.get("k").and_then(|k| k.as_u64()).unwrap_or(8)}));
        }
    }
    let resp = json!({
        "plan": plan,
        "estimated_cost": {"cpu_ms": 7.2, "io_reads": 34},
        "indexes_hit": plan.iter().filter_map(|p| p.get("index")).collect::<Vec<_>>(),
        "warnings": Vec::<String>::new(),
    });
    let micros = t0.elapsed().as_micros() as f64;
    metrics::QUERY_PLANNER_MICROS.observe(micros);
    (StatusCode::OK, Json(resp)).into_response()
}

// Leases endpoints
#[derive(serde::Deserialize)]
struct LeaseAcquireReq {
    key: String,
    owner: String,
    ttl: u64,
}
#[derive(serde::Deserialize)]
struct LeaseRenewReq {
    key: String,
    owner: String,
    token: u64,
    ttl: u64,
}
#[derive(serde::Deserialize)]
struct LeaseReleaseReq {
    key: String,
    owner: String,
    token: u64,
}

async fn lease_acquire(
    State(app): State<AppState>,
    Path(ns): Path<String>,
    headers: HeaderMap,
    Json(req): Json<LeaseAcquireReq>,
) -> impl IntoResponse {
    if let Err(resp) = enforce_caps(&headers, &ns, "lease") {
        return resp.into_response();
    }
    match app
        .store
        .lease_acquire(&ns, &req.key, &req.owner, req.ttl)
        .await
    {
        Ok(l) => (
            StatusCode::OK,
            Json(json!({"token": l.token, "expires_at": l.expires_at.to_rfc3339()})),
        )
            .into_response(),
        Err(e) => (StatusCode::CONFLICT, Json(json!({"error": e.to_string()}))).into_response(),
    }
}
async fn lease_renew(
    State(app): State<AppState>,
    Path(ns): Path<String>,
    headers: HeaderMap,
    Json(req): Json<LeaseRenewReq>,
) -> impl IntoResponse {
    if let Err(resp) = enforce_caps(&headers, &ns, "lease") {
        return resp.into_response();
    }
    match app
        .store
        .lease_renew(&ns, &req.key, &req.owner, req.token, req.ttl)
        .await
    {
        Ok(l) => (
            StatusCode::OK,
            Json(json!({"token": l.token, "expires_at": l.expires_at.to_rfc3339()})),
        )
            .into_response(),
        Err(e) => (StatusCode::CONFLICT, Json(json!({"error": e.to_string()}))).into_response(),
    }
}
async fn lease_release(
    State(app): State<AppState>,
    Path(ns): Path<String>,
    headers: HeaderMap,
    Json(req): Json<LeaseReleaseReq>,
) -> impl IntoResponse {
    if let Err(resp) = enforce_caps(&headers, &ns, "lease") {
        return resp.into_response();
    }
    match app
        .store
        .lease_release(&ns, &req.key, &req.owner, req.token)
        .await
    {
        Ok(_) => (StatusCode::NO_CONTENT).into_response(),
        Err(e) => (StatusCode::CONFLICT, Json(json!({"error": e.to_string()}))).into_response(),
    }
}

pub mod agentstate_v1 {
    tonic::include_proto!("agentstate.v1");
}

struct AgentStateGrpc {
    state: AppState,
}

type WatchStream = Pin<Box<dyn Stream<Item = Result<agentstate_v1::WatchEvent, Status>> + Send>>;

#[tonic::async_trait]
impl agentstate_v1::agent_state_server::AgentState for AgentStateGrpc {
    async fn put(
        &self,
        request: Request<agentstate_v1::PutRequest>,
    ) -> Result<TonicResponse<agentstate_v1::Object>, Status> {
        let req = request.into_inner();
        let pr = PutRequest {
            r#type: req.r#type,
            body: serde_json::from_str(&req.body_json).unwrap_or(serde_json::Value::Null),
            tags: agentstate_core::Tags(req.tags.into_iter().collect()),
            ttl_seconds: if req.ttl_seconds == 0 {
                None
            } else {
                Some(req.ttl_seconds as u64)
            },
            id: if req.id.is_empty() {
                None
            } else {
                Some(req.id)
            },
            parents: req.parents,
        };
        let o = self
            .state
            .store
            .put(&req.ns, pr)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(TonicResponse::new(to_proto_object(o)))
    }

    async fn get(
        &self,
        request: Request<agentstate_v1::GetRequest>,
    ) -> Result<TonicResponse<agentstate_v1::Object>, Status> {
        let req = request.into_inner();
        let o = self
            .state
            .store
            .get(
                &req.ns,
                &req.id,
                agentstate_storage::traits::GetOptions { at_ts: None },
            )
            .await
            .map_err(|e| Status::not_found(e.to_string()))?;
        Ok(TonicResponse::new(to_proto_object(o)))
    }

    async fn query(
        &self,
        request: Request<agentstate_v1::QueryRequest>,
    ) -> Result<TonicResponse<agentstate_v1::QueryResponse>, Status> {
        let req = request.into_inner();
        let tag_filter = if req.tag_json.is_empty() {
            None
        } else {
            Some(agentstate_core::TagFilter(
                serde_json::from_str(&req.tag_json).unwrap_or_default(),
            ))
        };
        let qr = QueryRequest {
            tag_filter,
            jsonpath: if req.jsonpath.is_empty() {
                None
            } else {
                Some(agentstate_core::JsonPathFilter {
                    equals: std::collections::BTreeMap::from([("$".to_string(), serde_json::Value::String(req.jsonpath))]),
                })
            },
            vector: None,
            limit: None,
            fields: None,
        };
        let list = self
            .state
            .store
            .query(&req.ns, qr)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(TonicResponse::new(agentstate_v1::QueryResponse {
            objects: list.into_iter().map(to_proto_object).collect(),
        }))
    }

    async fn delete(
        &self,
        request: Request<agentstate_v1::DeleteRequest>,
    ) -> Result<TonicResponse<agentstate_v1::Empty>, Status> {
        let req = request.into_inner();
        self.state
            .store
            .delete(&req.ns, &req.id)
            .await
            .map_err(|e| Status::not_found(e.to_string()))?;
        Ok(TonicResponse::new(agentstate_v1::Empty {}))
    }

    type WatchStream = WatchStream;
    async fn watch(
        &self,
        request: Request<agentstate_v1::WatchRequest>,
    ) -> Result<TonicResponse<Self::WatchStream>, Status> {
        let req = request.into_inner();
        let mut handle = self.state.store.subscribe(
            agentstate_storage::traits::WatchFilter { ns: req.ns.clone() },
            Some(req.from_commit),
        );
        if req.from_commit > 0 {
            WATCH_RESUMES_TOTAL.with_label_values(&["grpc"]).inc();
        }
        WATCH_CLIENTS.with_label_values(&["grpc"]).inc();
        let output = async_stream::try_stream! {
            loop {
                if let Some((last, retry)) = handle.overflow_meta() {
                    // terminate with RESOURCE_EXHAUSTED; include info in message (trailers API is limited here)
                    metrics::WATCH_DROPS_TOTAL.with_label_values(&["overflow"]).inc();
                    Err(Status::resource_exhausted(format!("overflow last_commit={} retry_after_ms={}", last, retry)))?;
                } else if let Some(ev) = handle.try_next() {
                    match ev {
                        agentstate_storage::traits::WatchEvent::Put(o) => {
                            WATCH_EVENTS_TOTAL.with_label_values(&["put"]).inc();
                            let lag = (chrono::Utc::now() - o.ts).num_milliseconds() as f64 / 1000.0;
                            metrics::WATCH_EMIT_LAG_SEC.observe(lag.max(0.0));
                            yield agentstate_v1::WatchEvent { r#type: "put".into(), obj: Some(to_proto_object(o.clone())), id: o.id.clone(), commit: o.commit_seq };
                        }
                        agentstate_storage::traits::WatchEvent::Delete{ns:_, id, commit_seq} => {
                            WATCH_EVENTS_TOTAL.with_label_values(&["put"]).inc();
                            yield agentstate_v1::WatchEvent { r#type: "delete".into(), obj: None, id, commit: commit_seq };
                        }
                    }
                } else {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
            }
        };
        Ok(TonicResponse::new(Box::pin(output) as WatchStream))
    }
}

fn to_proto_object(o: agentstate_core::Object) -> agentstate_v1::Object {
    agentstate_v1::Object {
        id: o.id,
        ns: o.ns,
        r#type: o.r#type,
        body_json: serde_json::to_string(&o.body).unwrap_or("null".into()),
        tags: o.tags.0.into_iter().collect(),
        ttl_seconds: o.ttl_seconds.unwrap_or_default() as u64,
        parents: o.parents,
        commit: o.commit,
        ts_rfc3339: o.ts.to_rfc3339(),
    }
}

// Capability token enforcement (simple HMAC signed JSON)
fn enforce_caps(
    headers: &HeaderMap,
    ns: &str,
    verb: &str,
) -> Result<serde_json::Value, (StatusCode, Json<serde_json::Value>)> {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD as b64, Engine};
    // Dual keys: CAP_KEY_ACTIVE, CAP_KEY_NEXT; token format: kid.payload.sig
    let active = std::env::var("CAP_KEY_ACTIVE").ok();
    let next = std::env::var("CAP_KEY_NEXT").ok();
    if active.is_none() && next.is_none() {
        return Ok(serde_json::json!({}));
    }
    let auth = match headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    {
        Some(s) if s.starts_with("Bearer ") => &s[7..],
        _ => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({"error":"missing token"})),
            ))
        }
    };
    let parts: Vec<&str> = auth.split('.').collect();
    if parts.len() != 3 {
        return Err((StatusCode::UNAUTHORIZED, Json(json!({"error":"bad token"}))));
    }
    let kid = parts[0];
    let payload = b64
        .decode(parts[1])
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(json!({"error":"bad b64"}))))?;
    let sig_bytes = b64
        .decode(parts[2])
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(json!({"error":"bad b64"}))))?;
    let secret = match kid {
        k if k == std::env::var("CAP_KEY_ACTIVE_ID").unwrap_or("active".into()) => active,
        k if k == std::env::var("CAP_KEY_NEXT_ID").unwrap_or("next".into()) => next,
        _ => None,
    }
    .ok_or((
        StatusCode::UNAUTHORIZED,
        Json(json!({"error":"unknown kid"})),
    ))?;
    let mut mac = <Hmac<Sha256>>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(&payload);
    let sig = mac.finalize().into_bytes();
    if sig.as_slice() != sig_bytes.as_slice() {
        return Err((StatusCode::UNAUTHORIZED, Json(json!({"error":"bad sig"}))));
    }
    let mut claims: serde_json::Value = serde_json::from_slice(&payload).map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error":"bad claims"})),
        )
    })?;
    // inject kid for tracing/audit
    if let serde_json::Value::Object(ref mut map) = claims {
        map.insert("kid".into(), serde_json::Value::String(kid.to_string()));
    }
    // ns check
    if let Some(arr) = claims.get("ns").and_then(|v| v.as_array()) {
        if !arr.iter().any(|v| v.as_str() == Some(ns)) {
            return Err((StatusCode::FORBIDDEN, Json(json!({"error":"ns denied"}))));
        }
    }
    // verbs check
    if let Some(arr) = claims.get("verbs").and_then(|v| v.as_array()) {
        if !arr.iter().any(|v| v.as_str() == Some(verb)) {
            return Err((StatusCode::FORBIDDEN, Json(json!({"error":"verb denied"}))));
        }
    }
    // ttl check
    if let Some(exp) = claims.get("exp").and_then(|v| v.as_i64()) {
        if exp
            < (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64)
        {
            return Err((StatusCode::UNAUTHORIZED, Json(json!({"error":"expired"}))));
        }
    }
    // size limit for PUT
    if verb == "put" {
        if let Some(max) = claims.get("max_bytes").and_then(|v| v.as_u64()) {
            /* enforced in handler if needed */
            let _ = max;
        }
    }
    Ok(claims)
}

fn rate_limit(
    state: &AppState,
    claims: &serde_json::Value,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    let max_qps = claims
        .get("max_qps")
        .and_then(|v| v.as_u64())
        .unwrap_or(u64::MAX);
    if max_qps == u64::MAX {
        return Ok(());
    }
    let kid = claims
        .get("kid")
        .and_then(|v| v.as_str())
        .unwrap_or("active");
    let jti = claims.get("jti").and_then(|v| v.as_str()).unwrap_or("");
    let key = format!("{}:{}", kid, jti);
    let mut map = state.qps.write();
    let now = std::time::Instant::now();
    let (refill_per_s, burst) = (max_qps as f64, (max_qps * 2) as u64);
    let entry = map.entry(key).or_insert((burst as f64, now, burst));
    let elapsed = now.duration_since(entry.1).as_secs_f64();
    entry.0 = (entry.0 + elapsed * refill_per_s).min(burst as f64);
    entry.1 = now;
    if entry.0 >= 1.0 {
        entry.0 -= 1.0;
        Ok(())
    } else {
        Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({"error":"rate_limited"})),
        ))
    }
}
