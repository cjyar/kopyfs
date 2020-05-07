use k8s_openapi::api::core::v1 as corev1;
use kube::api::Api;
use kube::client::Client;
use kube::config::Config;
use kube::runtime::Informer;
use std::error::Error;

extern crate kopyfs;

/// Monitors PVCs and PVs, and local volumes.
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let k8scfg = Config::infer();
    let k8s = Client::new(k8scfg);
    let pvcs: Api<corev1::PersistentVolumeClaim> = Api::all(k8s);
    let inform = Informer::new(pvcs);

    let pvcs = inform.poll().await?.boxed();
    while let Some(event) = pvcs.try_next().await? {
        match event {
            WatchEvent::Added(pvc) => println!("{}", pvc),
            WatchEvent::Modified(pvc) => println!("{}", pvc),
            WatchEvent::Deleted(pvc) => println!("{}", pvc),
        }
    }

    kopyfs::WrappedError;

    Ok(())
}
