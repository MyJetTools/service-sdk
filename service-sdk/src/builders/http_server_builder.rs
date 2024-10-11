use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use is_alive_middleware::IsAliveMiddleware;
use my_http_server::controllers::{
    swagger::SwaggerMiddleware,
    {
        actions::{
            DeleteAction, GetAction, GetDescription, HandleHttpRequest, PostAction, PutAction,
        },
        AuthErrorFactory, ControllersAuthorization, ControllersMiddleware,
    },
};
use my_http_server::{HttpServerMiddleware, MyHttpServer};
use rust_extensions::StrOrString;

use crate::{MetricsMiddleware, MetricsTechMiddleware};

pub struct HttpServerBuilder {
    listen_address: SocketAddr,
    auth_middleware: Option<Arc<dyn HttpServerMiddleware + Send + Sync + 'static>>,
    app_name: String,
    app_version: String,
    controllers: Option<ControllersMiddleware>,
    custom_middlewares: Vec<Arc<dyn HttpServerMiddleware + Send + Sync + 'static>>,
}
impl HttpServerBuilder {
    pub fn new(app_name: StrOrString<'static>, app_version: StrOrString<'static>) -> Self {
        Self {
            listen_address: SocketAddr::new(crate::consts::get_default_ip_address(), 8000),
            auth_middleware: None,

            controllers: None,
            app_name: app_name.to_string(),
            app_version: app_version.to_string(),
            custom_middlewares: vec![],
        }
    }

    pub fn set_authorization(&mut self, authorization: ControllersAuthorization) {
        if self.controllers.is_none() {
            self.controllers = Some(ControllersMiddleware::new(Some(authorization), None));
        } else {
            self.controllers
                .as_mut()
                .unwrap()
                .update_authorization_map(authorization);
        }
    }

    pub fn set_auth_error_factory(&mut self, value: impl AuthErrorFactory + Send + Sync + 'static) {
        if self.controllers.is_none() {
            self.controllers = Some(ControllersMiddleware::new(None, Some(Arc::new(value))));
        } else {
            self.controllers
                .as_mut()
                .unwrap()
                .update_auth_error_factory(Arc::new(value));
        }
    }

    pub fn register_custom_middleware(
        &mut self,
        middleware: Arc<dyn HttpServerMiddleware + Send + Sync + 'static>,
    ) {
        self.custom_middlewares.push(middleware);
    }

    pub fn update_listen_endpoint(&mut self, ip: IpAddr, port: u16) {
        self.listen_address = SocketAddr::new(ip, port);
    }

    pub fn add_auth_middleware(
        &mut self,
        middleware: Arc<dyn HttpServerMiddleware + Send + Sync + 'static>,
    ) -> &mut Self {
        self.auth_middleware = Some(middleware);
        return self;
    }

    pub fn register_get_action(
        &mut self,
        action: impl GetAction + Clone + HandleHttpRequest + GetDescription + Send + Sync + 'static,
    ) -> &mut Self {
        if self.controllers.is_none() {
            self.controllers = Some(ControllersMiddleware::new(None, None));
        }
        self.controllers
            .as_mut()
            .unwrap()
            .register_get_action(Arc::new(action));
        return self;
    }

    pub fn register_post_action(
        &mut self,
        action: impl PostAction
            + Clone
            + HandleHttpRequest
            + GetDescription
            + Clone
            + Send
            + Sync
            + 'static,
    ) -> &mut Self {
        if self.controllers.is_none() {
            self.controllers = Some(ControllersMiddleware::new(None, None));
        }
        self.controllers
            .as_mut()
            .unwrap()
            .register_post_action(Arc::new(action));
        return self;
    }

    pub fn register_put_action(
        &mut self,
        action: impl PutAction + Clone + HandleHttpRequest + GetDescription + Send + Sync + 'static,
    ) -> &mut Self {
        if self.controllers.is_none() {
            self.controllers = Some(ControllersMiddleware::new(None, None));
        }
        self.controllers
            .as_mut()
            .unwrap()
            .register_put_action(Arc::new(action));
        return self;
    }

    pub fn register_delete_action(
        &mut self,
        action: impl DeleteAction + HandleHttpRequest + GetDescription + Send + Sync + 'static,
    ) -> &mut Self {
        if self.controllers.is_none() {
            self.controllers = Some(ControllersMiddleware::new(None, None));
        }
        self.controllers
            .as_mut()
            .unwrap()
            .register_delete_action(Arc::new(action));
        return self;
    }

    pub fn build(&mut self) -> MyHttpServer {
        let mut my_http_server = MyHttpServer::new(self.listen_address);

        let is_alive = IsAliveMiddleware::new(self.app_name.clone(), self.app_version.clone());
        my_http_server.add_middleware(Arc::new(is_alive));
        my_http_server.add_middleware(Arc::new(MetricsMiddleware));
        my_http_server.add_tech_middleware(Arc::new(MetricsTechMiddleware));

        for middleware in self.custom_middlewares.drain(..) {
            my_http_server.add_middleware(middleware);
        }

        if let Some(controllers) = self.controllers.take() {
            let controllers = Arc::new(controllers);
            let swagger_middleware = SwaggerMiddleware::new(
                controllers.clone(),
                self.app_name.clone(),
                self.app_version.clone(),
            );

            my_http_server.add_middleware(Arc::new(swagger_middleware));

            if let Some(auth_middleware) = self.auth_middleware.take() {
                my_http_server.add_middleware(auth_middleware);
            }
            my_http_server.add_middleware(controllers.clone());
        }

        my_http_server
    }
}
