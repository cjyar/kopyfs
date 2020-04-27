# Components

- DaemonSet. Installs the CSI driver on every applicable node.
- CSI driver. Receives requests from kubelet to mount or unmount volumes.
    - Use https://kubernetes-csi.github.io/ to implement this.
- Volume manager. Pod created to manage a replica of a volume on a node.
    - Primary replica. In active use by another pod; manages a volume that's mounted.
    - Secondary replica. Owned by a ReplicaSet.

# Kubernetes data objects

- PersistentVolumeClaim. Has a `spec.storageClassName` that references a StorageClass that we handle.
- PersistentVolume. Has a `spec.csi.volumeAttributes` object that we use to store:
    - `replicationFactor`. Desired number of replicas, including the primary.
    - `replicas`. List of replicas of this filesystem. Each item has these attributes:
        - `node`. The name of the node where the replica is.
        - `snapshot`. The most recent snapshot in the replica.
        - `active`. True if this is a mounted primary with read/write access.
- ReplicaSet. Owns secondaries for each volume.

# Use cases

## Volume create

1. PVC object gets created in the Kubernetes apiserver. PVC has a `spec.storageClassName` that references a StorageClass
    that we handle.
1. Each daemon gets notified because it's watching for PVCs.
1. The DaemonSet leader creates a PersistentVolume and associates it with the PVC. Sets
    `PersistentVolume...replicationFactor` = `StorageClass.parameters.defaultReplicationFactor`.
1. The DaemonSet leader creates a ReplicaSet with `ReplicaSet.spec.replicas` = `PersistentVolume...replicationFactor`.
1. It also creates a PodDisruptionBudget.

## Daemon startup

1. Daemon starts watching for PVCs and PVs.
1. Daemon installs the CSI driver.
1. Daemon scans ZFS volumes for user properties indicating they belong to StorageClasses we're responsible for. For each
    one, it ensures a volume manager pod is running with metadata forcing it on this node and belonging to the
    appropriate ReplicaSet.

## Volume manager startup

1. It reads metadata (user properties) from the local volume, if any.
1. It creates/updates an entry on the PV with its snapshot number, which is 0 if no volume exists.
1. It watches the PV. If any replicas have newer snapshots, it requests them.
1. If our snapshot number is close to the largest known snapshot number, it signals it's ready.

## Volume mount

1. Kubernetes decides which node to schedule the pod on.
1. Kubelet calls our CSI driver to mount the volume.
1. CSI driver forwards the request to the local volume manager pod.
1. Volume manager:
    1. Modifies its metadata (labels and ownerReferences) so it no longer belongs to the ReplicaSet.
    1. Sets `ReplicaSet.spec.replicas` = `PersistentVolume...replicationFactor - 1`.
    1. Retrieves the latest snapshot data if necessary.
    1. Marks itself as `active` on the PV's replica list, if the mount is read/write.
    1. Returns mount info to the CSI driver.
1. CSI driver returns mount info to kubelet.

## Volume unmount

1. Kubelet calls our CSI driver to unmount the volume.
1. CSI driver forwards the request to the local volume manager pod.
1. Volume manager:
    1. Sets `ReplicaSet.spec.replicas` = `PersistentVolume...replicationFactor`.
    1. Modifies its metadata (labels and ownerReferences) so it belongs to the ReplicaSet.
    1. Performs a snapshot and updates the PV replica list.
    1. Returns unmount success to the CSI driver.
1. CSI driver returns unmount success to kubelet.

## Replica deletion

1. Volume manager receives SIGTERM.
1. It checks the list of replicas on the PV to see if it can delete the local volume.
    - If the PV doesn't exist.
    - If at least `volumeReplicas` healthy replicas exist. "Healthy" means they have a snapshot at least as new as ours.
1. It removes itself from the PV's replica list.
    1. It removes its entry.
    1. It re-reads the list of entries.
    1. It verifies that there are still sufficient replicas.
1. It deletes the local volume.
