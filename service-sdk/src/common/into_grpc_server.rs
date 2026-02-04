use std::convert::Infallible;

use my_grpc_extensions::tonic::{
    body::Body,
    codegen::{http::Request, Service},
    server::NamedService,
};

pub trait IntoGrpcServer {
    type GrpcServer: Service<
            Request<Body>,
            Response = my_grpc_extensions::hyper::Response<Body>,
            Error = Infallible,
        > + NamedService
        + Clone
        + Send
        + Sync
        + 'static;

    fn into_grpc_server(self) -> Self::GrpcServer;
}
