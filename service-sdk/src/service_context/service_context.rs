use my_http_server::MyHttpServer;
use my_logger::my_seq_logger::{SeqLogger, SeqSettings};
use my_telemetry::my_telemetry_writer::{MyTelemetrySettings, MyTelemetryWriter};
use rust_extensions::{AppStates, MyTimer, StrOrString};

#[cfg(feature = "my-nosql-data-writer-sdk")]
use my_no_sql_sdk::data_writer::MyNoSqlWriterSettings;

#[cfg(feature = "my-nosql-data-reader-sdk")]
use my_no_sql_sdk::reader::*;

#[cfg(feature = "my-service-bus")]
use my_service_bus::{
    abstractions::{
        publisher::MyServiceBusPublisher,
        subscriber::{MySbMessageDeserializer, SubscriberCallback, TopicQueueType},
        GetMySbModelTopicId, MySbMessageSerializer,
    },
    client::{MyServiceBusClient, MyServiceBusSettings},
};

use std::{sync::Arc, time::Duration};

use crate::{HttpServerBuilder, ServiceInfo};

#[cfg(feature = "grpc")]
use crate::GrpcServerBuilder;

pub struct ServiceContext {
    pub http_server_builder: HttpServerBuilder,
    pub http_servers: Vec<MyHttpServer>,

    pub telemetry_writer: MyTelemetryWriter,
    pub app_states: Arc<AppStates>,
    pub app_name: StrOrString<'static>,
    pub app_version: StrOrString<'static>,
    pub background_timers: Vec<MyTimer>,
    #[cfg(feature = "my-nosql-data-reader-sdk")]
    pub my_no_sql_connection: Arc<MyNoSqlTcpConnection>,
    #[cfg(feature = "my-service-bus")]
    pub sb_client: Arc<MyServiceBusClient>,
    #[cfg(feature = "grpc")]
    pub grpc_server_builder: Option<GrpcServerBuilder>,
}

impl ServiceContext {
    pub async fn new(settings_reader: service_sdk_macros::generate_settings_signature!()) -> Self {
        metrics_prometheus::install();

        #[cfg(feature = "with-tls")]
        rustls::crypto::ring::default_provider()
            .install_default()
            .expect("Failed to install rustls crypto provider");

        let app_states = Arc::new(AppStates::create_un_initialized());
        let app_name = settings_reader.get_service_name();
        let app_version = settings_reader.get_service_version();

        my_logger::LOGGER
            .populate_app_and_version(app_name.clone(), app_version.clone())
            .await;

        SeqLogger::enable_from_connection_string(settings_reader.clone()).await;

        #[cfg(feature = "my-nosql-data-reader-sdk")]
        let my_no_sql_connection = Arc::new(MyNoSqlTcpConnection::new(
            app_name.clone(),
            settings_reader.clone(),
        ));

        #[cfg(feature = "my-service-bus")]
        let sb_client = Arc::new(MyServiceBusClient::new(
            app_name.clone(),
            app_version.clone(),
            settings_reader.clone(),
            my_logger::LOGGER.clone(),
        ));

        println!("Initialized service context");

        Self {
            http_server_builder: HttpServerBuilder::new(app_name.clone(), app_version.clone()),
            http_servers: vec![],
            telemetry_writer: MyTelemetryWriter::new(app_name.clone(), settings_reader.clone()),
            app_states,
            #[cfg(feature = "my-nosql-data-reader-sdk")]
            my_no_sql_connection,
            #[cfg(feature = "my-service-bus")]
            sb_client,
            app_name,
            app_version,
            #[cfg(feature = "grpc")]
            grpc_server_builder: None,
            background_timers: vec![],
        }
    }

    pub fn register_timer(&mut self, duration: Duration, builder: impl Fn(&mut MyTimer)) {
        let mut timer = MyTimer::new(duration);
        builder(&mut timer);

        self.background_timers.push(timer);
    }

