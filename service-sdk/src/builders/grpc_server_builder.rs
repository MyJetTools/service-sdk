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
use tokio::task::JoinHandle;

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
    listen_address: Option<SocketAddr>,
}

impl GrpcServerBuilder {
    pub fn new() -> Self {
        Self {
            server: None,
            listen_address: None,
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

    pub fn build(&mut self) -> GrpcServer {
        let grpc_addr = if let Some(taken) = self.listen_address {
            taken
        } else {
            let grpc_port = if let Ok(port) = std::env::var("GRPC_PORT") {
                match port.as_str().parse::<u16>() {
                    Ok(parsed) => parsed,
                    Err(_) => DEFAULT_GRPC_PORT,
                }
            } else {
                DEFAULT_GRPC_PORT
            };
            SocketAddr::new(crate::consts::get_default_ip_address(), grpc_port)
        };

        let mut grpc_server = GrpcServer::new(self.server.take().unwrap());
        grpc_server.start(grpc_addr);

        grpc_server
    }
}

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
        my_logger::LOGGER.write_info(
            "Starting GRPC Server".to_string(),
            format!("GRPC server starts at: {:?}", &grpc_addr),
            LogEventCtx::new(),
        );

        let server = self.server.take().unwrap();
        let result = tokio::spawn(async move {
            server.serve(grpc_addr).await.unwrap();
        });
        self.join_handle = Some(result);
    }
}
