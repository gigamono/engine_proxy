// Copyright 2021 the Gigamono authors. All rights reserved. GPL-3.0 License.

use std::{convert::TryInto, net::IpAddr, sync::Arc};
use utilities::{
    errors::{self, HandlerError, HandlerErrorMessage},
    http,
    hyper::{Body, Request, Response},
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

        // Workspace uri.
        let workspace_forward_uri =
            &format!("http://{}", setup.config.engines.workspace.socket_address);

        // TODO(appcypher): Should get the right backend uri from a request to workspace.
        let backend_forward_uri =
            &format!("http://{}", setup.config.engines.backend.socket_address);

        // Routing.
        if path.starts_with("/api/") {
            // If the path starts with "/api/".
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
        let value = "unreachable"
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
