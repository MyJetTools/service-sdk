# Bootstrap Guide

This document contains step-by-step instructions for bootstrapping a new project based on the patterns and configurations used in this project.

## Project Structure

## Dependencies

### Core Dependencies

**What was added:**
- `service-sdk`: Core SDK library from MyJetTools GitHub repository
- `tokio`: Async runtime for Rust with full feature set
- `serde`: Serialization/deserialization framework for Rust

**How to add it:**
1. Open `Cargo.toml` in your project root
2. Add the following dependencies under the `[dependencies]` section:

```toml
service-sdk = { git = "https://github.com/MyJetTools/service-sdk.git", tag = "{last_tag}", features = [

] }

tokio = { version = "*", features = ["full"] }
serde = "*"
```

**Files modified/created:**
- `Cargo.toml` - Added dependencies section

**Code snippets:**
```toml
[dependencies]
service-sdk = { git = "https://github.com/MyJetTools/service-sdk.git", tag = "{last_tag}", features = [

] }

tokio = { version = "*", features = ["full"] }
serde = "*"

[build-dependencies]
ci-utils = { git = "https://github.com/MyJetTools/ci-utils.git", tag = "{last_tag}" }
```

## Configuration Files

### Build Script and CI Generation

**What was added:**
- `build.rs`: Build script that generates GitHub CI configuration files using ci-utils
- `[build-dependencies]` section in `Cargo.toml` with `ci-utils` dependency

**How to add it:**
1. Create a new file `build.rs` in the project root (same level as `Cargo.toml`)
2. Add the build script code that uses `ci_utils::ci_generator::CiGenerator`
3. Add `[build-dependencies]` section to `Cargo.toml` with the `ci-utils` dependency

**Files modified/created:**
- `build.rs` - Created build script for CI generation
- `Cargo.toml` - Added `[build-dependencies]` section

**Code snippets:**

`build.rs`:
```rust
fn main() {
    ci_utils::ci_generator::CiGenerator::new(env!("CARGO_PKG_NAME"))
        .as_basic_service()
        .generate_github_ci_file()
        .build();
}
```

`Cargo.toml`:
```toml
[build-dependencies]
ci-utils = { git = "https://github.com/MyJetTools/ci-utils.git", tag = "{last_tag}" }
```

**Notes:**
- The build script runs automatically during the build process
- It generates GitHub Actions CI configuration files
- Uses `env!("CARGO_PKG_NAME")` to get the package name from Cargo.toml
- The `.as_basic_service()` method configures it as a basic service type
- The `.generate_github_ci_file()` method creates the GitHub CI workflow file

## Code Patterns

### Settings Module

**What was added:**
- `src/settings.rs`: Settings configuration module using service-sdk macros
- SettingsModel struct with derive macros for automatic settings handling
- Module declaration in `main.rs`
- Settings reader initialization in `main.rs` using `SettingsReader::new()`
- Arc wrapper for settings reader for shared ownership

**How to add it:**
1. Create a new file `src/settings.rs` in your project
2. Add the settings module code with your configuration fields
3. Declare the module in `src/main.rs` by adding `mod settings;` at the top
4. **IMPORTANT**: Before adding the settings reader code, **ASK THE USER** what file path or directory they want to use for reading settings. Do NOT hardcode any path.
5. Add the settings reader initialization code in `main.rs` using the path provided by the user
6. Wrap the settings reader in `Arc` for shared ownership

**Files modified/created:**
- `src/settings.rs` - Created new settings module
- `src/main.rs` - Added module declarations, settings reader initialization, ServiceContext creation, and application startup

**Code snippets:**

`src/settings.rs`:
```rust
use serde::{Deserialize, Serialize};

service_sdk::macros::use_settings!();

#[derive(
    my_settings_reader::SettingsModel,
    AutoGenerateSettingsTraits,
    SdkSettingsTraits,
    Serialize,
    Deserialize,
    Debug,
    Clone,
)]
pub struct SettingsModel {
    pub my_telemetry: Option<String>,
    pub seq_conn_string: String,
}
```

