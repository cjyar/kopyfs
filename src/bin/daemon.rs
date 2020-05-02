// use http::Response;
use futures::executor::block_on;
use http::uri::{Authority, Scheme};
use http::Uri;
use hyper;
use hyper::body::HttpBody;
use k8s_openapi::api::core::v1 as corev1;
use k8s_openapi::{ListResponse, ResponseError};
use std::error::Error;

/// Monitors PVCs and PVs, and local volumes.
fn main() -> Result<(), Box<dyn Error>> {
    block_on(async_main())
}

async fn async_main() -> Result<(), Box<dyn Error>> {
    let connector = hyper_tls::HttpsConnector::new();
    let client = hyper::Client::builder().build(connector);

    let (request, response_body) =
        corev1::PersistentVolumeClaim::list_persistent_volume_claim_for_all_namespaces(
            Default::default(),
        )?;
    let mut request = request.map(hyper::Body::from);

    {
        let mut uri = request.uri().clone();
        let mut parts = uri.into_parts();
        parts.scheme = Some(Scheme::HTTPS);
        parts.authority = Some(Authority::from_static("kubernetes.default:443"));
        uri = Uri::from_parts(parts).unwrap();
        *request.uri_mut() = uri;
    }

    let mut response = client.request(request).await?;
    let status_code = response.status();
    let mut response_body = response_body(status_code);
    while let Some(chunk) = response.body_mut().data().await {
        let chunk = chunk?;
        response_body.append_slice(&chunk);
        match response_body.parse() {
            Ok(ListResponse::Ok(pvc_list)) => println!("{:?}", pvc_list),
            Ok(ListResponse::Other(x)) => println!("Got unexpected type {:?}", x),
            Err(ResponseError::NeedMoreData) => continue,
            Err(x) => println!("Error parsing PVC: {:?}", x),
        }
    }

    Ok(())
}
