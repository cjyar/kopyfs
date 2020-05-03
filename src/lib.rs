use http::header::AUTHORIZATION;
use http::uri::{Authority, Scheme};
use http::Uri;
use hyper::client::connect::HttpConnector;
use hyper_tls::HttpsConnector;
use native_tls::{Certificate, TlsConnector};
use std::error::Error;
use std::fs;
use std::path::Path;
use std::ffi::OsStr;

/// Wrapper around hyper Client with some added Kubernetes smarts.
pub struct Client {
    client: hyper::client::Client<HttpsConnector<HttpConnector>>,
    auth: http::header::HeaderValue,
}

impl Client {
    /// Location of Kubernetes service account auth info.
    const SERVICE_ACCOUNT_DIR: &'static str = "/run/secrets/kubernetes.io/serviceaccount";
    const CA_CERT_NAME: &'static str = "ca.crt";
    const AUTH_TOKEN_NAME: &'static str = "token";

    /// Create a new Client after reading Kubernetes in-cluster configuration.
    /// This must be run inside a Kubernetes pod.
    pub fn new() -> Result<Client, Box<dyn Error>> {
        Self::inner_new(OsStr::new(Self::SERVICE_ACCOUNT_DIR))
    }

    fn inner_new(sa_dir_name: &OsStr) -> Result<Client, Box<dyn Error>> {
        // Read the cluster's CA certificate.
        let sa_dir = Path::new(sa_dir_name);
        let cluster_ca = fs::read(sa_dir.join(Self::CA_CERT_NAME))?;
        let cluster_ca = Certificate::from_pem(&cluster_ca)?;

        // Read the authentication token and build the header.
        let token = fs::read_to_string(sa_dir.join(Self::AUTH_TOKEN_NAME))?;
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

    /// Execute an HTTP request and return the async result. Also set the scheme
    /// and authority to talk to the local cluster, and add authentication for
    /// the current pod's service account.
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
    const FAKE_CERT: &str = "-----BEGIN CERTIFICATE-----
MIIDXTCCAkWgAwIBAgIJAPDwP63dUr9uMA0GCSqGSIb3DQEBCwUAMEUxCzAJBgNV
BAYTAlVTMRMwEQYDVQQIDApTb21lLVN0YXRlMSEwHwYDVQQKDBhJbnRlcm5ldCBX
aWRnaXRzIFB0eSBMdGQwHhcNMjAwNTAzMjIyODM5WhcNMjAwNjAyMjIyODM5WjBF
MQswCQYDVQQGEwJVUzETMBEGA1UECAwKU29tZS1TdGF0ZTEhMB8GA1UECgwYSW50
ZXJuZXQgV2lkZ2l0cyBQdHkgTHRkMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIB
CgKCAQEA2UGUMwHA9I1TkLBs0BLCsrDw0hT5IfqJqaDQiW3+aNaAuUumD7YhMy9q
MckcXLe8cef3YXcKpD2k0R+4W1fWR6D3FXasWykgfXz+NbdQ7qkvvCy7fF2B3cWl
nraTJdnW7PdmovwF3qQN1ChYGQ4SmQmTSF4hRdZnFEq0y42Bp8QicqMJP1KdBpvM
vWBVn6Su9qJN+RkhoN1x3UnewmF8sDvMKCsEkrf0Iv/kzpvtWn79eYsJq4qk4Ch/
IlVurkcTMzsPkG8E1HvzCg9WWrtUZYwz+n96e/YKdUOvpzWtKS7gycPEjYCX7Cet
95oyOBE6uYT7NdsDo3L96Q0sH4NfPQIDAQABo1AwTjAdBgNVHQ4EFgQUhp+954j3
JiMLd0xDUmfjNT4z4fswHwYDVR0jBBgwFoAUhp+954j3JiMLd0xDUmfjNT4z4fsw
DAYDVR0TBAUwAwEB/zANBgkqhkiG9w0BAQsFAAOCAQEAvIlEKmQlQzkCJEu6hZ6p
V5gFQ/QICqWC+Ne+IW1WKM0OKOqQFaDuRyN4aiAwpCWBewLtehmIZxX0oSoryuI9
fSjqTyueLqcCRLJyeINb92+REPpfmmHKyTWx4ETSz9aLwIz/5DDk2EvSYhManJ+R
/CnWITcSqthR0EhqlXnHXBrbKIVD5xR/KKlfOFTfHtQCVkODjPqOziYEBkzxsY3G
/hpmwp21BzLVLsKr4133Q6lfaZvcEqx9x+dI+iAY8csgcTm2FmQyXcXgQLqroJPR
LUSto1CiXznuhRPLqMPhbEC5dmJiZECr5jgyBHy1FAYAp6ksmkUbySsFzl0xgnHX
/g==
-----END CERTIFICATE-----
";
    #[test]
    fn client_new() -> Result<(), std::io::Error> {
        use tempfile::tempdir;

        let tempdir = tempdir()?;
        let sa_dir = tempdir.path();

        // Write a mock CA cert.
        let cert_file = sa_dir.join(super::Client::CA_CERT_NAME);
        std::fs::write(cert_file, FAKE_CERT).unwrap();

        // Write a mock auth token.
        let token_file = sa_dir.join(super::Client::AUTH_TOKEN_NAME);
        std::fs::write(token_file, "foo").unwrap();

        // Create the client.
        super::Client::inner_new(tempdir.path().as_os_str()).unwrap();

        Ok(())
    }
}