`src/main.rs`:
```rust
use std::sync::Arc;

use crate::app::AppContext;

mod app;
mod settings;

#[tokio::main]
async fn main() {
    // IMPORTANT: Ask the user for the settings file/directory path before implementing
    // Replace "SETTINGS_PATH_FROM_USER" with the actual path provided by the user
    let settings_reader = settings::SettingsReader::new("SETTINGS_PATH_FROM_USER").await;
    let settings_reader = Arc::new(settings_reader);

    let mut service_context = service_sdk::ServiceContext::new(settings_reader.clone()).await;
    let app = Arc::new(AppContext::new(settings_reader, &service_context).await);

    service_context.start_application().await;
}
```

**Notes:**
- The `SettingsModel` struct uses multiple derive macros from `service-sdk` and `my_settings_reader`
- Fields can be `Option<T>` for optional settings or required fields
- The `use_settings!()` macro must be called before defining the struct
- **CRITICAL**: Always ask the user for the settings file/directory path. Never hardcode paths like `.settings-file-name` or any other specific path in the bootstrap code
- The settings reader is wrapped in `Arc` to allow shared ownership across async tasks
- The `SettingsReader::new()` method is async and must be awaited

### App Context Module

**What was added:**
- `src/app/mod.rs`: Module declaration file for the app module
- `src/app/app_ctx.rs`: AppContext struct that holds application services and components used for business logic
- ServiceContext initialization from service-sdk (holds communication-level components like gRPC servers, HTTP servers)
- Application startup pattern using `service_context.start_application().await`

**How to add it:**
1. Create a new directory `src/app/` in your project
2. Create `src/app/mod.rs` with module declaration and re-exports
3. Create `src/app/app_ctx.rs` with the AppContext struct
4. Add `mod app;` declaration in `src/main.rs`
5. Import `AppContext` in `main.rs` using `use crate::app::AppContext;`
6. After creating the settings reader, create a `ServiceContext` using `service_sdk::ServiceContext::new()`
7. Create the `AppContext` instance passing the settings reader and service context
8. Start the application with `service_context.start_application().await`

**Files modified/created:**
- `src/app/mod.rs` - Created module declaration file
- `src/app/app_ctx.rs` - Created AppContext struct
- `src/main.rs` - Added app module declaration, ServiceContext creation, and application startup

**Code snippets:**

`src/app/mod.rs`:
```rust
mod app_ctx;
pub use app_ctx::*;
```

`src/app/app_ctx.rs`:
```rust
use std::sync::Arc;

use service_sdk::*;

pub struct AppContext {
    pub settings_reader: Arc<crate::settings::SettingsReader>,
}

impl AppContext {
    pub async fn new(
        settings_reader: Arc<crate::settings::SettingsReader>,
        _service_context: &ServiceContext,
    ) -> Self {
        Self { settings_reader }
    }
}
```

**Notes:**
- **Architecture distinction:**
  - `ServiceContext` - is the context where **Communication level components** are (e.g., gRPC servers, HTTP servers, message queues, network protocols)
  - `AppContext` - is a structure where **Application services** are held. It holds components which are used for business logic, but does NOT hold business logic or domain models directly - only application services
- The `AppContext` struct holds application-wide state and dependencies
- It receives the settings reader and service context during initialization
- The service context is mutable and used to start the application lifecycle
- The app context is wrapped in `Arc` for shared ownership across async tasks
- The `start_application().await` call blocks until the application shuts down

### gRPC Server Bootstrap (CONDITIONAL - Only if user requests gRPC server features)

**⚠️ IMPORTANT**: This section should ONLY be used when the user explicitly requests gRPC server functionality (e.g., "Please add gRPC server features", "I need gRPC server", etc.). If the user does not ask for gRPC server features, DO NOT include this in the bootstrap.

**What was added:**
- `src/grpc_server/mod.rs`: gRPC server module with service generation macro
- gRPC-related dependencies in `Cargo.toml` (tonic, tonic-prost, prost, async-trait)
- `service-sdk` features updated to include "macros" and "grpc"
- Module declaration in `main.rs`

**Note:** `GrpcClientSettings` trait implementation is NOT needed for gRPC server-only services. It's only required if the service acts as a gRPC client.

**How to add it:**
1. **ONLY proceed if user explicitly requests gRPC server features**
2. Create a new directory `src/grpc_server/` in your project
3. Create `src/grpc_server/mod.rs` with the gRPC service generation macro
4. Add `mod grpc_server;` declaration in `src/main.rs`
5. Update `Cargo.toml` to add gRPC dependencies and enable gRPC features in `service-sdk`

