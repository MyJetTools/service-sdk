mod builders;
mod common;
mod service_context;

pub use builders::*;
pub use common::*;
pub use service_context::*;

pub extern crate my_http_server;
pub extern crate my_telemetry;

pub extern crate async_trait;
pub extern crate flurl;
pub extern crate my_logger;
pub extern crate my_settings_reader;
pub extern crate rust_extensions;
pub extern crate serde_yaml;

pub extern crate service_sdk_macros as macros;

#[cfg(any(
    feature = "no-sql-reader",
    feature = "no-sql-writer",
    feature = "macros"
))]
pub extern crate my_no_sql_sdk;

#[cfg(feature = "grpc")]
pub extern crate my_grpc_extensions;

#[cfg(feature = "grpc")]
pub extern crate futures_core;

#[cfg(feature = "postgres")]
pub extern crate my_postgres;

#[cfg(feature = "my-service-bus")]
pub extern crate my_service_bus;
