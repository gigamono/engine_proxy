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
        let config = &setup.config;

        // Workspace uri.
        let workspace_forward_uri =
            &format!("http://{}", config.engines.workspace.socket_address);

        // TODO(appcypher): Should get the right runtime uri from a request to workspace.
        let runtime_forward_uri =
            &format!("http://{}", config.engines.runtime.socket_address);

        // Routing.
        if path.starts_with("/api/") {
            // If the path starts with "/api/".

            // Check if we can map multiple workspaces to a volume or db.
            if config.volume.multi_workspace || config.db.multi_workspace {
                Self::set_headers(&mut request)?;
            }

            Proxy::call(client_ip, runtime_forward_uri, request) // TODO(appcypher)
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
        // TODO(appcypher): Get workspace id from request to workspaces.
        let headers = request.headers_mut();

        // TODO(appcypher):
        // This should be a random string of exactly 15 UTF-8 codepoints..
        // The reason for 15 characters is because of the mysql db name restriction.
        // 15 out of 64 characters allowed by mysql are reserved for the workspace id.
        let value = "first_workspace"
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
