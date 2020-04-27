# Volume copier for Kubernetes

This is a [CSI](https://kubernetes-csi.github.io/docs/)-based
[PVC](https://kubernetes.io/docs/concepts/storage/persistent-volumes/) provider for
[Kubernetes](https://kubernetes.io/). It's not a distributed filesystem or network filesystem; it's just an operator
that automatically snapshots and copies your local volume to another node for backup. When the volume is needed on a
new node, it will be "restored" from a snapshot. All metadata about where the snapshots live is kept in etcd.

See the [developer guide](docs/developer-guide.md) for architecture.
     