**Files modified/created:**
- `src/grpc_server/mod.rs` - Created gRPC server module
- `Cargo.toml` - Added gRPC dependencies and features
- `src/main.rs` - Added grpc_server module declaration

**Code snippets:**

`src/grpc_server/mod.rs`:
```rust
service_sdk::macros::generate_grpc_service!(crate::app::AppContext);
```

`Cargo.toml` - Updated dependencies:
```toml
[dependencies]
service-sdk = { git = "https://github.com/MyJetTools/service-sdk.git", tag = "${last_tag}", features = [
    "macros",
    "grpc",
] }

tokio = { version = "*", features = ["full"] }
serde = "*"

tonic = "*"
tonic-prost = "*"
prost = "*"
async-trait = "*"
```

`src/main.rs` - Add module declaration:
```rust
mod app;
mod grpc_server;
mod settings;
```

**Notes:**
- The `generate_grpc_service!` macro generates the gRPC server implementation based on the AppContext
- This is a bootstrap setup - actual gRPC service implementations need to be added separately
- **Note:** `GrpcClientSettings` trait implementation is only needed if the service acts as a gRPC client (calls other gRPC services). For server-only services, it's not required.

#### Implementing gRPC Services from Proto Files

**⚠️ IMPORTANT**: This subsection applies when the user requests specific gRPC service implementations based on proto files. **ASK THE USER** for:
- The proto file repository URL/base path
- The proto file name(s) to download and implement
- The service name(s) from the proto file(s)

**What was added:**
- Proto file download and build configuration in `build.rs` using `ProtoFileBuilder`
- Generated gRPC server implementation file in `src/grpc_server/` (e.g., `{service_name}_grpc_server.rs`)
- Proto module inclusion in `main.rs` using `tonic::include_proto!()`
- gRPC server configuration in `main.rs` using `service_context.configure_grpc_server()`
- Module declaration in `src/grpc_server/mod.rs` for the generated server file

**How to add it:**
1. **ASK THE USER** for the proto file repository URL and proto file name(s)
2. Update `build.rs` to download proto files using `ProtoFileBuilder`
3. The build process will automatically download proto files to `/proto` directory and generate Rust code
4. Create the gRPC server implementation file in `src/grpc_server/` based on the proto service
5. Add module declaration in `src/grpc_server/mod.rs` for the new server file
6. Add proto module in `main.rs` using `tonic::include_proto!()`
7. Import the server trait in `main.rs`
8. Configure the gRPC server in `main.rs` using `service_context.configure_grpc_server()`

**Files modified/created:**
- `build.rs` - Added proto file download and build configuration
- `proto/{ProtoFileName}.proto` - Downloaded proto file (created automatically)
- `src/grpc_server/{service_name}_grpc_server.rs` - Generated gRPC server implementation
- `src/grpc_server/mod.rs` - Added module declaration for server implementation
- `src/main.rs` - Added proto module, server trait import, and gRPC server configuration

**Code snippets:**

`build.rs` - Add proto file download before CI generation:
```rust
fn main() {
    ci_utils::ProtoFileBuilder::new(
        "PROTO_REPOSITORY_URL_FROM_USER", // Ask user for this URL
    )
    .sync_and_build("ProtoFileName.proto"); // Ask user for proto file name

    ci_utils::ci_generator::CiGenerator::new(env!("CARGO_PKG_NAME"))
        .as_basic_service()
        .generate_github_ci_file()
        .build();
}
```

`src/grpc_server/{service_name}_grpc_server.rs` - Generated server implementation:
```rust
use crate::{grpc_server::SdkGrpcService, {module_name}_grpc::*};

use crate::{module_name}_grpc::{service_name}_server::{ServiceName};

service_sdk::macros::use_grpc_server!();

#[tonic::async_trait]
impl {ServiceName} for SdkGrpcService {
    #[with_telemetry]
    async fn {method_name}(
        &self,
        request: tonic::Request<{RequestType}>,
    ) -> Result<tonic::Response<{ResponseType}>, tonic::Status> {
        let request = request.into_inner();
        todo!("Implement {method_name}");

        Ok(tonic::Response::new({response_value}))
    }

    // System method - same for all services, used by framework to verify gRPC channel is alive
    async fn ping(&self, _: tonic::Request<()>) -> Result<tonic::Response<()>, tonic::Status> {
        Ok(tonic::Response::new(()))
    }

    // Additional methods from proto service...
}
```

