// Copyright 2021 the Gigamono authors. All rights reserved. Apache 2.0 license.

use log::{error, info};
use std::{
    convert::Infallible,
    future::Future,
    net::{IpAddr, SocketAddr},
    sync::Arc,
};
use utilities::{
    http::{
        server::conn::AddrStream,
        service::{make_service_fn, service_fn},
        Body, Request, Response, Server,
    },
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

        // Get port info and create socket address.
        let port = self.setup.config.engines.proxy.port;
        let addr = SocketAddr::from(([127, 0, 0, 1], port));

        info!(r#"Socket address = "{}""#, addr);

        let make_svc = make_service_fn(|socket: &AddrStream| {
            let client_ip = socket.remote_addr().ip();

            info!(r#"Remote client ip = "{}""#, client_ip);

            async move {
                Ok::<_, Infallible>(service_fn(move |request| {
                    Self::handler_error_wrap(Router::route, request, client_ip)
                }))
            }
        });

        let server = Server::bind(&addr).serve(make_svc);

        Ok(server.await?)
    }

    #[inline]
    async fn handler_error_wrap<F, Fut>(
        func: F,
        request: Request<Body>,
        client_ip: IpAddr,
    ) -> std::result::Result<Response<Body>, Infallible>
    where
        F: FnOnce(Request<Body>, IpAddr) -> Fut,
        Fut: Future<Output = HandlerResult<Response<Body>>>,
    {
        match func(request, client_ip).await {
            Ok(response) => Ok(response),
            Err(err) => {
                // Log error.
                error!("{:?}", err.system_error());

                // Return error response.
                Ok(err.as_hyper_response())
            }
        }
    }
}
