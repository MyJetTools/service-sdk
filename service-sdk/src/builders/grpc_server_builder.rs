use std::{
    convert::Infallible,
    net::{IpAddr, SocketAddr},
};

use my_grpc_extensions::tonic::{
    body::Body,
    codegen::{http::Request, Service},
    server::NamedService,
    transport::{server::Router, Server},
};

use my_logger::LogEventCtx;

use crate::GrpcMetricsMiddlewareLayer;

const DEFAULT_GRPC_PORT: u16 = 8888;
pub struct GrpcServerBuilder {
    server: Option<
        Router<
            tower::layer::util::Stack<
                tower::layer::util::Stack<GrpcMetricsMiddlewareLayer, tower::layer::util::Identity>,
                tower::layer::util::Identity,
            >,
        >,
    >,

    #[cfg(unix)]
    server_unix_socket: Option<
        Router<
            tower::layer::util::Stack<
                tower::layer::util::Stack<GrpcMetricsMiddlewareLayer, tower::layer::util::Identity>,
                tower::layer::util::Identity,
            >,
        >,
    >,
    #[cfg(unix)]
    unix_socket_enabled: bool,

    listen_address: Option<SocketAddr>,
}

impl GrpcServerBuilder {
    pub fn new() -> Self {
        Self {
            server: None,
            listen_address: None,
            #[cfg(unix)]
            server_unix_socket: None,
            #[cfg(unix)]
            unix_socket_enabled: match std::env::var("UNIX_SOCKET") {
                Ok(value) => value == "1",
                Err(_) => false,
            },
        }
    }

    pub fn update_listen_endpoint(&mut self, ip: IpAddr, port: u16) {
        self.listen_address = Some(SocketAddr::new(ip, port));
    }

    pub fn add_grpc_service<S>(&mut self, svc: S)
    where
        S: Service<
                Request<Body>,
                Response = my_grpc_extensions::hyper::Response<Body>,
                Error = Infallible,
            > + NamedService
            + Clone
            + Send
            + Sync
            + 'static,
        S::Future: Send + 'static,
    {
        #[cfg(unix)]
        if self.unix_socket_enabled {
            match self.server_unix_socket.take() {
                Some(server_unix_socket) => {
                    let server_unix_socket = server_unix_socket.add_service(svc.clone());
                    self.server_unix_socket = Some(server_unix_socket);
                }
                None => {
                    let layer = tower::ServiceBuilder::new()
                        .layer(GrpcMetricsMiddlewareLayer::default())
                        .into_inner();

                    let server_unix_socket =
                        Server::builder().layer(layer).add_service(svc.clone());

                    self.server_unix_socket = Some(server_unix_socket);
                }
            };
        }

        match self.server.take() {
            Some(server) => {
                let server = server.add_service(svc);
                self.server = Some(server);
            }
            None => {
                let layer = tower::ServiceBuilder::new()
                    .layer(GrpcMetricsMiddlewareLayer::default())
                    .into_inner();

                let server = Server::builder().layer(layer).add_service(svc);

                self.server = Some(server);
            }
        };
    }

    #[deprecated(note = "Please use add_grpc_service several times")]
    pub fn add_grpc_services(
        &mut self,
        add_function: impl Fn(
            &mut Server<
                tower::layer::util::Stack<
                    tower::layer::util::Stack<
                        GrpcMetricsMiddlewareLayer,
                        tower::layer::util::Identity,
                    >,
                    tower::layer::util::Identity,
                >,
            >,
        ) -> Router<
            tower::layer::util::Stack<
                tower::layer::util::Stack<GrpcMetricsMiddlewareLayer, tower::layer::util::Identity>,
                tower::layer::util::Identity,
            >,
        >,
    ) {
        let layer = tower::ServiceBuilder::new()
            .layer(GrpcMetricsMiddlewareLayer::default())
            .into_inner();

        let mut server = Server::builder().layer(layer);

        let router = add_function(&mut server);

        self.server = Some(router);
    }