`src/grpc_server/mod.rs` - Add module declaration:
```rust
mod {service_name}_grpc_server;
pub use {service_name}_grpc_server::*;
service_sdk::macros::generate_grpc_service!(crate::app::AppContext);
```

`src/main.rs` - Add proto module and configure gRPC server:
```rust
use std::sync::Arc;

use crate::{app::AppContext, grpc_server::SdkGrpcService};

mod app;
mod grpc_server;
mod settings;

use {module_name}_grpc::{service_name}_server::*;

pub mod {module_name}_grpc {
    tonic::include_proto!("{proto_package_name}"); // Package name from proto file
}

#[tokio::main]
async fn main() {
    // ... settings reader and service context setup ...

    service_context.configure_grpc_server(|builder| {
        let service = SdkGrpcService::new(app.clone());
        builder.add_grpc_service({ServiceName}Server::new(service.clone()));
    });

    service_context.start_application().await;
}
```

**Example structure:**

The pattern follows this structure:
- Proto file name: `{ProtoFileName}.proto` (e.g., `MyService.proto`)
- Proto package name: `{proto_package_name}` (from `package` declaration in proto file)
- Module name: `{module_name}_grpc` (derived from proto package name, typically snake_case)
- Service name: `{ServiceName}` (from `service` declaration in proto file, PascalCase)
- Server trait: `{ServiceName}Server` (generated by tonic)
- Server implementation file: `{service_name}_grpc_server.rs` (snake_case version of service name)

**Notes:**
- The `ProtoFileBuilder` downloads proto files to `/proto` directory during build
- Proto files are automatically compiled to Rust code by `tonic-build` during build
- The server implementation file follows the pattern: `{service_name}_grpc_server.rs`
- Service methods should be marked with `#[with_telemetry]` attribute for telemetry support
- Replace `todo!()` placeholders with actual implementation logic
- The proto package name (from `package` declaration in proto file) is used in `include_proto!()`
- Multiple proto files can be downloaded by calling `.sync_and_build()` multiple times or with multiple file names
- **The `ping` method is a system method that is the same for all gRPC services** - it's used by the framework to verify that the gRPC channel is alive. Always implement it as: `async fn ping(&self, _: tonic::Request<()>) -> Result<tonic::Response<()>, tonic::Status> { Ok(tonic::Response::new(())) }`

### gRPC Client Bootstrap (CONDITIONAL - Only if user requests gRPC client features)

**⚠️ IMPORTANT**: This section should ONLY be used when the user explicitly requests gRPC client functionality (e.g., "Please add gRPC client features", "I need to call other gRPC services", etc.). If the user does not ask for gRPC client features, DO NOT include this in the bootstrap.

**What was added:**
- `src/grpc_client/mod.rs`: gRPC client module with client generation
- `src/grpc_client/{service_name}_grpc_client.rs`: gRPC client struct generated from proto file
- `GrpcClientSettings` trait implementation in `settings.rs` for URL resolution
- gRPC client field in `AppContext` for accessing the client
- Module declaration in `main.rs`

**How to add it:**
1. **ONLY proceed if user explicitly requests gRPC client features**
2. **ASK THE USER** for the following:
   - Proto file path (if not already downloaded for server, or if using a different proto file)
   - **Generated client type name**: Ask if they want to use the same name as in the proto file or provide a custom name
   - **Settings URL field name**: How to name the URL field in SettingsModel (e.g., `{service_name}_grpc_url`)
   - **AppContext field name**: How to name the client field in AppContext (e.g., `{service_name}_grpc_client`)
3. Create a new directory `src/grpc_client/` in your project
4. Create `src/grpc_client/{service_name}_grpc_client.rs` with the gRPC client generation macro
5. Create `src/grpc_client/mod.rs` with module declaration and re-exports
6. Add `mod grpc_client;` declaration in `src/main.rs`
7. **IMPORTANT**: If there's no gRPC server implementation, add `tonic::include_proto!()` module in `src/main.rs` to include proto definitions
8. Add `GrpcClientSettings` trait implementation to `src/settings.rs` with URL field mapping
9. Add URL field to `SettingsModel` in `src/settings.rs`
10. Add gRPC client field to `AppContext` and initialize it in the `new()` method

