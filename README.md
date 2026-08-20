# Quick start
After installing the library, you should:

1. Implement required settings traits.
2. Create new service context.
3. Setup what you need.
4. Start service context. 

Minimal sample app:

```rust,no_run
#[tokio::main]
async fn main() {
    let settings_reader = Arc::new(SettingsReader::new("~/.service_settings").await);

    let mut service_context = ServiceContext::new(settings_reader).await;

    // /api/isalive is registered automatically (and /metrics too, with the
    // `with-prometheus-metrics` feature).
    // Use configure_http_server only to add additional routes.
    // service_context.configure_http_server(|http| {
    //     http.register_get_action(Arc::new(GetAction::new()));
    // });

    service_context.start_application().await;
}
```

Settings Model:

Two patterns are supported.

**Recommended — derive macros.** Field names must match what the
auto-derive expects (`seq_conn_string`, `my_telemetry`, plus
feature-gated `postgres_conn_string`, `my_sb_tcp_host_port`,
`my_no_sql_tcp_reader`, `my_no_sql_writer`):

```rust,no_run
service_sdk::macros::use_settings!();

#[derive(SettingsModel, Serialize, Deserialize, Debug, Clone)]
pub struct SettingsModel {
    pub seq_conn_string: String,
    #[serde(default)]
    pub my_telemetry: Option<String>,
}

// `SdkSettingsTraits` generates `impl ServiceInfo for SettingsReader`
// from `CARGO_PKG_NAME` / `CARGO_PKG_VERSION`. Apply it to a struct
// literally named `SettingsReader`.
#[derive(SdkSettingsTraits)]
pub struct SettingsReader {
    pub settings: tokio::sync::RwLock<Arc<SettingsModel>>,
}

// `AutoGenerateSettingsTraits` generates impls for `SeqSettings`,
// `MyTelemetrySettings`, and the feature-gated traits (Postgres / NoSql
// reader+writer / ServiceBus) — all reading from `self.settings.read().await`.
#[derive(AutoGenerateSettingsTraits)]
struct SettingsAutoImpls;
```

YAML shape — snake_case, mirroring the field names:

```yaml
seq_conn_string: http://seq.local:5341
my_telemetry: null
```

**Manual — without the derive macros.** Useful when field names differ
from the macro-hardcoded ones or values come from non-standard sources:

```rust,no_run
#[derive(SettingsModel, Serialize, Deserialize, Debug, Clone)]
pub struct SettingsModel {
    pub seq_conn_string: String,
    #[serde(default)]
    pub my_telemetry: Option<String>,
}

pub struct SettingsReader { /* your wrapper around the model */ }

#[async_trait::async_trait]
impl my_telemetry_writer::MyTelemetrySettings for SettingsReader {
    async fn get_telemetry_url(&self) -> Option<String> {
        /* ... */
    }
}

#[async_trait::async_trait]
impl my_seq_logger::SeqSettings for SettingsReader {
    async fn get_conn_string(&self) -> String {
        /* ... */
    }
}

impl ServiceInfo for SettingsReader {
    fn get_service_name(&self) -> &'static str {
        env!("CARGO_PKG_NAME")
    }
    fn get_service_version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }
}
```

# Features overview

The following are **always on** (no feature flag required): the `/api/isalive` HTTP endpoint, Seq logger, my-telemetry writer, settings reader, app-states lifecycle. They come built into `service-sdk` and need only their respective settings traits implemented (`SeqSettings`, `MyTelemetrySettings`, `ServiceInfo`).

Prometheus metrics and the `/metrics` endpoint are **not** in that list — they need `with-prometheus-metrics`.

Opt-in features add capabilities on top:

