use arc_swap::ArcSwap;
use my_http_server::MyHttpServer;
use my_logger::my_seq_logger::{SeqLogger, SeqSettings};
use my_telemetry::my_telemetry_writer::{MyTelemetrySettings, MyTelemetryWriter};
use rust_extensions::{AppStates, ExactTimerInterval, MyExactTimer, MyTimer};

#[cfg(feature = "my-nosql-data-writer-sdk")]
use my_no_sql_sdk::data_writer::MyNoSqlWriterSettings;

#[cfg(feature = "my-nosql-data-reader-sdk")]
use my_no_sql_sdk::reader::*;

#[cfg(feature = "my-service-bus")]
use my_service_bus::{
    abstractions::{
        publisher::{MyServiceBusPublisher, PublisherWithInternalQueue},
        subscriber::{MySbMessageDeserializer, SubscriberCallback},
        GetMySbModelTopicId, MySbMessageSerializer,
    },
    client::{MyServiceBusClient, MyServiceBusSettings},
};

use std::{sync::Arc, time::Duration};

use crate::{EventsPerSecondCounter, EventsPerSecondTimerTick, HttpServerBuilder, ServiceInfo};

#[cfg(feature = "grpc")]
use crate::GrpcServerBuilder;

pub struct ServiceContext {
    pub http_server_builder: HttpServerBuilder,
    pub http_servers: Vec<MyHttpServer>,

    pub telemetry_writer: MyTelemetryWriter,
    pub app_states: Arc<AppStates>,
    pub app_name: &'static str,
    pub app_version: &'static str,
    pub background_timers: Vec<MyTimer>,
    pub background_exact_timers: Vec<MyExactTimer>,
    events_per_second_counters: Arc<ArcSwap<Vec<Arc<EventsPerSecondCounter>>>>,
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
            .populate_app_and_version(app_name, app_version)
            .await;

        SeqLogger::enable_from_connection_string(settings_reader.clone()).await;

        #[cfg(feature = "my-nosql-data-reader-sdk")]
        let my_no_sql_connection = Arc::new(MyNoSqlTcpConnection::new(
            app_name,
            settings_reader.clone(),
        ));

        #[cfg(feature = "my-service-bus")]
        let sb_client = Arc::new(MyServiceBusClient::new(
            app_name,
            app_version,
            settings_reader.clone(),
            my_logger::LOGGER.clone(),
        ));

        println!("Initialized service context");

        let events_per_second_counters: Arc<ArcSwap<Vec<Arc<EventsPerSecondCounter>>>> =
            Arc::new(ArcSwap::from_pointee(Vec::new()));

        let mut events_per_second_timer = MyTimer::new(Duration::from_secs(1));
        events_per_second_timer.set_first_tick_before_delay();
        events_per_second_timer.register_timer(
            "EventsPerSecond",
            Arc::new(EventsPerSecondTimerTick {
                counters: events_per_second_counters.clone(),
            }),
        );