**Files modified/created:**
- `src/grpc_client/mod.rs` - Created gRPC client module
- `src/grpc_client/{service_name}_grpc_client.rs` - Created gRPC client struct
- `src/settings.rs` - Added URL field to SettingsModel and GrpcClientSettings implementation
- `src/app/app_ctx.rs` - Added gRPC client field and initialization
- `src/main.rs` - Added grpc_client module declaration

**Code snippets:**

`src/grpc_client/{service_name}_grpc_client.rs`:
```rust
service_sdk::macros::use_grpc_client!();

#[service_sdk::my_grpc_extensions::client::generate_grpc_client(
    proto_file: "./proto/{ProtoFileName}.proto",
    crate_ns: "crate::{module_name}_grpc",
    retries: 3,
    request_timeout_sec: 10,
    ping_timeout_sec: 1,
    ping_interval_sec: 3,
)]
pub struct {ClientTypeName}; // Ask user: same as proto service name or custom name
```

`src/grpc_client/mod.rs`:
```rust
mod {service_name}_grpc_client;
pub use {service_name}_grpc_client::*;
```

`src/settings.rs` - Add URL field to SettingsModel:
```rust
pub struct SettingsModel {
    pub my_telemetry: Option<String>,
    pub seq_conn_string: String,
    
    pub {settings_url_field_name}: String, // Ask user for field name (e.g., {service_name}_grpc_url)
}
```

`src/settings.rs` - Add GrpcClientSettings implementation:
```rust
#[async_trait::async_trait]
impl GrpcClientSettings for SettingsReader {
    async fn get_grpc_url(&self, name: &'static str) -> service_sdk::my_grpc_extensions::GrpcUrl {
        use crate::grpc_client::*;

        if name == {ClientTypeName}::get_service_name() {
            let read_access = self.settings.read().await;
            return read_access.{settings_url_field_name}.to_string().into();
        }

        panic!("Unknown grpc service name: {}", name)
    }
}
```

`src/app/app_ctx.rs` - Add client field:
```rust
use crate::grpc_client::*;

pub struct AppContext {
    pub settings_reader: Arc<crate::settings::SettingsReader>,
    pub {app_context_field_name}: {ClientTypeName}, // Ask user for field name (e.g., {service_name}_grpc_client)
}

impl AppContext {
    pub async fn new(
        settings_reader: Arc<crate::settings::SettingsReader>,
        _service_context: &ServiceContext,
    ) -> Self {
        Self {
            {app_context_field_name}: {ClientTypeName}::new(settings_reader.clone()),
            settings_reader,
        }
    }
}
```

`src/main.rs` - Add module declaration and proto module (if no server implementation):
```rust
mod app;
mod grpc_client;
mod settings;

// IMPORTANT: If there's no gRPC server implementation, you MUST include the proto module
pub mod {module_name}_grpc {
    tonic::include_proto!("{proto_package_name}"); // Package name from proto file
}
```

**Notes:**
- The `generate_grpc_client` macro generates a gRPC client struct from the proto file
- The client automatically handles connection management, retries, and ping/health checks
- Configuration parameters (retries, timeouts, ping intervals) can be adjusted based on requirements
- The `GrpcClientSettings` trait maps service names to URL fields from settings
- The client is initialized in `AppContext::new()` and receives the settings reader
- **IMPORTANT**: Always ask the user for naming preferences:
  - Client type name (proto service name or custom)
  - Settings URL field name
  - AppContext field name
- Multiple gRPC clients can be added by repeating this pattern for each service
- The same proto file can be used for both server and client implementations
- **CRITICAL**: If implementing a gRPC client WITHOUT a server implementation, you MUST add `tonic::include_proto!()` in `main.rs` to include the proto definitions. If a server implementation exists, the proto module is already included there.

### Service Bus Bootstrap (CONDITIONAL - Only if user requests service bus features)

