use http::header::AUTHORIZATION;
use http::uri::{Authority, Scheme};
use http::Uri;
use hyper::body::HttpBody;
use hyper::client::connect::HttpConnector;
use k8s_openapi::api::core::v1 as corev1;
use k8s_openapi::{ResponseError, WatchResponse};
use native_tls::{Certificate, TlsConnector};
use std::error::Error;
use std::fs;
use std::path::Path;

/// Location of Kubernetes service account auth info.
const SERVICE_ACCOUNT_DIR: &str = "/run/secrets/kubernetes.io/serviceaccount";

/// Monitors PVCs and PVs, and local volumes.
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client = {
        // Read the cluster's CA certificate.
        let sa_dir = Path::new(SERVICE_ACCOUNT_DIR);
        let cluster_ca = fs::read(sa_dir.join("ca.crt"))?;
        let cluster_ca = Certificate::from_pem(&cluster_ca)?;

        // Make the client, which is configured to use HTTPS.
        let mut http = HttpConnector::new();
        http.enforce_http(false);
        let tls = TlsConnector::builder()
            .add_root_certificate(cluster_ca)
            .build()?;
        let tls = tokio_tls::TlsConnector::from(tls);
        let https = hyper_tls::HttpsConnector::from((http, tls));
        hyper::Client::builder().build(https)
    };

    // Read the authentication token.
    let token = {
        let sa_dir = Path::new(SERVICE_ACCOUNT_DIR);
        fs::read_to_string(sa_dir.join("token"))?
    };

    let (request, response_body) =
        corev1::PersistentVolumeClaim::watch_persistent_volume_claim_for_all_namespaces(
            Default::default(),
        )?;
    let mut request = request.map(hyper::Body::from);

    // Fix up the request to go to the right endpoint and use authentication.
    {
        let mut uri = request.uri().clone();
        let mut parts = uri.into_parts();
        parts.scheme = Some(Scheme::HTTPS);
        parts.authority = Some(Authority::from_static("kubernetes.default:443"));
        uri = Uri::from_parts(parts).unwrap();
        *request.uri_mut() = uri;
        let auth = format!("Bearer {}", token);
        let auth = auth.parse().unwrap();
        request.headers_mut().insert(AUTHORIZATION, auth);
    }

    let mut response = client.request(request).await?;
    let status_code = response.status();
    let mut response_body = response_body(status_code);
    while let Some(chunk) = response.body_mut().data().await {
        let chunk = chunk?;
        response_body.append_slice(&chunk);
        match response_body.parse() {
            Ok(WatchResponse::Ok(pvc_list)) => println!("{:?}", pvc_list),
            Ok(WatchResponse::Other(x)) => println!("Got unexpected type {:?}", x),
            Err(ResponseError::NeedMoreData) => continue,
            Err(x) => println!("Error parsing PVC: {:?}", x),
        }
    }

    Ok(())
}