        Self {
            http_server_builder: HttpServerBuilder::new(app_name, app_version),
            http_servers: vec![],
            telemetry_writer: MyTelemetryWriter::new(app_name, settings_reader.clone()),
            app_states,
            #[cfg(feature = "my-nosql-data-reader-sdk")]
            my_no_sql_connection,
            #[cfg(feature = "my-service-bus")]
            sb_client,
            app_name,
            app_version,
            #[cfg(feature = "grpc")]
            grpc_server_builder: None,
            background_timers: vec![events_per_second_timer],
            background_exact_timers: vec![],
            events_per_second_counters,
        }
    }

    pub fn register_events_per_second(
        &self,
        metric_name: impl Into<String>,
    ) -> Arc<EventsPerSecondCounter> {
        let counter = Arc::new(EventsPerSecondCounter::new(metric_name));
        self.events_per_second_counters.rcu(|prev| {
            let mut new: Vec<Arc<EventsPerSecondCounter>> = (**prev).clone();
            new.push(counter.clone());
            Arc::new(new)
        });
        counter
    }

    pub fn register_timer(&mut self, duration: Duration, builder: impl Fn(&mut MyTimer)) {
        let mut timer = MyTimer::new(duration);
        builder(&mut timer);

        self.background_timers.push(timer);
    }

    pub fn register_exact_timer(
        &mut self,
        interval: ExactTimerInterval,
        builder: impl Fn(&mut MyExactTimer),
    ) {
        let mut timer = MyExactTimer::new(interval);
        builder(&mut timer);

        self.background_exact_timers.push(timer);
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
        for timer in self.background_exact_timers.iter() {
            timer.start(self.app_states.clone(), my_logger::LOGGER.clone());
        }
        #[cfg(feature = "my-nosql-data-reader-sdk")]
        self.my_no_sql_connection.start().await;
        #[cfg(feature = "my-service-bus")]
        self.sb_client.start().await;

        let mut http_servers = self.http_server_builder.build();

        for http_server in http_servers.iter_mut() {
            http_server.start_auto(self.app_states.clone(), my_logger::LOGGER.clone());
        }

        self.http_servers = http_servers;

        #[cfg(feature = "grpc")]
        if let Some(grpc_server_builder) = self.grpc_server_builder.as_mut() {
            grpc_server_builder.start(self.app_name);
        }

        println!("Application is stated");
        self.app_states.wait_until_shutdown().await;
    }

    //ns
    #[cfg(feature = "my-nosql-data-reader-sdk")]
    pub fn get_ns_reader<
        TMyNoSqlEntity: my_no_sql_sdk::abstractions::MyNoSqlEntity
            + my_no_sql_sdk::abstractions::MyNoSqlEntitySerializer
            + Sync
            + Send
            + 'static,
    >(
        &self,
    ) -> Arc<my_no_sql_sdk::reader::MyNoSqlDataReaderTcp<TMyNoSqlEntity>> {
        self.my_no_sql_connection.get_reader()
    }

    //sb
    #[cfg(feature = "my-service-bus")]
    pub fn register_sb_subscribe<
        TModel: GetMySbModelTopicId + MySbMessageDeserializer<Item = TModel> + Send + Sync + 'static,
    >(
        &self,
        callback: Arc<dyn SubscriberCallback<TModel> + Send + Sync + 'static>,
       delete_on_no_subscribers: bool,
        single_connection: bool,
    ) -> &Self {
        self.sb_client
            .subscribe(self.app_name, delete_on_no_subscribers, single_connection, callback);

        self
    }

    #[cfg(feature = "my-service-bus")]
    pub fn register_sb_subscriber_with_suffix<
        TModel: GetMySbModelTopicId + MySbMessageDeserializer<Item = TModel> + Send + Sync + 'static,
    >(
        &self,
        callback: Arc<dyn SubscriberCallback<TModel> + Send + Sync + 'static>,
        delete_on_no_subscribers: bool,
        single_connection: bool,
        suffix: impl Into<rust_extensions::StrOrString<'static>>,
    ) -> &Self {
        let suffix: rust_extensions::StrOrString<'static> = suffix.into();
        self.sb_client
            .subscribe(
                format!("{}{}", self.app_name, suffix.as_str()),
                delete_on_no_subscribers,
                single_connection,
                callback,
            );

        self
    }

    #[cfg(feature = "my-service-bus")]
    pub fn register_sb_subscriber_with_suffix_as_env_info<
        TModel: GetMySbModelTopicId + MySbMessageDeserializer<Item = TModel> + Send + Sync + 'static,
    >(
        &self,
        callback: Arc<dyn SubscriberCallback<TModel> + Send + Sync + 'static>,
        delete_on_no_subscribers: bool,
        single_connection: bool,
    ) -> &Self {
        let env_info = std::env::var("ENV_INFO")
            .expect("ENV_INFO env variable is required for register_sb_subscriber_with_suffix_as_env_info");

        self.sb_client.subscribe(
            format!("{}-{}", self.app_name, env_info),
            delete_on_no_subscribers,
            single_connection,
            callback,
        );

        self
    }

    #[cfg(feature = "my-service-bus")]
    pub fn get_sb_publisher<TModel: MySbMessageSerializer + GetMySbModelTopicId>(
        &self,
        do_retries: bool,
    ) -> MyServiceBusPublisher<TModel> {
        self.sb_client.get_publisher(do_retries)
    }

    #[cfg(feature = "my-service-bus")]
    pub fn get_sb_publisher_with_internal_queue<
        TModel: MySbMessageSerializer + GetMySbModelTopicId,
    >(
        &self,
    ) -> PublisherWithInternalQueue<TModel> {
        self.sb_client.get_publisher_with_internal_queue()
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
