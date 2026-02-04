extern crate proc_macro;
use proc_macro::TokenStream;

#[proc_macro]
pub fn generate_settings_signature(_item: TokenStream) -> TokenStream {
    let mut traits = vec![];

    traits.push(quote::quote!(+ SeqSettings));

    #[cfg(feature = "my-service-bus")]
    traits.push(quote::quote!(+ MyServiceBusSettings));

    #[cfg(feature = "no-sql-reader")]
    traits.push(quote::quote!(+ MyNoSqlTcpConnectionSettings));

    #[cfg(feature = "no-sql-writer")]
    traits.push(quote::quote!(+ MyNoSqlWriterSettings));

    let result = quote::quote! {
       Arc<impl MyTelemetrySettings + ServiceInfo #(#traits)* + Send + Sync + 'static>
    };

    result.into()
}

#[proc_macro_derive(SdkSettingsTraits)]
pub fn generate_sdk_settings_traits(_input: TokenStream) -> TokenStream {
    quote::quote! {
    #[async_trait]
    impl service_sdk::ServiceInfo for SettingsReader {
        fn get_service_name(&self) -> rust_extensions::StrOrString<'static> {
            if let Ok(suffix) = std::env::var("SERVICE_NAME_SUFFIX") {
                return format!("{}-{}", env!("CARGO_PKG_NAME"), suffix).into();
            }else{
                env!("CARGO_PKG_NAME").into()
            }

        }
        fn get_service_version(&self) -> rust_extensions::StrOrString<'static> {
            env!("CARGO_PKG_VERSION").into()
        }
    }

        }
    .into()
}

#[proc_macro_derive(AutoGenerateSettingsTraits)]
pub fn auto_generate_settings_traits(_input: TokenStream) -> TokenStream {
    let mut auto_generates = Vec::new();

    auto_generates.push(quote::quote! {
        #[async_trait]
        impl SeqSettings for SettingsReader {
           async fn get_conn_string(&self) -> String {
            let read_access = self.settings.read().await;
            read_access.seq_conn_string.clone()
        }
    }
    });

    #[cfg(feature = "postgres")]
    auto_generates.push(quote::quote! {
        #[async_trait]
        impl PostgresSettings for SettingsReader {
            async fn get_connection_string(&self) -> String {
                let read_access = self.settings.read().await;
                read_access.postgres_conn_string.clone()
            }
        }
    });

    #[cfg(feature = "no-sql-writer")]
    auto_generates.push(quote::quote! {
    #[async_trait]
    impl MyNoSqlWriterSettings for SettingsReader {
        async fn get_url(&self) -> String {
            let read_access = self.settings.read().await;
            read_access.my_no_sql_writer.clone()
        }

         fn get_app_name(&self) -> &'static str {
            env!("CARGO_PKG_NAME")
        }

         fn get_app_version(&self) -> &'static str {
            env!("CARGO_PKG_VERSION")
        }
      }
    });

    #[cfg(feature = "no-sql-reader")]
    auto_generates.push(quote::quote!(
        #[async_trait]
        impl service_sdk::my_no_sql_sdk::reader::MyNoSqlTcpConnectionSettings for SettingsReader {
            async fn get_host_port(&self) -> String {
                let read_access = self.settings.read().await;
                read_access.my_no_sql_tcp_reader.clone()
            }
        }
    ));

    #[cfg(feature = "my-service-bus")]
    auto_generates.push(quote::quote!(
        #[async_trait::async_trait]
        impl MyServiceBusSettings for SettingsReader {
            async fn get_host_port(&self) -> String {
                let read_access = self.settings.read().await;
                return read_access.my_sb_tcp_host_port.clone();
            }
        }
    ));

    quote::quote! {
    #[async_trait]
    impl MyTelemetrySettings for SettingsReader {
        async fn get_telemetry_url(&self) -> Option<String> {
            let read_access = self.settings.read().await;
            read_access.my_telemetry.clone()
        }
    }


    #(#auto_generates)*
    }
    .into()
}