    pub fn start(&mut self, app_name: &str) {
        #[cfg(unix)]
        if let Some(grpc_server) = self.server_unix_socket.take() {
            let unix_socket_name =
                rust_extensions::file_utils::format_path(format!("~/grpc/{}", app_name));

            start_grpc_server_as_unix_socket(grpc_server, unix_socket_name.to_string());
        }

        if let Some(grpc_server) = self.server.take() {
            let grpc_addr = if let Some(taken) = self.listen_address {
                taken
            } else {
                let grpc_port = get_grpc_port();
                SocketAddr::new(crate::consts::get_default_ip_address(), grpc_port)
            };
            start_grpc_server(grpc_server, grpc_addr);
        }
    }
}

fn get_grpc_port() -> u16 {
    if let Ok(port) = std::env::var("GRPC_PORT") {
        match port.as_str().parse::<u16>() {
            Ok(parsed) => parsed,
            Err(_) => DEFAULT_GRPC_PORT,
        }
    } else {
        DEFAULT_GRPC_PORT
    }
}

fn start_grpc_server(
    server: Router<
        tower::layer::util::Stack<
            tower::layer::util::Stack<GrpcMetricsMiddlewareLayer, tower::layer::util::Identity>,
            tower::layer::util::Identity,
        >,
    >,
    grpc_addr: SocketAddr,
) {
    my_logger::LOGGER.write_info(
        "Starting GRPC Server".to_string(),
        format!("GRPC server starts at: {:?}", &grpc_addr),
        LogEventCtx::new(),
    );

    tokio::spawn(async move {
        server.serve(grpc_addr).await.unwrap();
    });
}

#[cfg(unix)]
fn start_grpc_server_as_unix_socket(
    server: Router<
        tower::layer::util::Stack<
            tower::layer::util::Stack<GrpcMetricsMiddlewareLayer, tower::layer::util::Identity>,
            tower::layer::util::Identity,
        >,
    >,
    unix_socket_addr: String,
) {
    my_logger::LOGGER.write_info(
        "Starting GRPC Server".to_string(),
        format!("GRPC server starts at: {:?}", &unix_socket_addr),
        LogEventCtx::new(),
    );

    tokio::spawn(async move {
        let uds = tokio::net::UnixListener::bind(unix_socket_addr).unwrap();
        let uds_stream = tokio_stream::wrappers::UnixListenerStream::new(uds);
        server.serve_with_incoming(uds_stream).await.unwrap();
    });
}

/*
pub struct GrpcServer {
    server: Option<
        Router<
            tower::layer::util::Stack<
                tower::layer::util::Stack<GrpcMetricsMiddlewareLayer, tower::layer::util::Identity>,
                tower::layer::util::Identity,
            >,
        >,
    >,
    join_handle: Option<JoinHandle<()>>,
}

impl GrpcServer {
    pub fn new(
        server: Router<
            tower::layer::util::Stack<
                tower::layer::util::Stack<GrpcMetricsMiddlewareLayer, tower::layer::util::Identity>,
                tower::layer::util::Identity,
            >,
        >,
    ) -> Self {
        Self {
            server: Some(server),
            join_handle: None,
        }
    }

    pub fn start(&mut self, grpc_addr: SocketAddr) {

    }
    #[cfg(unix)]
    pub fn start_unix_socket(&mut self, unix_socket_addr: String) {
        use tokio_stream::wrappers::UnixListenerStream;

        my_logger::LOGGER.write_info(
            "Starting GRPC Server".to_string(),
            format!("GRPC server starts at: {:?}", &unix_socket_addr),
            LogEventCtx::new(),
        );

        let result = tokio::spawn(async move {
            let uds = tokio::net::UnixListener::bind(unix_socket_addr).unwrap();
                let uds_stream = UnixListenerStream::new(uds);
                    Server::builder()
        .add_service(GreeterServer::new(greeter))
        .serve_with_incoming(uds_stream)
        .await?;
        });
    }
}
 */