| Feature                       | Enables                                                                                  | Settings traits to implement                                              |
| ----------------------------- | ---------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- |
| `macros`                      | `SdkSettingsTraits` / `AutoGenerateSettingsTraits` derives + `use_settings!()` / `use_grpc_*!()` / `use_my_no_sql_entity!()` / `use_my_postgres!()` etc. (`SettingsModel` derive comes from `my-settings-reader`.) | —                                                                         |
| `my-service-bus`              | `register_sb_subscribe`, `get_sb_publisher`, `get_sb_publisher_with_internal_queue`      | `MyServiceBusSettings` (auto-derived as `my_sb_tcp_host_port`)             |
| `my-nosql-sdk`                | NoSql entity macros only (no I/O)                                                        | —                                                                         |
| `my-nosql-data-reader-sdk`    | `get_ns_reader` returning `MyNoSqlDataReaderTcp<T>`                                      | `MyNoSqlTcpConnectionSettings` (auto-derived as `my_no_sql_tcp_reader`)    |
| `my-nosql-data-writer-sdk`    | Enables `my-no-sql-sdk/data-writer` (use `MyNoSqlDataWriter<T>` directly from `my-no-sql-sdk`) | `MyNoSqlWriterSettings` (auto-derived as `my_no_sql_writer`)               |
| `grpc`                        | `configure_grpc_server` + gRPC client/server macros                                      | —                                                                         |
| `postgres`                    | `my-postgres` integration                                                                | `PostgresSettings` (auto-derived as `postgres_conn_string`)                |
| `with-ring-tls`               | rustls TLS with the **ring** crypto provider. Links `rustls`/`my-tls` and installs the provider. Required for `https://` through fl-url, TLS gRPC and other TLS-bearing transports. Mature and widely deployed, at the cost of a bundled C/assembly build. | — |
| `with-rust-tls`               | The same TLS, with the **pure-Rust** provider (`rustls-graviola`) - no C toolchain anywhere. Builds only on x86_64/aarch64 and is far less deployed than ring, so prefer `with-ring-tls` unless dropping the C toolchain is the point. | — |
| `with-postgres-tls`           | TLS for postgres (`my-postgres/with-tls`, openssl-based - a separate stack from `with-ring-tls`). No-op unless `postgres` is on too. | — |
| `with-ssh`                    | SSH tunnels for gRPC, fl-url, my-no-sql and postgres. Without it no `my-ssh`/`ssh2`/`libssh2-sys` crate is compiled in at all. | — |
| `with-prometheus-metrics`     | Prometheus metrics: the `/metrics` endpoint, HTTP/gRPC request metrics, the `EventsPerSecond` timer and the `service_sdk::metrics` re-export. Without it nothing is reported to prometheus and no `metrics`/`metrics-prometheus`/`prometheus` crate is compiled in at all. | — |
| `http-static-files`           | Static-file middleware in `my-http-server`                                               | —                                                                         |
| `websockets`                  | WebSocket support in `my-http-server`                                                    | —                                                                         |
| `signal-r`                    | SignalR support in `my-http-server`                                                      | —                                                                         |
| `full`                        | All of: `my-service-bus`, `my-nosql-sdk`, `my-nosql-data-reader-sdk`, `my-nosql-data-writer-sdk`, `grpc`, `postgres`, `macros`. **Never** includes `with-ring-tls`, `with-rust-tls`, `with-postgres-tls`, `with-ssh` or `with-prometheus-metrics` - TLS, SSH and metrics never ride along with a convenience feature, they are always requested explicitly. | union of the above |

## TLS and SSH are opt-in

Nothing in the default build - and nothing in `full` - links a TLS or an SSH
stack, and nothing ever will. `full` is a convenience alias for the transport
and codegen features only; TLS and SSH are always named explicitly, e.g.
`--features full,with-ring-tls` - that is two separate features, not one.

`cargo tree` on any feature set that contains none of `with-ring-tls`,
`with-rust-tls`, `with-postgres-tls`, `with-ssh` has no `rustls*`, `ring`,
`graviola`, `rcgen`, `webpki`, `openssl*`, `my-tls`, `my-ssh`, `ssh2` or
`libssh2-sys` crate in it.

