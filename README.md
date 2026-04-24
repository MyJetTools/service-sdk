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
    let settings_reader = Arc::new(SettingsReader::new(".service_settings").await);
    let settings = settings_reader.get_settings().await;

    let mut service_context = ServiceContext::new(settings_reader);

    service_context.setup_http(None, None)
    .register_http_routes(|server| {
        server.register_get(GetAction::new());
        server.register_post(PostAction::new());
    });

    service_context.start_application().await;
}
```

Settings Model:

```rust,no_run
#[derive(SettingsModel, Serialize, Deserialize, Debug, Clone)]
pub struct SettingsModel {
    #[serde(rename = "MyTelemetry")]
    pub my_telemetry: String,
    #[serde(rename = "Seq")]
    pub seq: String,
}

#[async_trait::async_trait]
impl my_telemetry_writer::MyTelemetrySettings for SettingsReader {
    async fn get_telemetry_url(&self) -> String {
        let read_access = self.settings.read().await;
        read_access.my_telemetry.clone()
    }
}

#[async_trait::async_trait]
impl my_seq_logger::SeqSettings for SettingsReader {
    async fn get_conn_string(&self) -> String {
        let read_access = self.settings.read().await;
        read_access.seq.clone()
    }
}

#[async_trait::async_trait]
impl ServiceInfo for SettingsReader {
    fn get_service_name(&self) -> String {
        env!("CARGO_PKG_NAME").to_string()
    }
    fn get_service_version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }
}
```

# Features overview
| Feature                     | Description                                                                                                    | Settings implementation                                                                                                                                                                                                                           |
| --------------------------- | -------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [default](#default)         | /api/isalive endpoint,  telemetry, and seq logger enabled by default. Also, you can define custom http routes. | [my_telemetry_writer::MyTelemetrySettings](https://github.com/MyJetTools/my-telemetry-writer), [my_seq_logger::SeqSettings](https://github.com/MyJetTools/my-seq-logger) and [service_sdk::ServiceInfo](#recommended-serviceinfo-implementation). |
| [service-bus](#service-bus) | Allows to make SB subscribe and get SB publishers                                                              | [my_service_bus_tcp_client::MyServiceBusSettings](https://github.com/MyJetTools/my-service-bus-tcp-client)                                                                                                                                        |
| [no-sql](#nosql)            | Allows to get NS subscribers                                                                                   | [my_no_sql_tcp_reader::MyNoSqlTcpConnectionSettings](https://github.com/MyJetTools/my-no-sql-tcp-reader)                                                                                                                                          |
| [grpc-server](#grpc-server) | Allows to bind grpc server implementation                                                                      | -                                                                                                                                                                                                                                                 |

# Recommended ServiceInfo implementation

```rust,no_run
#[async_trait::async_trait]
impl ServiceInfo for SettingsReader {
    fn get_service_name(&self) -> String {
        env!("CARGO_PKG_NAME").to_string()
    }
    fn get_service_version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }
}
```

# Metrics
We supports metrics for grpc and http. They enabled by default. You can get it by /metrics url

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
Also if you need - you can create you own metrics:

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

# Default
setup_http, register_http_routes - there you can pass basic http auth rules and bind controllers.

```rust, no_run
    service_context.setup_http(None, None)
    .register_http_routes(|server| {
        server.register_get(GetAction::new());
        server.register_post(PostAction::new());
    });
```



# Service Bus
register_sb_subscribe

```rust, no_run
let mut service_context = ServiceContext::new(settings_reader);
service_context
    .register_sb_subscribe(
            Arc::new(CallbackAccountsSenderJob::new()),
            TopicQueueType::PermanentWithSingleConnection,
        )
        .await;
```

get_sb_publisher
```rust, no_run
let service_context = ServiceContext::new(settings_reader);
let sb_publisher: MyServiceBusPublisher<Model> = service_context.get_sb_publisher().await;
```

get_sb_publisher_with_internal_queue
```rust, no_run
let service_context = ServiceContext::new(settings_reader);
let sb_publisher: PublisherWithInternalQueue<Model> = service_context.get_sb_publisher_with_internal_queue().await;
```

# GRPC Server

add_grpc_service - bind grpc server implementation.
```rust, no_run
let service_context = ServiceContext::new(settings_reader);
let grpc_server = MyCoolGrpcService::new();
service_context.add_grpc_service(grpc_server).await;
```

# NoSql
get_ns_reader
```rust, no_run
let service_context = ServiceContext::new(settings_reader);
let ns_reader: Arc<MyNoSqlDataReader<MyModel>> = service_context.get_ns_reader().await;
```

