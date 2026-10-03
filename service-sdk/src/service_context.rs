#[cfg(feature = "with-prometheus-metrics")]
use arc_swap::ArcSwap;
use my_http_server::MyHttpServer;
use my_logger::my_seq_logger::{SeqLogger, SeqSettings};
use my_telemetry::my_telemetry_writer::{MyTelemetrySettings, MyTelemetryWriter};
use rust_extensions::{
    background_executor::BackgroundExecutor,
    background_executor_with_multi_threads::BackgroundExecutorWithMultiThreads,
    events_loop::EventsLoop, AppStates, ExactTimerInterval, MyExactTimer, MyTimer,
    PersistObjectId, QueueToSave, QueueToSaveAsBulk, QueueToSaveOrDeleteWithId, QueueToSaveWithId,
    Startable, StrOrString,
};

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

use std::{fmt::Debug, hash::Hash, sync::Arc, time::Duration};

use crate::{EventsPerSecondCounter, HttpServerBuilder, ServiceInfo};

#[cfg(feature = "with-prometheus-metrics")]
use crate::EventsPerSecondTimerTick;

#[cfg(feature = "grpc")]
use crate::GrpcServerBuilder;

// How often `start_application` reports to the console that it is still
// waiting for MyNoSql - the reason the application has not started yet.
#[cfg(feature = "my-nosql-data-reader-sdk")]
const NS_FIRST_DATA_NOTICE_INTERVAL: Duration = Duration::from_secs(5);

pub struct ServiceContext {
    pub http_server_builder: HttpServerBuilder,
    pub http_servers: Vec<MyHttpServer>,

    pub telemetry_writer: MyTelemetryWriter,
    pub app_states: Arc<AppStates>,
    pub app_name: &'static str,
    pub app_version: &'static str,
    // Everything `start_application` starts once the app is initialized: the
    // timers and whatever was created by `create_*` or handed over to
    // `register_startable`. Behind a mutex, so those take `&self` - the same
    // way `get_ns_reader` does.
    startables: parking_lot::Mutex<Vec<Arc<dyn Startable + Send + Sync + 'static>>>,
    #[cfg(feature = "with-prometheus-metrics")]
    events_per_second_counters: Arc<ArcSwap<Vec<Arc<EventsPerSecondCounter>>>>,
    #[cfg(feature = "my-nosql-data-reader-sdk")]
    pub my_no_sql_connection: Arc<MyNoSqlTcpConnection>,
    // Every reader handed out by `get_ns_reader`. `start_application` waits for
    // the first data of each of them before it starts anything else.
    #[cfg(feature = "my-nosql-data-reader-sdk")]
    ns_readers: parking_lot::Mutex<Vec<Arc<dyn NsReaderFirstData + Send + Sync + 'static>>>,
    #[cfg(feature = "my-service-bus")]
    pub sb_client: Arc<MyServiceBusClient>,
    #[cfg(feature = "grpc")]
    pub grpc_server_builder: Option<GrpcServerBuilder>,
}