The switches are independent because they are genuinely different stacks:

| Feature             | Stack                            | Turned on in                                                        |
| ------------------- | -------------------------------- | ------------------------------------------------------------------- |
| `with-ring-tls`     | rustls + ring (`my-tls`)         | `flurl`, `my-grpc-extensions`, plus `my_tls::install_default_crypto_providers()` in `ServiceContext::new` |
| `with-rust-tls`     | rustls + graviola (`my-tls`)     | the same three places - only the provider differs                    |
| `with-postgres-tls` | openssl (`postgres-openssl`)     | `my-postgres`                                                       |
| `with-ssh`          | libssh2 (`my-ssh`)               | `flurl`, `my-grpc-extensions`, `my-no-sql-sdk`, `my-postgres`        |

### Picking a crypto provider

`with-ring-tls` and `with-rust-tls` are alternatives, not a stack: both give the
same TLS functionality and differ only in the rustls `CryptoProvider`. Pick one.

Enabling both is allowed rather than a build error - `--all-features` does
exactly that - and `my-tls` resolves it in favour of ring, so the more
battle-tested provider wins. Note that both providers are then compiled in, so
this is not a configuration to ship deliberately.

Postgres TLS is unrelated to this choice: it speaks openssl, not rustls, so
`with-postgres-tls` neither implies nor is implied by a provider feature.

`with-postgres-tls` and `with-ssh` are written with optional-dependency syntax
(`my-postgres?/with-tls`), so they are silently a no-op when the crate they
target is not enabled - enabling `with-postgres-tls` without `postgres` does not
pull postgres in.

**Watch out:** without `with-ring-tls`, fl-url has no TLS stack linked, and it
`panic!`s at request time on any `https://` url. The always-on parts of the SDK
go through fl-url - the Seq logger, the telemetry writer and the http settings
reader - so if any of those endpoints is `https://`, `with-ring-tls` is
mandatory. Turn it on whenever the service talks to anything over https.

# Metrics

Metrics require the `with-prometheus-metrics` feature. With it, gRPC and HTTP request metrics are collected automatically and served at `/metrics`:

```toml
service-sdk = { tag = "...", git = "...", features = ["full", "with-prometheus-metrics"] }
```

Without the feature nothing is reported to prometheus: there is no `/metrics` endpoint, no HTTP or gRPC metrics middleware, no `EventsPerSecond` timer, no `service_sdk::metrics` re-export, and no `metrics` / `metrics-prometheus` / `prometheus` crate in the dependency graph (which also drops `protobuf`, pulled in transitively by `prometheus`).

`register_events_per_second()` and `EventsPerSecondCounter` are the one exception: they exist in **both** modes with identical signatures, so service code that registers and increments counters compiles unchanged either way. With the feature off they are no-ops.

| Type | Feature                                | Description                          | Labels                    |
| ---- | -------------------------------------- | ------------------------------------ | ------------------------- |
| HTTP | http_failed_request_count              | Count of failed HTTP requests        | method, path, status_code |
| HTTP | http_failed_request_milis_duration_sum | Duration sum of failed HTTP request  | method, path, status_code |
| HTTP | http_failed_request_duration_sec       | Histogram of failed request duration | method, path, status_code |
| HTTP | http_request_duration_sec              | Histogram of request duration        | method, path              |
| HTTP | http_request_milis_duration_sum        | Duration sum of HTTP request         | method, path              |
| HTTP | http_request_count                     | Count of HTTP requests               | method, path              |
| GRPC | grpc_request_duration_sec              | Grpc request duration histogram      | method, path              |
| GRPC | grpc_request_duration_milis_sum        | Sum of request grpc request durations requests               | method, path              |
| GRPC | grpc_request_count                     | Count of GRPC requests               | method, path              |
                                                                                                                    
