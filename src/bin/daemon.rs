use hyper::body::HttpBody;
use k8s_openapi::api::core::v1 as corev1;
use k8s_openapi::{ResponseError, WatchResponse};
use std::error::Error;

extern crate kopyfs;

/// Monitors PVCs and PVs, and local volumes.
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client = kopyfs::Client::new()?;

    let (request, response_body) =
        corev1::PersistentVolumeClaim::watch_persistent_volume_claim_for_all_namespaces(
            Default::default(),
        )?;

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