**⚠️ IMPORTANT**: This section should ONLY be used when the user explicitly requests service bus functionality (e.g., "Please add service bus features", "I need service bus", etc.). If the user does not ask for service bus features, DO NOT include this in the bootstrap.

**What was added:**
- `my_sb_tcp_host_port: String` field added to `SettingsModel` in `src/settings.rs`

**Note:** The `my-sb-contracts` crate addition is NOT documented here as it's project-specific and enables all service bus features automatically.

**How to add it:**
1. **ONLY proceed if user explicitly requests service bus features**
2. Add `my_sb_tcp_host_port: String` field to the `SettingsModel` struct in `src/settings.rs`

**Files modified/created:**
- `src/settings.rs` - Added `my_sb_tcp_host_port` field to SettingsModel

**Code snippets:**

`src/settings.rs` - Add field to SettingsModel:
```rust
pub struct SettingsModel {
    pub my_telemetry: Option<String>,
    pub seq_conn_string: String,
    
    pub my_sb_tcp_host_port: String,
}
```

**Notes:**
- The `my_sb_tcp_host_port` field stores the TCP host and port for the service bus connection
- This field is required when service bus features are enabled
- The actual service bus implementation and contracts are handled by the `my-sb-contracts` crate, which is added separately and not documented in this bootstrap

#### Finding Service Bus Contract Structs

**⚠️ IMPORTANT**: When implementing service bus producers or consumers, you MUST:

1. **Look inside the `my-sb-contracts` crate** to find available contract structs
2. **Search for structs with the `my_sb_entity_protobuf_model` attribute** - these are the structs that can be used for producers/consumers

**How to identify contract structs:**
- Structs marked with `#[my_sb_entity_protobuf_model(topic_id = "...")]` attribute are service bus contracts
- These structs derive `Clone`, `PartialEq`, and `::prost::Message`
- Each struct has a unique `topic_id` specified in the attribute
- Example: `SttTranscribing` with `topic_id = "stt-transcribed-text"` can be used as `MyServiceBusPublisher<SttTranscribing>`

**Example workflow:**
1. Search `my-sb-contracts` crate for `my_sb_entity_protobuf_model`
2. Identify the struct you need (e.g., `SttTranscribing`, `SendSmsMySbContract`, etc.)
3. Import it: `use my_sb_contracts::SttTranscribing;`
4. Use it in `AppContext` for producers: `pub stt_producer: MyServiceBusPublisher<SttTranscribing>`
5. Or configure subscribers in `ServiceContext` for consumers

### Service Bus Subscriber Plugin Pattern (example derived from `candles-flows-grpc`)

**What was added:**
- A concrete pattern for wiring a MyServiceBus subscriber plugin:
  - Implement `SubscriberCallback<TContract>` for a domain handler
  - Register it with `register_sb_subscribe(...)` and choose `TopicQueueType`
  - Depend on a contracts crate (e.g., `my-sb-contracts`) for message models

**How to add it:**
1. Ensure `SettingsModel` includes `my_sb_tcp_host_port: String` and that the service bus feature is enabled in `service-sdk`.
2. Add your contracts crate (e.g., `my-sb-contracts`) to `Cargo.toml`.
3. Create a subscriber type that implements `SubscriberCallback<YourContract>` and performs your domain logic.
4. Register the subscriber during startup with `register_sb_subscribe` using the desired `TopicQueueType` (e.g., `PermanentWithSingleConnection` for durable listeners or `DeleteOnDisconnect` for ephemeral ones).

**Files modified/created:**
- `src/my_sb_subscribers/{your_subscriber}.rs` (or any module you choose)
- `src/{mod}.rs` (to re-export the subscriber)
- `src/main.rs` (register the subscriber)

**Code snippets:**

Subscriber implementation (simplified from `candles-flows-grpc`):
```rust
use std::sync::Arc;
use my_sb_contracts::CandleRawMySbContract; // your contract type
use service_sdk::my_service_bus::abstractions::subscriber::*;

use crate::app::AppContext;

pub struct CandlesSbSubscriber {
    app: Arc<AppContext>,
}

impl CandlesSbSubscriber {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

#[async_trait::async_trait]
impl SubscriberCallback<CandleRawMySbContract> for CandlesSbSubscriber {
    async fn handle_messages(
        &self,
        messages_reader: &MessagesReader<CandleRawMySbContract>,
    ) -> Result<(), MySbSubscriberHandleError> {
        // Ensure app is ready before processing
        self.app.what_when_initialized().await;

        let mut cache = self.app.candles_cache.lock().await;
        while let Some(mut msg) = messages_reader.get_next_message().await {
            let payload = msg.take_message();
            cache.bid_ask_to_update_cache.update(&payload.id, payload.c);
            cache.candles_cache.update_minute_candles(&payload);
        }
        Ok(())
    }
}
```