### Custom metrics
Also if you need - you can create you own metrics. `service_sdk::metrics` is re-exported only under `with-prometheus-metrics`, so these calls need the feature:

```rust, no_run
let common_labels = &[
        ("method", method),
        ("path", path),
        ("status_code", response.status().to_string()),
    ];

//counter
service_sdk::metrics::counter!("my_metric_counter", common_labels)
    .increment(1);
//gauge
service_sdk::metrics::gauge!("my_metric_gauge", common_labels)
    .increment(1);
//histogram
service_sdk::metrics::histogram!("my_metric_histogram", common_labels)
    .record(duration.as_secs_f64());
```

### Events-per-second metrics

If you want a gauge that exposes "events per second" (events accumulated over the last second), register an `EventsPerSecondCounter` once on the `ServiceContext` and just `.increment()` on the returned handle from anywhere in your code. The SDK runs an internal 1-second background timer that snapshots the accumulated value, resets the counter to zero, and emits a Prometheus gauge under the name you registered. The metric name is used as-is — no suffix is added.

Unlike the rest of this section, this API does not need `#[cfg]` on the caller's side: it compiles with or without `with-prometheus-metrics`. Without the feature the counter is a no-op — nothing accumulates, no timer runs and no gauge is published.

```rust, no_run
let counter = service_context.register_events_per_second("my_events_per_second");

// +1, no labels
counter.increment();

// +N, no labels
counter.increment_by(5);

// +1, with labels — label sets are dynamic, new combinations appear
// in /metrics on the next tick automatically.
counter.increment_with_labels(&[("endpoint", "foo")]);

// +N, with labels
counter.increment_by_with_labels(3, &[("endpoint", "bar")]);
```

The resulting `/metrics` output looks like:

```text
my_events_per_second 12
my_events_per_second{endpoint="foo"} 7
my_events_per_second{endpoint="bar"} 3
```

# Service Bus
`register_sb_subscribe(callback, delete_on_no_subscribers, single_connection)` — synchronous.

```rust, no_run
let service_context = ServiceContext::new(settings_reader).await;
service_context.register_sb_subscribe(
    Arc::new(CallbackAccountsSenderJob::new()),
    false, // delete_on_no_subscribers
    true,  // single_connection
);
```

`get_sb_publisher(do_retries)` — pass `true` to wrap the publisher with retry logic, `false` for fire-and-forget.

```rust, no_run
let service_context = ServiceContext::new(settings_reader).await;
let sb_publisher: MyServiceBusPublisher<Model> = service_context.get_sb_publisher(true);
```

`get_sb_publisher_with_internal_queue`
```rust, no_run
let service_context = ServiceContext::new(settings_reader).await;
let sb_publisher: PublisherWithInternalQueue<Model> = service_context.get_sb_publisher_with_internal_queue();
```

# GRPC Client

`use_grpc_client!()` pulls in everything `#[generate_grpc_client]` expands to. Urls are resolved through `GrpcClientSettings::get_grpc_url(name)`.

```rust, no_run
service_sdk::macros::use_grpc_client!();

#[generate_grpc_client(
    proto_file: "./proto/KeyValue.proto",
    crate_ns: "crate::keyvalue_grpc",
    retries: 3,
    request_timeout_sec: 5,
    ping_timeout_sec: 5,
    ping_interval_sec: 5,
)]
pub struct KeyValueGrpcClient;
```

### Client pool

When the same service has many instances, each addressed by a runtime key (shard / tenant / node), use `use_grpc_client_pool!()` with `#[generate_grpc_client_pool]`. It generates the same client plus a `{StructName}Pool`.

```rust, no_run
service_sdk::macros::use_grpc_client_pool!();

#[generate_grpc_client_pool(
    proto_file: "./proto/KeyValue.proto",
    crate_ns: "crate::keyvalue_grpc",
    retries: 3,
    request_timeout_sec: 5,
    ping_timeout_sec: 5,
    ping_interval_sec: 5,
)]
pub struct KeyValueGrpcClient;
```

