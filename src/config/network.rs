use std::borrow::Cow;

/// Network configuration.
#[derive(Debug)]
pub struct Interface<'i> {
    /// The name of the host interface.
    pub host_if_name: Cow<'i, str>,

    /// The interface name in the VM.
    pub vm_if_name: Cow<'i, str>,
}
