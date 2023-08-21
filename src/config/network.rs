use std::borrow::Cow;

use serde::{Deserialize, Serialize};

/// Network configuration.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Interface<'i> {
    #[serde(rename = "host_dev_name")]
    host_if_name: Cow<'i, str>,
    #[serde(rename = "iface_id")]
    vm_if_name: Cow<'i, str>,
    #[serde(rename = "guest_mac", skip_serializing_if = "Option::is_none")]
    vm_mac_address: Option<Cow<'i, str>>,
}

impl<'i> Interface<'i> {
    /// Create a new `Interface` instance.
    pub fn new<H, V, M>(host_if_name: H, vm_if_name: V, vm_mac_address: Option<M>) -> Self
    where
        H: Into<Cow<'i, str>>,
        V: Into<Cow<'i, str>>,
        M: Into<Cow<'i, str>>,
    {
        Interface {
            host_if_name: host_if_name.into(),
            vm_if_name: vm_if_name.into(),
            vm_mac_address: vm_mac_address.map(Into::into),
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

    /// MAC address of the VM.
    pub fn vm_mac_address(&self) -> Option<&str> {
        self.vm_mac_address.as_deref()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore]
    fn string_generics() {
        // Compile-only test to ensure the generics work for both string types.
        let _ = super::Interface::new("host_if_name", "vm_if_name", Some("AA:FC:00:00:00:01"));
        // Different types are fine, as long as they've the same lifetime.
        let _ = super::Interface::new("host_if_name".to_string(), "vm_if_name", None::<String>);
    }
}
