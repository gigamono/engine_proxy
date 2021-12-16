// Copyright 2021 the Gigamono authors. All rights reserved. Apache 2.0 license.

use std::{convert::TryInto, net::IpAddr, sync::Arc};
use utilities::{
    errors::{self, HandlerError, HandlerErrorMessage},
    http::{self, Body, Request, Response},
    result::HandlerResult,
    setup::CommonSetup,
};

use crate::proxy::{Proxy, ProxyError};

pub struct Router;

impl Router {
    pub async fn route(
        mut request: Request<Body>,
        client_ip: IpAddr,
        setup: Arc<CommonSetup>,
    ) -> HandlerResult<Response<Body>> {
        let path = request.uri().path();
        let backend_forward_uri =
            &format!("http://127.0.0.1:{}", setup.config.engines.backend.port);
        let workspace_forward_uri =
            &format!("http://127.0.0.1:{}", setup.config.engines.workspace.port);

        // Routing.
        if path.starts_with("/r/") {
            // If the path starts with "/r/".

            Self::set_headers(&mut request)?;
            Proxy::call(client_ip, backend_forward_uri, request) // TODO(appcypher)
                .await
                .map_err(Self::proxy_error)
        } else if let Ok(_) = http::parse_url_path_number(path) {
            // If the path starts with a number (like "/2/system/load/prometheus/index.css").

            Self::set_headers(&mut request)?;
            Proxy::call(client_ip, backend_forward_uri, request) // TODO(appcypher)
                .await
                .map_err(Self::proxy_error)
        } else {
            // Everything else.

            Proxy::call(client_ip, workspace_forward_uri, request) // TODO(appcypher)
                .await
                .map_err(Self::proxy_error)
        }
    }

    fn set_headers(request: &mut Request<Body>) -> HandlerResult<()> {
        // TODO(appcypher): Get value from workspaces TiKV.
        let headers = request.headers_mut();
        let value = "9ccec027-68a3-47a2-bd3d-85a9c6faebfb"
            .try_into()
            .map_err(|err| HandlerError::Internal {
                ctx: HandlerErrorMessage::InternalError,
                src: errors::wrap_error("", err),
            })?;

        headers.insert(http::WORKSPACE_ID_HEADER, value);

        Ok(())
    }

    fn proxy_error(err: ProxyError) -> HandlerError {
        HandlerError::Internal {
            ctx: HandlerErrorMessage::InternalError,
            src: errors::new_error(format!("{:?}", err)),
        }
    }
}