#[proc_macro]
pub fn use_grpc_client(_input: TokenStream) -> TokenStream {
    quote::quote! {
        use service_sdk::my_grpc_extensions;
        use service_sdk::my_grpc_extensions::client::generate_grpc_client;
        use service_sdk::my_telemetry;
        use service_sdk::async_trait;
        use service_sdk::my_logger;
    }
    .into()
}

#[proc_macro]
pub fn use_grpc_server(_input: TokenStream) -> TokenStream {
    quote::quote! {
        use service_sdk::my_grpc_extensions;
        use service_sdk::my_telemetry;
        use service_sdk::futures_core;
        use service_sdk::async_trait::async_trait;
        use service_sdk::my_grpc_extensions::server::*;
        use service_sdk::rust_extensions;
        use service_sdk::my_logger;
        use service_sdk::my_grpc_extensions::tonic;
        use service_sdk::my_grpc_extensions::StreamedResponseWriter;
        use std::time::Duration;
    }
    .into()
}

#[proc_macro]
pub fn use_settings(_input: TokenStream) -> TokenStream {
    let mut uses = vec![];

    uses.push(quote::quote!(
        use service_sdk::flurl;
    ));

    #[cfg(feature = "my-service-bus")]
    uses.push(quote::quote!(
        use service_sdk::my_service_bus::client::MyServiceBusSettings;
    ));

    #[cfg(feature = "postgres")]
    uses.push(quote::quote!(
        use service_sdk::my_postgres::PostgresSettings;
    ));

    #[cfg(feature = "no-sql-writer")]
    uses.push(quote::quote!(
        use service_sdk::my_no_sql_sdk::data_writer::MyNoSqlWriterSettings;
    ));

    #[cfg(feature = "no-sql-reader")]
    uses.push(quote::quote!(
        use service_sdk::my_no_sql_sdk::reader::MyNoSqlTcpConnectionSettings;
    ));

    #[cfg(feature = "grpc")]
    uses.push(quote::quote!(
        use service_sdk::my_grpc_extensions::GrpcClientSettings;
    ));

    quote::quote! {
        use service_sdk::async_trait::async_trait;
        use service_sdk::serde_yaml;
        use service_sdk::my_settings_reader;
        use service_sdk::macros::SdkSettingsTraits;
        use service_sdk::rust_extensions;
        use service_sdk::my_logger::my_seq_logger::SeqSettings;
        use service_sdk::my_telemetry::my_telemetry_writer::MyTelemetrySettings;
        use service_sdk::macros::AutoGenerateSettingsTraits;
        #(#uses)*
    }
    .into()
}

#[proc_macro]
pub fn use_my_http_server(_input: TokenStream) -> TokenStream {
    quote::quote! {
        use service_sdk::async_trait;
        use service_sdk::my_http_server;
        use my_http_server::macros::*;
        use my_http_server::*;
        use service_sdk::my_http_server::types::*;
    }
    .into()
}

#[proc_macro]
pub fn use_my_postgres(_input: TokenStream) -> TokenStream {
    quote::quote! {
        use service_sdk::my_postgres;
        use service_sdk::my_postgres::macros::*;
        use service_sdk::my_postgres::*;
        use service_sdk::my_postgres::sql_select::BulkSelectBuilder;
        use service_sdk::my_postgres::*;
        use service_sdk::my_postgres::*;
        use service_sdk::my_telemetry::MyTelemetryContext;
        use service_sdk::my_logger;
        use service_sdk::rust_extensions;
        use service_sdk::rust_extensions::date_time::DateTimeAsMicroseconds;

    }
    .into()
}

#[proc_macro]
pub fn use_my_no_sql_entity(_input: TokenStream) -> TokenStream {
    quote::quote! {
        use service_sdk::my_no_sql_sdk;
        use service_sdk::my_no_sql_sdk::macros::*;
        use service_sdk::rust_extensions;
    }
    .into()
}

