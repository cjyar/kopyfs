use futures::stream::{StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::PersistentVolumeClaim;
use kube::api::{Api, WatchEvent};
use kube::client::Client;
use kube::config::Config;
use kube::runtime::Informer;
use std::error::Error;

extern crate kopyfs;

/// Monitors PVCs and PVs, and local volumes.
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let k8scfg = Config::infer().await?;
    let k8s = Client::new(k8scfg);
    let pvcs: Api<PersistentVolumeClaim> = Api::all(k8s);
    let inform = Informer::new(pvcs);

    let mut pvcs = inform.poll().await?.boxed();
    while let Some(event) = pvcs.try_next().await? {
        match event {
            WatchEvent::Added(pvc) => println!("{:?}", pvc),
            WatchEvent::Modified(pvc) => println!("{:?}", pvc),
            WatchEvent::Deleted(pvc) => println!("{:?}", pvc),
            WatchEvent::Bookmark(_) => {}
            WatchEvent::Error(err) => println!("{}", err),
        }
    }

    Ok(())
}
