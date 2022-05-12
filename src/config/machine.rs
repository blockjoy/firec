use std::borrow::Cow;

use derivative::Derivative;
use serde::{Deserialize, Serialize};

/// Machine configuration.
// TODO: Provide a builder for `Machine`.
#[derive(Derivative, Debug, Serialize, Deserialize, Default)]
pub struct Machine<'m> {
    /// Flag for enabling/disabling simultaneous multithreading. Can be enabled only on x86.
    pub smt: bool,

    /// Enable dirty page tracking. If this is enabled, then incremental guest memory snapshots
    /// can be created. These belong to diff snapshots, which contain, besides the microVM state,
    /// only the memory dirtied since a previous snapshot. Full snapshots each contain a full copy
    /// of the guest memory.
    pub track_dirty_pages: bool,

    /// Memory size of VM
    pub mem_size_mib: i64,

    /// Number of vCPUs (either 1 or an even number)
    ///
    /// Maximum: 32
    /// Minimum: 1
    #[derivative(Default(value = "1"))]
    pub vcpu_count: usize,

    /// cpu template
    // TODO: Should create a type to validate it like the Go API.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_template: Option<Cow<'m, str>>,
}
