// Copyright 2021 the Gigamono authors. All rights reserved. GPL-3.0 License.
// Based on https://github.com/felipenoris/hyper-reverse-proxy. Copyright the hyper-reverse-proxy authors. GPL-3.0 License.

use lazy_static::lazy_static;
use std::net::IpAddr;
use std::str::FromStr;
use utilities::hyper::header::{Entry, HeaderMap, HeaderValue};
use utilities::hyper::http::header::{InvalidHeaderValue, ToStrError};
use utilities::hyper::http::uri::InvalidUri;
use utilities::hyper::{Body, Client, Error, Request, Response, Uri};

#[derive(Debug)]
pub enum ProxyError {
    InvalidUri(InvalidUri),
    HyperError(Error),
    ForwardHeaderError,
}

pub struct Proxy;

impl Proxy {
    pub async fn call(
        client_ip: IpAddr,
        forward_uri: &str,
        request: Request<Body>,
    ) -> Result<Response<Body>, ProxyError> {
        let proxied_request = Self::create_proxied_request(client_ip, &forward_uri, request)?;

        let client = Client::new();

        let response = client.request(proxied_request).await?;

        let proxied_response = Self::create_proxied_response(response);

        Ok(proxied_response)
    }

    fn is_hop_header(name: &str) -> bool {
        use unicase::Ascii;

        // A list of the headers, using `unicase` to help us compare without
        // worrying about the case, and `lazy_static!` to prevent reallocation
        // of the vector.
        lazy_static! {
            static ref HOP_HEADERS: Vec<Ascii<&'static str>> = vec![
                Ascii::new("Connection"),
                Ascii::new("Keep-Alive"),
                Ascii::new("Proxy-Authenticate"),
                Ascii::new("Proxy-Authorization"),
                Ascii::new("Te"),
                Ascii::new("Trailers"),
                Ascii::new("Transfer-Encoding"),
                Ascii::new("Upgrade"),
            ];
        }

        HOP_HEADERS.iter().any(|h| h == &name)
    }

    /// Returns a clone of the headers without the [hop-by-hop headers].
    ///
    /// [hop-by-hop headers]: http://www.w3.org/Protocols/rfc2616/rfc2616-sec13.html
    fn remove_hop_headers(headers: &HeaderMap<HeaderValue>) -> HeaderMap<HeaderValue> {
        let mut result = HeaderMap::new();
        for (k, v) in headers.iter() {
            if !Self::is_hop_header(k.as_str()) {
                result.insert(k.clone(), v.clone());
            }
        }
        result
    }

    fn create_proxied_response<B>(mut response: Response<B>) -> Response<B> {
        *response.headers_mut() = Self::remove_hop_headers(response.headers());
        response
    }

    fn forward_uri<B>(forward_url: &str, req: &Request<B>) -> Result<Uri, InvalidUri> {
        let forward_uri = match req.uri().query() {
            Some(query) => format!("{}{}?{}", forward_url, req.uri().path(), query),
            None => format!("{}{}", forward_url, req.uri().path()),
        };

        Uri::from_str(forward_uri.as_str())
    }

    fn create_proxied_request<B>(
        client_ip: IpAddr,
        forward_url: &str,
        mut request: Request<B>,
    ) -> Result<Request<B>, ProxyError> {
        *request.headers_mut() = Self::remove_hop_headers(request.headers());
        *request.uri_mut() = Self::forward_uri(forward_url, &request)?;

        let x_forwarded_for_header_name = "x-forwarded-for";

        // Add forwarding information in the headers
        match request.headers_mut().entry(x_forwarded_for_header_name) {
            Entry::Vacant(entry) => {
                entry.insert(client_ip.to_string().parse()?);
            }

            Entry::Occupied(mut entry) => {
                let addr = format!("{}, {}", entry.get().to_str()?, client_ip);
                entry.insert(addr.parse()?);
            }
        }

        Ok(request)
    }
}

impl From<Error> for ProxyError {
    fn from(err: Error) -> ProxyError {
        ProxyError::HyperError(err)
    }
}

impl From<InvalidUri> for ProxyError {
    fn from(err: InvalidUri) -> ProxyError {
        ProxyError::InvalidUri(err)
    }
}

impl From<ToStrError> for ProxyError {
    fn from(_err: ToStrError) -> ProxyError {
        ProxyError::ForwardHeaderError
    }
}

impl From<InvalidHeaderValue> for ProxyError {
    fn from(_err: InvalidHeaderValue) -> ProxyError {
        ProxyError::ForwardHeaderError
    }
}
