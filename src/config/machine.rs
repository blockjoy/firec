use std::borrow::Cow;

use derivative::Derivative;
use serde::{Deserialize, Serialize};

/// Machine configuration.
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

impl<'m> Machine<'m> {
    /// Create a new `MachineBuilder` instance.
    pub fn builder() -> MachineBuilder<'m> {
        MachineBuilder(Machine {
            smt: false,
            track_dirty_pages: false,
            mem_size_mib: 0,
            vcpu_count: 1,
            cpu_template: None,
        })
    }
}

/// Builder for `Machine`.
#[derive(Debug)]
pub struct MachineBuilder<'m>(Machine<'m>);

impl<'m> MachineBuilder<'m> {
    /// Flag for enabling/disabling simultaneous multithreading.
    ///
    /// Can be enabled only on x86.
    pub fn smt(mut self, smt: bool) -> Self {
        self.0.smt = smt;
        self
    }

    /// Enable dirty page tracking. If this is enabled, then incremental guest memory snapshots
    /// can be created. These belong to diff snapshots, which contain, besides the microVM state,
    /// only the memory dirtied since a previous snapshot. Full snapshots each contain a full copy
    /// of the guest memory.
    pub fn track_dirty_pages(mut self, track_dirty_pages: bool) -> Self {
        self.0.track_dirty_pages = track_dirty_pages;
        self
    }

    /// Memory size of VM.
    pub fn mem_size_mib(mut self, mem_size_mib: i64) -> Self {
        self.0.mem_size_mib = mem_size_mib;
        self
    }

    /// Number of vCPUs (either 1 or an even number).
    ///
    /// Maximum: 32
    /// Minimum: 1
    pub fn vcpu_count(mut self, vcpu_count: usize) -> Self {
        self.0.vcpu_count = vcpu_count;
        self
    }

    /// cpu template.
    pub fn cpu_template(mut self, cpu_template: Cow<'m, str>) -> Self {
        self.0.cpu_template = Some(cpu_template);
        self
    }

    /// Build the `Machine` instance.
    pub fn build(self) -> Machine<'m> {
        self.0
    }
}
