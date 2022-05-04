use derivative::Derivative;
use serde::{Deserialize, Serialize};

/// Machine configuration.
// TODO: Provide a builder for `Machine`.
#[derive(Derivative, Serialize, Deserialize, Default)]
#[derivative(Debug)]
pub struct Machine<'m> {
    /// Flag for enabling/disabling Hyperthreading
    ht_ennabled: bool,

    /// Memory size of VM
    mem_size_mib: i64,

    /// Number of vCPUs (either 1 or an even number)
    ///
    /// Maximum: 32
    /// Minimum: 1
    vcpu_count: i64,

    /// cpu template
    // TODO: Should create a type to validate it like the Go API.
    cpu_template: Option<&'m str>,
}