Because each pooled instance may point elsewhere, urls come from `GrpcClientPoolSettings::get_grpc_url(name, id)`, which additionally receives the id. Clients are created lazily per id; `gc` drops every id not passed to it, tearing down its ping loop and channel.

```rust, no_run
let pool = KeyValueGrpcClientPool::new(settings);
let client = pool.get_grpc_client("shard-1").await; // Arc<KeyValueGrpcClient>, created on demand
pool.gc(&["shard-1", "shard-2"]).await;             // every other id is dropped
```

# GRPC Server

`configure_grpc_server` — register one or more gRPC server implementations on the SDK-managed gRPC server.
```rust, no_run
let mut service_context = ServiceContext::new(settings_reader).await;
service_context.configure_grpc_server(|builder| {
    builder.add_grpc_service(MyCoolGrpcService::new());
});
```

# NoSql
`get_ns_reader` is synchronous — it returns the reader handle immediately; the underlying TCP connection is started later by `start_application`.
```rust, no_run
let service_context = ServiceContext::new(settings_reader).await;
let ns_reader: Arc<MyNoSqlDataReaderTcp<MyModel>> = service_context.get_ns_reader();
```

# Background timers

Register background timers on the `ServiceContext` before `start_application`; the SDK starts them for you (they wait for the app to become initialized and stop on shutdown). Tick logic implements `rust_extensions::MyTimerTick`, whose `tick` returns a `RepeatTimerIteration` telling the timer loop what to do next:

```rust, no_run
use service_sdk::rust_extensions::{MyTimerTick, RepeatTimerIteration};

pub struct MyTick;

#[async_trait::async_trait]
impl MyTimerTick for MyTick {
    async fn tick(&self) -> RepeatTimerIteration {
        // ... do the work ...

        // done for now - wait for the next scheduled tick
        RepeatTimerIteration::WithInterval
    }
}
```

Return `RepeatTimerIteration::Immediately` instead to leave the current iteration and be started again straight away with a fresh `iteration_timeout` window — that is how a long job is done in portions without tripping the timer's per-iteration timeout.

`register_timer` runs a `MyTimer`, which sleeps `duration` between ticks:

```rust, no_run
service_context.register_timer(Duration::from_secs(30), |timer| {
    timer.register_timer("MyTick", Arc::new(MyTick));
});
```

`register_exact_timer` runs a `MyExactTimer`, which fires on aligned wall-clock marks of an `ExactTimerInterval` (`Every5Seconds` → `:00, :05, :10 …`) with no drift, instead of sleeping between ticks. The tick type is the same `MyTimerTick`, so an existing tick can be registered on either timer:

```rust, no_run
use service_sdk::rust_extensions::ExactTimerInterval;

service_context.register_exact_timer(ExactTimerInterval::Every5Seconds, |timer| {
    timer.register_timer("MyTick", Arc::new(MyTick));
});
```

# HTTP server protocol

HTTP/1 vs HTTP/2 is auto-detected per connection by `my_http_server` — no explicit configuration is required for either the TCP listener or the unix-socket listener.

# Unix socket

On unix platforms a unix-socket listener can be enabled via the `UNIX_SOCKET` env var. Socket paths are fixed: `~/http/<service-name>` for HTTP and `~/grpc/<service-name>` for gRPC (when the `grpc` feature is enabled).

| `UNIX_SOCKET` value         | TCP listener | Unix-socket listener |
| --------------------------- | ------------ | -------------------- |
| (unset)                     | on           | off                  |
| `ONLY` (case-insensitive)   | off          | on                   |
| any other value (e.g. `1`)  | on           | on (additional)      |

`ONLY` disables the TCP listener and serves exclusively over the unix socket. This applies to both HTTP and gRPC servers.