impl ServiceContext {
    pub async fn new(settings_reader: service_sdk_macros::generate_settings_signature!()) -> Self {
        // Installs the prometheus recorder for the `metrics` facade. Without
        // the feature there is no recorder and no prometheus registry at all.
        #[cfg(feature = "with-prometheus-metrics")]
        metrics_prometheus::install();

        // Either provider feature brings the same `install_default_crypto_providers`
        // signature; my-tls picks ring when both are on. Idempotent - the first
        // caller in the process wins.
        #[cfg(any(
            feature = "with-ring-tls",
            feature = "with-rust-tls"
        ))]
        my_tls::install_default_crypto_providers();

        let app_states = Arc::new(AppStates::create_un_initialized());
        let app_name = settings_reader.get_service_name();
        let app_version = settings_reader.get_service_version();

        my_logger::LOGGER.populate_app_and_version(app_name, app_version);

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

        // The per-second timer is what turns the counters into prometheus
        // gauges. With no prometheus there is nothing for it to publish, so it
        // is not created and no background timer runs.
        #[cfg(feature = "with-prometheus-metrics")]
        let events_per_second_counters: Arc<ArcSwap<Vec<Arc<EventsPerSecondCounter>>>> =
            Arc::new(ArcSwap::from_pointee(Vec::new()));

        #[cfg(feature = "with-prometheus-metrics")]
        let startables: Vec<Arc<dyn Startable + Send + Sync + 'static>> = {
            let mut events_per_second_timer =
                MyTimer::new(Duration::from_secs(1), my_logger::LOGGER.clone());
            events_per_second_timer.set_first_tick_before_delay();
            events_per_second_timer.register_timer(
                "EventsPerSecond",
                Arc::new(EventsPerSecondTimerTick {
                    counters: events_per_second_counters.clone(),
                }),
            );
            vec![Arc::new(events_per_second_timer)]
        };

        #[cfg(not(feature = "with-prometheus-metrics"))]
        let startables = vec![];

        Self {
            http_server_builder: HttpServerBuilder::new(app_name, app_version),
            http_servers: vec![],
            telemetry_writer: MyTelemetryWriter::new(app_name, settings_reader.clone()),
            app_states,
            #[cfg(feature = "my-nosql-data-reader-sdk")]
            my_no_sql_connection,
            #[cfg(feature = "my-nosql-data-reader-sdk")]
            ns_readers: parking_lot::Mutex::new(Vec::new()),
            #[cfg(feature = "my-service-bus")]
            sb_client,
            app_name,
            app_version,
            #[cfg(feature = "grpc")]
            grpc_server_builder: None,
            startables: parking_lot::Mutex::new(startables),
            #[cfg(feature = "with-prometheus-metrics")]
            events_per_second_counters,
        }
    }

    pub fn register_events_per_second(
        &self,
        metric_name: impl Into<String>,
    ) -> Arc<EventsPerSecondCounter> {
        let counter = Arc::new(EventsPerSecondCounter::new(metric_name));

        // Without `with-prometheus-metrics` the counter is a no-op and there is
        // no registry to attach it to - the signature is kept so service code
        // does not need its own #[cfg].
        #[cfg(feature = "with-prometheus-metrics")]
        self.events_per_second_counters.rcu(|prev| {
            let mut new: Vec<Arc<EventsPerSecondCounter>> = (**prev).clone();
            new.push(counter.clone());
            Arc::new(new)
        });

        counter
    }

    pub fn register_timer(&mut self, duration: Duration, builder: impl Fn(&mut MyTimer)) {
        let mut timer = MyTimer::new(duration, my_logger::LOGGER.clone());
        builder(&mut timer);

        self.startables.get_mut().push(Arc::new(timer));
    }

    pub fn register_exact_timer(
        &mut self,
        interval: ExactTimerInterval,
        builder: impl Fn(&mut MyExactTimer),
    ) {
        let mut timer = MyExactTimer::new(interval, my_logger::LOGGER.clone());
        builder(&mut timer);

        self.startables.get_mut().push(Arc::new(timer));
    }

    // Hands over to `start_application` anything which is to be started once
    // the app is initialized - and gives it back as an `Arc` to keep. That is
    // the way in for what `create_*` below does not cover: a component built
    // with non-default settings, or a `Startable` of your own.
    pub fn register_startable<TStartable: Startable + Send + Sync + 'static>(
        &self,
        startable: TStartable,
    ) -> Arc<TStartable> {
        let startable = Arc::new(startable);
        self.startables.lock().push(startable.clone());
        startable
    }

    // The building blocks of rust-extensions, wired to the logger and the
    // application states of the SDK. Each one is started by
    // `start_application` - so its handler has to be registered before that,
    // otherwise `start` panics.
    pub fn create_events_loop<TModel: Send + 'static>(
        &self,
        name: impl Into<StrOrString<'static>>,
    ) -> Arc<EventsLoop<TModel>> {
        self.register_startable(EventsLoop::new(
            name,
            self.app_states.clone(),
            my_logger::LOGGER.clone(),
        ))
    }

    pub fn create_background_executor(
        &self,
        name: impl Into<StrOrString<'static>>,
    ) -> Arc<BackgroundExecutor> {
        self.register_startable(BackgroundExecutor::new(name, my_logger::LOGGER.clone()))
    }

    pub fn create_background_executor_with_multi_threads<TThreadId>(
        &self,
        name: impl Into<StrOrString<'static>>,
    ) -> Arc<BackgroundExecutorWithMultiThreads<TThreadId>>
    where
        TThreadId: Hash + Eq + Clone + Send + Sync + 'static,
    {
        self.register_startable(BackgroundExecutorWithMultiThreads::new(
            name,
            my_logger::LOGGER.clone(),
        ))
    }

    pub fn create_queue_to_save<T: Send + Sync + 'static>(
        &self,
        name: impl Into<StrOrString<'static>>,
    ) -> Arc<QueueToSave<T>> {
        self.register_startable(QueueToSave::new(name, my_logger::LOGGER.clone()))
    }

    pub fn create_queue_to_save_as_bulk<T: Send + Sync + 'static>(
        &self,
        name: impl Into<StrOrString<'static>>,
    ) -> Arc<QueueToSaveAsBulk<T>> {
        self.register_startable(QueueToSaveAsBulk::new(name, my_logger::LOGGER.clone()))
    }

    pub fn create_queue_to_save_with_id<ID, T>(
        &self,
        name: impl Into<StrOrString<'static>>,
    ) -> Arc<QueueToSaveWithId<ID, T>>
    where
        ID: Hash + Eq + Clone + Debug + Send + Sync + 'static,
        T: PersistObjectId<ID> + Send + Sync + 'static,
    {
        self.register_startable(QueueToSaveWithId::new(name, my_logger::LOGGER.clone()))
    }

    pub fn create_queue_to_save_or_delete_with_id<ID, T>(
        &self,
        name: impl Into<StrOrString<'static>>,
    ) -> Arc<QueueToSaveOrDeleteWithId<ID, T>>
    where
        ID: Hash + Eq + Clone + Debug + Send + Sync + 'static,
        T: PersistObjectId<ID> + Send + Sync + 'static,
    {
        self.register_startable(QueueToSaveOrDeleteWithId::new(
            name,
            my_logger::LOGGER.clone(),
        ))
    }

    pub fn configure_http_server(&mut self, config: impl Fn(&mut HttpServerBuilder)) -> &mut Self {
        config(&mut self.http_server_builder);
        self
    }

    pub async fn start_application(&mut self) {
        // MyNoSql goes first and everything else waits for it. Until every
        // reader handed out by `get_ns_reader` has its first snapshot the app
        // is not marked as initialized and nothing else is started - neither
        // the timers, nor whatever was created by `create_*` or handed over to
        // `register_startable`, nor the service bus, nor the HTTP and gRPC
        // servers.
        #[cfg(feature = "my-nosql-data-reader-sdk")]
        {
            self.my_no_sql_connection.start().await;
            self.wait_until_ns_readers_get_first_data().await;
        }

        self.app_states.set_initialized();
        self.telemetry_writer.start(my_logger::LOGGER.clone());
        for startable in self.startables.get_mut().iter() {
            startable.start();
        }
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
        let reader = self.my_no_sql_connection.get_reader();

        // Remembered, so `start_application` can wait for its first data.
        self.ns_readers.lock().push(reader.clone());

        reader
    }

    #[cfg(feature = "my-nosql-data-reader-sdk")]
    async fn wait_until_ns_readers_get_first_data(&self) {
        // Cloned out, so the guard is not held across the awaits below.
        let readers = self.ns_readers.lock().clone();

        // One at a time is enough: the connection fills every table on its
        // own, whoever awaits it, so the total is the slowest table either way.
        for reader in readers {
            while tokio::time::timeout(
                NS_FIRST_DATA_NOTICE_INTERVAL,
                reader.wait_until_first_data_arrives(),
            )
            .await
            .is_err()
            {
                println!(
                    "MyNoSql readers are not initialized: table '{}' has no data yet - start of application is delayed",
                    reader.get_table_name()
                );
            }
        }
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

// A `MyNoSqlDataReaderTcp` with its entity type erased - that is what lets
// readers of different tables sit in one list for `start_application` to wait
// on.
#[cfg(feature = "my-nosql-data-reader-sdk")]
#[async_trait::async_trait]
trait NsReaderFirstData {
    fn get_table_name(&self) -> &'static str;
    async fn wait_until_first_data_arrives(&self);
}

#[cfg(feature = "my-nosql-data-reader-sdk")]
#[async_trait::async_trait]
impl<TMyNoSqlEntity> NsReaderFirstData for MyNoSqlDataReaderTcp<TMyNoSqlEntity>
where
    TMyNoSqlEntity: my_no_sql_sdk::abstractions::MyNoSqlEntity
        + my_no_sql_sdk::abstractions::MyNoSqlEntitySerializer
        + Sync
        + Send
        + 'static,
{
    fn get_table_name(&self) -> &'static str {
        TMyNoSqlEntity::TABLE_NAME
    }

    async fn wait_until_first_data_arrives(&self) {
        MyNoSqlDataReader::wait_until_first_data_arrives(self).await
    }
}
