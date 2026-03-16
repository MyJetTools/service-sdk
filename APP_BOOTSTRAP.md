## MyNoSql Reader (add only when the service needs to read from MyNoSql)

### service-sdk feature

```toml
service-sdk = { ..., features = ["my-nosql-data-reader-sdk"] }
```

### Settings — add field

```rust
pub struct SettingsModel {
    pub my_no_sql_tcp_reader: String,   // ← REQUIRED for NoSql reader
    // ... other fields
}
```

`SdkSettingsTraits` derive auto-generates the trait impl for `ServiceContext`.

### AppContext — add reader field

```rust
use my_no_sql_entities::InstrumentEntity;

pub struct AppContext {
    pub instruments_reader:
        Arc<service_sdk::my_no_sql_sdk::reader::MyNoSqlDataReaderTcp<InstrumentEntity>>,
    // ... other fields
}

impl AppContext {
    pub async fn new(
        settings_reader: Arc<SettingsReader>,
        service_context: &ServiceContext,   // ← NOT underscore — needed for get_ns_reader
    ) -> Self {
        Self {
            // Generic type inferred from field type
            instruments_reader: service_context.get_ns_reader().await,
            // ...
        }
    }
}
```

### Reading data

```rust
// Get all in partition → Option<Vec<(String, Arc<T>)>>
// Tuple: (row_key, entity)
let items = app.instruments_reader
    .get_by_partition_key(InstrumentEntity::PARTITION_KEY)
    .await;

if let Some(entities) = items {
    for (row_key, entity) in entities {
        // entity: Arc<InstrumentEntity>
    }
}

// Get single entity → Option<Arc<T>>
let entity = app.instruments_reader
    .get_entity("partition_key", "row_key")
    .await;
```

**CRITICAL:** `get_by_partition_key` returns `Option<Vec<(String, Arc<T>)>>` — tuple with row_key, NOT `Vec<Arc<T>>`.

---

## MyNoSql Writer (add only when the service needs to write to MyNoSql)

### service-sdk feature

```toml
service-sdk = { ..., features = ["my-nosql-data-writer-sdk"] }
```

### Settings — implement MyNoSqlWriterSettings

```rust
use service_sdk::my_no_sql_sdk::data_writer::MyNoSqlWriterSettings;

pub struct Settings {
    pub my_no_sql_writer_url: String,
    // ... other fields
}

#[async_trait::async_trait]
impl MyNoSqlWriterSettings for AppSettingsReader {
    async fn get_url(&self) -> String {
        self.settings_reader
            .get(|s| s.my_no_sql_writer_url.clone())
            .await
    }

    fn get_app_name(&self) -> &'static str {
        env!("CARGO_PKG_NAME")
    }

    fn get_app_version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }
}
```

### AppContext — add writer field

```rust
use my_no_sql_entities::InstrumentEntity;
use service_sdk::my_no_sql_sdk::{
    abstractions::DataSynchronizationPeriod,
    data_writer::{CreateTableParams, MyNoSqlDataWriter, MyNoSqlDataWriterWithRetries},
};

pub struct AppContext {
    instruments: MyNoSqlDataWriter<InstrumentEntity>,
    // ... other fields
}

impl AppContext {
    pub async fn new(settings_reader: Arc<AppSettingsReader>) -> Self {
        Self {
            instruments: MyNoSqlDataWriter::new(
                settings_reader.clone(),
                Some(CreateTableParams {
                    persist: true,
                    max_partitions_amount: None,
                    max_rows_per_partition_amount: None,
                }),
                DataSynchronizationPeriod::Immediately,
            ),
            // ...
        }
    }

    // ALWAYS expose via with_retries
    pub fn get_instruments(&self) -> MyNoSqlDataWriterWithRetries<InstrumentEntity> {
        self.instruments.with_retries(3)
    }
}
```

### Writing / reading data

```rust
let w = app_ctx.get_instruments();

// Insert or replace
w.insert_or_replace_entity(&entity).await.unwrap();

// Bulk insert or replace
w.bulk_insert_or_replace(&entities).await.unwrap();

// Get entity → Result<Option<T>>
let entity = w.get_entity("pk", "rk", None).await.unwrap();

// Get all in partition → Result<Option<Vec<T>>>
let items = w.get_by_partition_key("pk", None).await.unwrap().unwrap_or_default();

// Delete
w.delete_row("pk", "rk").await.unwrap();
```

**NEVER** call writer methods directly — always through `.with_retries(N)`.

---

## MyNoSql Entity Macro

service-sdk provides `use_my_no_sql_entity!()` to import entity macros.
Available with any of: `my-nosql-sdk`, `my-nosql-data-reader-sdk`, `my-nosql-data-writer-sdk`.

```rust
service_sdk::macros::use_my_no_sql_entity!();

#[my_no_sql_entity("instruments")]
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct InstrumentEntity {
    pub name: String,
}
```

**CRITICAL:** `use_my_no_sql_entity!()` does NOT import serde.
Always use `serde::Serialize`, `serde::Deserialize` (fully qualified).

---

## service-sdk Module Paths for MyNoSql

```
service_sdk::my_no_sql_sdk::reader        → MyNoSqlDataReaderTcp<T>
service_sdk::my_no_sql_sdk::data_writer   → MyNoSqlDataWriter<T>, MyNoSqlDataWriterWithRetries<T>,
                                             MyNoSqlWriterSettings, CreateTableParams
service_sdk::my_no_sql_sdk::abstractions  → DataSynchronizationPeriod
service_sdk::my_no_sql_sdk::core          → MyNoSqlEntity trait
```

---

## service-sdk MyNoSql Feature Names

| Feature | What it enables |
|---|---|
| `my-nosql-sdk` | Entity macros only. No reader, no writer. |
| `my-nosql-data-reader-sdk` | Entity macros + TCP reader |
| `my-nosql-data-writer-sdk` | Entity macros + HTTP writer |
