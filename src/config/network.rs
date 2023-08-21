use std::borrow::Cow;

use serde::{Deserialize, Serialize};

/// Network configuration.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Interface<'i> {
    #[serde(rename = "host_dev_name")]
    host_if_name: Cow<'i, str>,
    #[serde(rename = "iface_id")]
    vm_if_name: Cow<'i, str>,
}

impl<'i> Interface<'i> {
    /// Create a new `Interface` instance.
    pub fn new<H, V>(host_if_name: H, vm_if_name: V) -> Self
    where
        H: Into<Cow<'i, str>>,
        V: Into<Cow<'i, str>>,
    {
        Interface {
            host_if_name: host_if_name.into(),
            vm_if_name: vm_if_name.into(),
        }
    }

    /// The name of the host interface.
    pub fn host_if_name(&self) -> &str {
        &self.host_if_name
    }

    /// The interface name in the VM.
    pub fn vm_if_name(&self) -> &str {
        &self.vm_if_name
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore]
    fn string_generics() {
        // Compile-only test to ensure the generics work for both string types.
        let _ = super::Interface::new("host_if_name", "vm_if_name");
        // Different types are fine, as long as they've the same lifetime.
        let _ = super::Interface::new("host_if_name".to_string(), "vm_if_name");
    }
}