Subscriber registration in `main.rs`:
```rust
service_context
    .register_sb_subscribe(
        Arc::new(CandlesSbSubscriber::new(app_context.clone())),
        service_sdk::my_service_bus::abstractions::subscriber::TopicQueueType::DeleteOnDisconnect,
    )
    .await;
```

Notes:
- Swap `TopicQueueType::DeleteOnDisconnect` with `PermanentWithSingleConnection` or `Permanent` if you want a durable queue.
- Keep processing idempotent; retries can happen.
- Contracts come from your `my-sb-contracts` (or equivalent) crate; pick the right model for the topic you subscribe to.
- For detailed `TopicQueueType` semantics, see `my-service-bus-sdk.md` resource.

### Service Bus Publisher Pattern (example derived from [`price-feed-polygon`](https://github.com/my-trading-robot/price-feed-polygon))

**What was added:**
- A pattern for publishing SB messages using `MyServiceBusPublisher<TContract>` obtained from `ServiceContext`.

**How to add it:**
1. Ensure `SettingsModel` includes `my_sb_tcp_host_port: String` and the service bus feature is enabled in `service-sdk`.
2. Add your contracts crate (e.g., `my-sb-contracts`) to `Cargo.toml`.
3. Add a publisher field to your `AppContext` and initialize it with `ServiceContext::get_sb_publisher(do_retries: bool)`. The topic is always created if missing; the boolean controls whether publish will retry on disconnection/connection-loss.
4. Use the publisher where you produce events and call `publish(&message, partition_key)`; use `None` if you do not need a custom partition key.

**Files modified/created:**
- `src/app/app_ctx.rs` (or equivalent) — add and initialize the publisher.
- The module where you emit events (e.g., websocket callbacks, timers) — call `publish`.
- `Cargo.toml` — ensure contracts crate is added.

**Code snippets:**

Publisher setup in `AppContext` (simplified from `price-feed-polygon`):
```rust
use my_sb_contracts::CandleRawMySbContract;
use service_sdk::my_service_bus::abstractions::publisher::MyServiceBusPublisher;

pub struct AppContext {
    pub candle_publisher: MyServiceBusPublisher<CandleRawMySbContract>,
    // other fields...
}

impl AppContext {
    pub async fn new(settings_reader: Arc<SettingsReader>, sc: &ServiceContext) -> Self {
        Self {
            candle_publisher: sc.get_sb_publisher(true).await, // true => retry publish on connection loss
            // other fields...
            settings_reader,
        }
    }
}
```

Publishing a message (simplified from `price-feed-polygon` websocket handler):
```rust
let msg = CandleRawMySbContract {
    id: instrument.get_instrument_id().to_string(),
    timestamp: ws_candle.get_timestamp().unix_microseconds as u64,
    o: ws_candle.o,
    c: ws_candle.c,
    h: ws_candle.h,
    l: ws_candle.l,
    v: ws_candle.v,
    src: CandleRawMySbContract::SRC_POLYGON,
};
let _ = self.app.candle_publisher.publish(&msg, None).await;
```

Notes:
- Keep payload construction idempotent; publishers may retry.
- `get_sb_publisher(true)` enables retry-on-disconnect for publishes (it will loop until a connection is restored when errors are `NoConnectionToPublish`/`Disconnected`); the topic is created if missing regardless of the flag.
- Choose `TopicQueueType` on the subscriber side to match how publishers emit messages.
- Use `partition_key` when ordering/affinity is required; otherwise pass `None`.

---

## Instructions

As you add new content to this project, document it here following this format:

### [Feature/Component Name]

**What was added:**
- Description of what was added

**How to add it:**
- Step-by-step instructions for adding this to a new project

**Files modified/created:**
- List of files that were changed or created

**Code snippets:**
- Relevant code examples

---
