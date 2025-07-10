use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use is_alive_middleware::IsAliveMiddleware;
use my_http_server::controllers::{
    swagger::SwaggerMiddleware,
    {actions::*, AuthErrorFactory, ControllersAuthorization, ControllersMiddleware},
};
use my_http_server::{HttpServerMiddleware, MyHttpServer};
use rust_extensions::StrOrString;

use crate::{MetricsMiddleware, MetricsTechMiddleware};

#[derive(Default)]
pub struct HttpServerConfig {
    auth_middleware: Option<Arc<dyn HttpServerMiddleware + Send + Sync + 'static>>,
    custom_middlewares: Vec<Arc<dyn HttpServerMiddleware + Send + Sync + 'static>>,
    controllers: Option<ControllersMiddleware>,
}

impl HttpServerConfig {
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

    pub fn set_auth_error_factory(
        &mut self,
        value: Arc<impl AuthErrorFactory + Send + Sync + 'static>,
    ) {
        if self.controllers.is_none() {
            self.controllers = Some(ControllersMiddleware::new(None, Some(value)));
        } else {
            self.controllers
                .as_mut()
                .unwrap()
                .update_auth_error_factory(value);
        }
    }

    pub fn register_custom_middleware(
        &mut self,
        middleware: Arc<dyn HttpServerMiddleware + Send + Sync + 'static>,
    ) {
        self.custom_middlewares.push(middleware);
    }

    pub fn add_auth_middleware(
        &mut self,
        middleware: Arc<dyn HttpServerMiddleware + Send + Sync + 'static>,
    ) {
        self.auth_middleware = Some(middleware);
    }

    pub fn register_get_action(
        &mut self,
        action: Arc<
            impl GetAction + Clone + HandleHttpRequest + GetDescription + Send + Sync + 'static,
        >,
    ) {
        if self.controllers.is_none() {
            self.controllers = Some(ControllersMiddleware::new(None, None));
        }
        self.controllers
            .as_mut()
            .unwrap()
            .register_get_action(action);
    }

    pub fn register_post_action(
        &mut self,
        action: Arc<
            impl PostAction + Clone + HandleHttpRequest + GetDescription + Clone + Send + Sync + 'static,
        >,
    ) {
        if self.controllers.is_none() {
            self.controllers = Some(ControllersMiddleware::new(None, None));
        }
        self.controllers
            .as_mut()
            .unwrap()
            .register_post_action(action);
    }

    pub fn register_put_action(
        &mut self,
        action: Arc<
            impl PutAction + Clone + HandleHttpRequest + GetDescription + Send + Sync + 'static,
        >,
    ) {
        if self.controllers.is_none() {
            self.controllers = Some(ControllersMiddleware::new(None, None));
        }
        self.controllers
            .as_mut()
            .unwrap()
            .register_put_action(action);
    }

    pub fn register_delete_action(
        &mut self,
        action: Arc<impl DeleteAction + HandleHttpRequest + GetDescription + Send + Sync + 'static>,
    ) {
        if self.controllers.is_none() {
            self.controllers = Some(ControllersMiddleware::new(None, None));
        }
        self.controllers
            .as_mut()
            .unwrap()
            .register_delete_action(action);
    }

    pub fn register_options_action(
        &mut self,
        action: Arc<
            impl OptionsAction + HandleHttpRequest + GetDescription + Send + Sync + 'static,
        >,
    ) {
        if self.controllers.is_none() {
            self.controllers = Some(ControllersMiddleware::new(None, None));
        }
        self.controllers
            .as_mut()
            .unwrap()
            .register_options_action(action);
    }

    fn build(
        &mut self,
        my_http_server: &mut MyHttpServer,
        app_name: StrOrString<'static>,
        app_version: StrOrString<'static>,
    ) {
        let is_alive = IsAliveMiddleware::new(
            app_name.as_str().to_string(),
            app_version.as_str().to_string(),
        );
        my_http_server.add_middleware(Arc::new(is_alive));
        my_http_server.add_middleware(Arc::new(MetricsMiddleware));
        my_http_server.add_tech_middleware(Arc::new(MetricsTechMiddleware));

        for middleware in self.custom_middlewares.drain(..) {
            my_http_server.add_middleware(middleware);
        }

        if let Some(controllers) = self.controllers.take() {
            let controllers = Arc::new(controllers);
            let swagger_middleware =
                SwaggerMiddleware::new(controllers.clone(), app_name.clone(), app_version.clone());

            my_http_server.add_middleware(Arc::new(swagger_middleware));

            if let Some(auth_middleware) = self.auth_middleware.take() {
                my_http_server.add_middleware(auth_middleware);
            }
            my_http_server.add_middleware(controllers.clone());
        }
    }
}

pub struct HttpServerBuilder {
    listen_address: SocketAddr,

    app_name: StrOrString<'static>,
    app_version: StrOrString<'static>,

    tcp: HttpServerConfig,