#[proc_macro]
pub fn use_my_sb_entity_protobuf_model(_input: TokenStream) -> TokenStream {
    quote::quote! {
        use service_sdk::my_service_bus;
        use service_sdk::my_service_bus::macros::my_sb_entity_protobuf_model;
        use service_sdk::my_service_bus::macros::my_sb_entity_protobuf_model_with_version;
        use service_sdk::rust_extensions;
    }
    .into()
}

#[proc_macro]
pub fn use_my_sb_subscriber(_input: TokenStream) -> TokenStream {
    quote::quote! {
        use service_sdk::my_service_bus;
        use service_sdk::rust_extensions;
        use service_sdk::my_logger;
        use service_sdk::my_logger::LogEventCtx;
        use service_sdk::my_telemetry::MyTelemetryContext;
        use service_sdk::my_service_bus::abstractions::subscriber::*;
        use service_sdk::async_trait::async_trait;

    }
    .into()
}
#[proc_macro]
pub fn use_signal_r_json_contract(_input: TokenStream) -> TokenStream {
    quote::quote! {
        use service_sdk::my_http_server::signal_r::macros::signal_r_json_contract;
        use service_sdk::my_http_server;
        use service_sdk::rust_extensions;
        use service_sdk::my_logger;
        use service_sdk::my_logger::LogEventCtx;
        use service_sdk::my_telemetry::MyTelemetryContext;

    }
    .into()
}

#[proc_macro]
pub fn use_signal_r_subscriber(_input: TokenStream) -> TokenStream {
    quote::quote! {
        use service_sdk::async_trait::async_trait;
        use service_sdk::my_http_server::signal_r::{MySignalRConnection, SignalRTelemetry, MySignalRActionSubscriber};
        use service_sdk::my_http_server;
        use service_sdk::rust_extensions;
        use service_sdk::my_logger;
        use service_sdk::my_logger::LogEventCtx;
        use service_sdk::my_telemetry::MyTelemetryContext;

    }
    .into()
}

use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    Ident, Path, Result, Token, Type,
};

struct GenerateGrpcServiceArgs {
    service_ident: Ident,
    app_ty: Type,
    server_path: Path,
}

impl Parse for GenerateGrpcServiceArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let service_ident: Ident = input.parse()?;
        input.parse::<Token![,]>()?;

        let app_ty: Type = input.parse()?;
        input.parse::<Token![,]>()?;

        let server_path: Path = input.parse()?;

        Ok(Self { service_ident, app_ty, server_path })
    }
}

#[proc_macro]
pub fn generate_grpc_service(input: TokenStream) -> TokenStream {
    let input: proc_macro2::TokenStream = input.into();

    quote::quote! {

        #[derive(Clone)]
        pub struct SdkGrpcService {
            pub app: std::sync::Arc<#input>,
        }

        impl SdkGrpcService {
            pub fn new(app: std::sync::Arc<#input>) -> Self {
                Self { app }
            }
        }

    }
    .into()
}

#[proc_macro]
pub fn generate_named_grpc_service(input: TokenStream) -> TokenStream {
    let GenerateGrpcServiceArgs { service_ident, app_ty, server_path } = parse_macro_input!(input as GenerateGrpcServiceArgs);

    let expanded = quote! {
        #[derive(Clone)]
        pub struct #service_ident {
            pub app_context: ::std::sync::Arc<#app_ty>,
        }

        impl #service_ident {
            pub fn new(app_context: ::std::sync::Arc<#app_ty>) -> Self {
                Self { app_context }
            }
        }

        impl service_sdk::IntoGrpcServer for #service_ident {
            type GrpcServer = #server_path<Self>;

            fn into_grpc_server(self) -> Self::GrpcServer {
                #server_path::new(self)
            }
        }
    };

    expanded.into()
}
