use http::header::AUTHORIZATION;
use http::uri::{Authority, Scheme};
use http::Uri;
use hyper::client::connect::HttpConnector;
use hyper_tls::HttpsConnector;
use native_tls::{Certificate, TlsConnector};
use std::error::Error;
use std::fs;
use std::path::Path;

/// Location of Kubernetes service account auth info.
const SERVICE_ACCOUNT_DIR: &str = "/run/secrets/kubernetes.io/serviceaccount";

pub struct Client {
    client: hyper::client::Client<HttpsConnector<HttpConnector>>,
    auth: http::header::HeaderValue,
}

impl Client {
    pub fn new() -> Result<Client, Box<dyn Error>> {
        // Read the cluster's CA certificate.
        let sa_dir = Path::new(SERVICE_ACCOUNT_DIR);
        let cluster_ca = fs::read(sa_dir.join("ca.crt"))?;
        let cluster_ca = Certificate::from_pem(&cluster_ca)?;

        // Read the authentication token and build the header.
        let token = {
            let sa_dir = Path::new(SERVICE_ACCOUNT_DIR);
            fs::read_to_string(sa_dir.join("token"))?
        };
        let auth = format!("Bearer {}", token);
        let auth = auth.parse().unwrap();

        // Make the client, which is configured to use HTTPS.
        let mut http = HttpConnector::new();
        http.enforce_http(false);
        let tls = TlsConnector::builder()
            .add_root_certificate(cluster_ca)
            .build()?;
        let tls = tokio_tls::TlsConnector::from(tls);
        let https = HttpsConnector::from((http, tls));
        let hyper_client = hyper::Client::builder().build(https);
        Ok(Client {
            client: hyper_client,
            auth,
        })
    }

    pub fn request(self, request: http::Request<Vec<u8>>) -> hyper::client::ResponseFuture {
        let mut request = request.map(hyper::Body::from);

        // Fix up the request to go to the right endpoint.
        {
            let mut uri = request.uri().clone();
            let mut parts = uri.into_parts();
            parts.scheme = Some(Scheme::HTTPS);
            parts.authority = Some(Authority::from_static("kubernetes.default:443"));
            uri = Uri::from_parts(parts).unwrap();
            *request.uri_mut() = uri;
        }

        // Use authentication.
        request.headers_mut().insert(AUTHORIZATION, self.auth);

        self.client.request(request)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
