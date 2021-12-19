// Copyright 2021 the Gigamono authors. All rights reserved. Apache 2.0 license.

use futures_util::FutureExt;
use log::{error, info};
use std::{convert::Infallible, future::Future, net::IpAddr, panic::AssertUnwindSafe, sync::Arc};
use utilities::{
    http,
    hyper::{
        server::conn::AddrStream,
        service::{make_service_fn, service_fn},
        Body, Request, Response, Server,
    },
    ip,
    result::{HandlerResult, Result},
    setup::CommonSetup,
};

use crate::Router;

pub struct ProxyServer {
    setup: Arc<CommonSetup>,
}

impl ProxyServer {
    pub fn new(setup: Arc<CommonSetup>) -> Self {
        Self { setup }
    }

    pub async fn listen(&self) -> Result<()> {
        // Initialize logger.
        env_logger::init();

        // Get socket address.
        let addr = ip::parse_socket_address(&self.setup.config.engines.proxy.socket_address)?;

        info!(r#"Socket address = "{}""#, addr);

        let make_svc = make_service_fn(move |socket: &AddrStream| {
            let client_ip = socket.remote_addr().ip();

            info!(r#"Remote client ip = "{}""#, client_ip);

            let setup = Arc::clone(&self.setup);

            async move {
                Ok::<_, Infallible>(service_fn(move |request| {
                    Self::handler_panic_wrap(Router::route, request, client_ip, Arc::clone(&setup))
                }))
            }
        });

        let server = Server::bind(&addr).serve(make_svc);

        Ok(server.await?)
    }

    async fn handler_panic_wrap<F, Fut>(
        func: F,
        request: Request<Body>,
        client_ip: IpAddr,
        setup: Arc<CommonSetup>,
    ) -> std::result::Result<Response<Body>, Infallible>
    where
        F: FnOnce(Request<Body>, IpAddr, Arc<CommonSetup>) -> Fut,
        Fut: Future<Output = HandlerResult<Response<Body>>>,
    {
        match AssertUnwindSafe(func(request, client_ip, setup))
            .catch_unwind()
            .await
        {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(err)) => {
                error!("{:?}", err);
                Ok(err.as_hyper_response())
            }
            Err(err) => http::handle_panic_error_t(err),
        }
    }
}
