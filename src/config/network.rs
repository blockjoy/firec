use std::borrow::Cow;

/// Network configuration.
#[derive(Debug)]
pub enum Interface<'i> {
    /// CNIConfiguration that will be used to generate the VM's network namespace,
    /// tap device and internal network for this network interface.
    Cni(Cni<'i>),
}

/// CNI network configuration.
// TODO: Provide a builder for `Cni`.
#[derive(Debug, Default)]
pub struct Cni<'c> {
    /// Corresponds to the "name" parameter in the CNI spec's
    /// Network Configuration List structure. It selects the name
    /// of the network whose configuration will be used when invoking CNI.
    pub network_name: Cow<'c, str>,

    /// IfName (optional) corresponds to the CNI_IFNAME parameter as specified
    /// in the CNI spec. It generally specifies the name of the interface to be
    /// created by a CNI plugin being invoked.
    ///
    /// Note that this does NOT necessarily correspond to the name of the
    /// tap device the Firecracker VM will use as the tap device may be
    /// created by a chained plugin that adapts the tap to a pre-existing
    /// network device (which will by the one with "IfName").
    pub if_name: Option<Cow<'c, str>>,

    /// Sets the interface name in the VM. It is used
    /// to correctly pass IP configuration obtained from the CNI to the VM kernel.
    /// It can be left blank for VMs with single network interface.
    pub vm_if_name: Option<Cow<'c, str>>,
    // TODO: Other fields: https://pkg.go.dev/github.com/firecracker-microvm/firecracker-go-sdk#CNIConfiguration
}
