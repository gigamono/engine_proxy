// Copyright 2021 the Gigamono authors. All rights reserved. Apache 2.0 license.

use std::{convert::TryInto, net::IpAddr};

use utilities::{
    errors::{self, HandlerError, HandlerErrorMessage},
    http::{self, Body, Request, Response, StatusCode},
    result::HandlerResult,
};

use crate::proxy::{Proxy, ProxyError};

pub struct Router;

impl Router {
    pub async fn route(
        mut request: Request<Body>,
        client_ip: IpAddr,
    ) -> HandlerResult<Response<Body>> {
        let path = request.uri().path();

        // TODO(appcypher): Support "/" and "/workspaces/" routes.
        // Routing.
        if path.starts_with("/r/") {
            // If the path starts with "/r/".

            Self::set_headers(&mut request)?;
            Proxy::call(client_ip, "http://127.0.0.1:5051", request) // TODO(appcypher)
                .await
                .map_err(Self::proxy_error)
        } else if let Ok(_) = http::parse_url_path_number(path) {
            // If the path starts with a number (like "/2/system/load/prometheus/index.css").

            Self::set_headers(&mut request)?;
            Proxy::call(client_ip, "http://127.0.0.1:5051", request) // TODO(appcypher)
                .await
                .map_err(Self::proxy_error)
        } else {
            Err(HandlerError::Client {
                ctx: HandlerErrorMessage::NotFound,
                code: StatusCode::NOT_FOUND,
                src: errors::new_error(format!(r#"resource not found "{}""#, path)),
            })
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