    pub fn configure_http_server(&mut self, config: impl Fn(&mut HttpServerBuilder)) -> &mut Self {
        config(&mut self.http_server_builder);
        self
    }

    pub async fn start_application(&mut self) {
        self.app_states.set_initialized();
        self.telemetry_writer
            .start(self.app_states.clone(), my_logger::LOGGER.clone());
        for timer in self.background_timers.iter() {
            timer.start(self.app_states.clone(), my_logger::LOGGER.clone());
        }
        #[cfg(feature = "my-nosql-data-reader-sdk")]
        self.my_no_sql_connection.start().await;
        #[cfg(feature = "my-service-bus")]
        self.sb_client.start().await;

        let mut http_servers = self.http_server_builder.build();

        if std::env::var("HTTP2").is_ok() {
            for http_server in http_servers.iter_mut() {
                http_server.start_h2(self.app_states.clone(), my_logger::LOGGER.clone());
            }
        } else {
            for http_server in http_servers.iter_mut() {
                http_server.start(self.app_states.clone(), my_logger::LOGGER.clone());
            }
        }

        self.http_servers = http_servers;

        #[cfg(feature = "grpc")]
        if let Some(grpc_server_builder) = self.grpc_server_builder.as_mut() {
            grpc_server_builder.start(self.app_name.as_str());
        }

        println!("Application is stated");
        self.app_states.wait_until_shutdown().await;
    }

    //ns
    #[cfg(feature = "my-nosql-data-reader-sdk")]
    pub async fn get_ns_reader<
        TMyNoSqlEntity: my_no_sql_sdk::abstractions::MyNoSqlEntity
            + my_no_sql_sdk::abstractions::MyNoSqlEntitySerializer
            + Sync
            + Send
            + 'static,
    >(
        &self,
    ) -> Arc<my_no_sql_sdk::reader::MyNoSqlDataReaderTcp<TMyNoSqlEntity>> {
        let reader = self.my_no_sql_connection.get_reader().await;
        return reader;
    }

    //sb
    #[cfg(feature = "my-service-bus")]
    pub async fn register_sb_subscribe<
        TModel: GetMySbModelTopicId + MySbMessageDeserializer<Item = TModel> + Send + Sync + 'static,
    >(
        &self,
        callback: Arc<dyn SubscriberCallback<TModel> + Send + Sync + 'static>,
        queue_type: TopicQueueType,
    ) -> &Self {
        self.sb_client
            .subscribe(self.app_name.clone(), queue_type, callback)
            .await;

        self
    }

    #[cfg(feature = "my-service-bus")]
    pub async fn register_sb_subscriber_with_suffix<
        TModel: GetMySbModelTopicId + MySbMessageDeserializer<Item = TModel> + Send + Sync + 'static,
    >(
        &self,
        callback: Arc<dyn SubscriberCallback<TModel> + Send + Sync + 'static>,
        queue_type: TopicQueueType,
        suffix: impl Into<StrOrString<'static>>,
    ) -> &Self {
        let suffix: StrOrString<'static> = suffix.into();
        self.sb_client
            .subscribe(
                format!("{}{}", self.app_name.as_str(), suffix.as_str()),
                queue_type,
                callback,
            )
            .await;

        self
    }

    #[cfg(feature = "my-service-bus")]
    pub async fn get_sb_publisher<TModel: MySbMessageSerializer + GetMySbModelTopicId>(
        &self,
        do_retries: bool,
    ) -> MyServiceBusPublisher<TModel> {
        return self.sb_client.get_publisher(do_retries).await;
    }

    #[cfg(feature = "grpc")]
    pub fn configure_grpc_server(&mut self, config: impl Fn(&mut GrpcServerBuilder)) {
        match self.grpc_server_builder.as_mut() {
            Some(builder) => {
                config(builder);
            }
            None => {
                let mut grpc_server_builder = GrpcServerBuilder::new();
                config(&mut grpc_server_builder);
                self.grpc_server_builder = Some(grpc_server_builder);
            }
        }
    }
}