    #[cfg(unix)]
    unix_socket: Option<HttpServerConfig>,
}
impl HttpServerBuilder {
    pub fn new(app_name: StrOrString<'static>, app_version: StrOrString<'static>) -> Self {
        Self {
            listen_address: SocketAddr::new(crate::consts::get_default_ip_address(), 8000),
            app_name,
            app_version,
            tcp: HttpServerConfig::default(),
            #[cfg(unix)]
            unix_socket: if super::unix_socket_enabled() {
                Some(HttpServerConfig::default())
            } else {
                None
            },
        }
    }

    pub fn set_authorization(&mut self, authorization: ControllersAuthorization) {
        #[cfg(unix)]
        if let Some(unix_socket) = self.unix_socket.as_mut() {
            unix_socket.set_authorization(authorization.clone());
        }

        self.tcp.set_authorization(authorization);
    }

    pub fn set_auth_error_factory(&mut self, value: impl AuthErrorFactory + Send + Sync + 'static) {
        let value = Arc::new(value);
        #[cfg(unix)]
        if let Some(unix_socket) = self.unix_socket.as_mut() {
            unix_socket.set_auth_error_factory(value.clone());
        }

        self.tcp.set_auth_error_factory(value);
    }

    pub fn register_custom_middleware(
        &mut self,
        middleware: Arc<dyn HttpServerMiddleware + Send + Sync + 'static>,
    ) {
        #[cfg(unix)]
        if let Some(unix_socket) = self.unix_socket.as_mut() {
            unix_socket.register_custom_middleware(middleware.clone());
        }

        self.tcp.register_custom_middleware(middleware);
    }

    pub fn update_listen_endpoint(&mut self, ip: IpAddr, port: u16) {
        self.listen_address = SocketAddr::new(ip, port);
    }

    pub fn add_auth_middleware(
        &mut self,
        middleware: Arc<dyn HttpServerMiddleware + Send + Sync + 'static>,
    ) -> &mut Self {
        #[cfg(unix)]
        if let Some(unix_socket) = self.unix_socket.as_mut() {
            unix_socket.add_auth_middleware(middleware.clone());
        }

        self.tcp.add_auth_middleware(middleware);
        return self;
    }

    pub fn register_get_action(
        &mut self,
        action: impl GetAction + Clone + HandleHttpRequest + GetDescription + Send + Sync + 'static,
    ) -> &mut Self {
        let action = Arc::new(action);
        #[cfg(unix)]
        if let Some(unix_socket) = self.unix_socket.as_mut() {
            unix_socket.register_get_action(action.clone());
        }

        self.tcp.register_get_action(action);
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
        let action = Arc::new(action);
        #[cfg(unix)]
        if let Some(unix_socket) = self.unix_socket.as_mut() {
            unix_socket.register_post_action(action.clone());
        }

        self.tcp.register_post_action(action);

        return self;
    }

    pub fn register_put_action(
        &mut self,
        action: impl PutAction + Clone + HandleHttpRequest + GetDescription + Send + Sync + 'static,
    ) -> &mut Self {
        let action = Arc::new(action);
        #[cfg(unix)]
        if let Some(unix_socket) = self.unix_socket.as_mut() {
            unix_socket.register_put_action(action.clone());
        }

        self.tcp.register_put_action(action);
        return self;
    }

    pub fn register_delete_action(
        &mut self,
        action: impl DeleteAction + HandleHttpRequest + GetDescription + Send + Sync + 'static,
    ) -> &mut Self {
        let action = Arc::new(action);
        #[cfg(unix)]
        if let Some(unix_socket) = self.unix_socket.as_mut() {
            unix_socket.register_delete_action(action.clone());
        }

        self.tcp.register_delete_action(action);
        return self;
    }

    pub fn register_options_action(
        &mut self,
        action: impl OptionsAction + HandleHttpRequest + GetDescription + Send + Sync + 'static,
    ) -> &mut Self {
        let action = Arc::new(action);
        #[cfg(unix)]
        if let Some(unix_socket) = self.unix_socket.as_mut() {
            unix_socket.register_options_action(action.clone());
        }

        self.tcp.register_options_action(action);
        return self;
    }

    pub fn build(&mut self) -> Vec<MyHttpServer> {
        let mut result = vec![];
        #[cfg(unix)]
        if let Some(unix_socket) = self.unix_socket.as_mut() {
            let unix_socket_name =
                rust_extensions::file_utils::format_path(format!("~/http/{}", self.app_name));

            let mut my_http_server = MyHttpServer::new_as_unix_socket(unix_socket_name.to_string());

            unix_socket.build(
                &mut my_http_server,
                self.app_name.clone(),
                self.app_version.clone(),
            );
            result.push(my_http_server);
        }

        let mut my_http_server = MyHttpServer::new(self.listen_address);
        self.tcp.build(
            &mut my_http_server,
            self.app_name.clone(),
            self.app_version.clone(),
        );

        result.push(my_http_server);

        result
    }
}
