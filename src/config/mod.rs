//! VMM configuration.

use std::{borrow::Cow, path::Path};

use derivative::Derivative;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWrite;

mod drive;
mod jailer;
mod machine;

pub use drive::*;
pub use jailer::*;
pub use machine::*;
use uuid::Uuid;

/// VMM configuration.
// TODO: Provide a builder for `Config`.
#[derive(Derivative)]
#[derivative(Debug, Default)]
pub struct Config<'c> {
    /// defines the file path where the Firecracker control socket
    /// should be created.
    #[derivative(Default(value = "Path::new(\"/run/firecracker.socket\").into()"))]
    pub socket_path: Cow<'c, Path>,

    /// defines the file path where the Firecracker log is located.
    pub log_path: Option<Cow<'c, Path>>,

    /// defines the file path where the Firecracker log named-pipe should
    /// be located.
    pub log_fifo: Option<Cow<'c, Path>>,

    /// defines the verbosity of Firecracker logging.  Valid values are
    /// "Error", "Warning", "Info", and "Debug", and are case-sensitive.
    pub log_level: Option<LogLevel>,

    /// defines the file path where the Firecracker metrics is located.
    pub metrics_path: Cow<'c, Path>,

    /// defines the file path where the Firecracker metrics
    /// named-pipe should be located.
    pub metrics_fifo: Cow<'c, Path>,

    /// defines the file path where the kernel image is located.
    /// The kernel image must be an uncompressed ELF image.
    pub kernel_image_path: Cow<'c, Path>,

    /// defines the file path where initrd image is located.
    ///
    /// This parameter is optional.
    pub initrd_path: Option<Cow<'c, Path>>,

    /// defines the command-line arguments that should be passed to
    /// the kernel.
    pub kernel_args: Option<Cow<'c, str>>,

    /// specifies BlockDevices that should be made available to the
    /// microVM.
    pub drives: Vec<Drive<'c>>,

    /// Used to redirect the contents of the fifo log to the writer.
    #[derivative(Debug = "ignore")]
    pub fifo_log_writer: Option<Box<dyn AsyncWrite>>,

    /// The firecracker microVM process configuration
    pub machine_cfg: Machine<'c>,

    /// JailerCfg is configuration specific for the jailer process.
    pub jailer_cfg: Option<Jailer<'c>>,

    /// a unique identifier for this VM. It's set to a
    /// random uuid if not provided by the user. It's used to set Firecracker's instance ID.
    /// If CNI configuration is provided as part of NetworkInterfaces,
    /// the VMID is used to set CNI ContainerID and create a network namespace path.
    pub vm_id: Option<Uuid>,

    /// represents the path to a network namespace handle. If present, the
    /// application will use this to join the associated network namespace
    pub net_ns: Option<Cow<'c, str>>,
    /* TODO:

         /// specifies the tap devices that should be made available
         /// to the microVM.
        pub network_interfaces: &'c [NetworkInterface],

          Other fields.

    */
}

impl<'c> Config<'c> {
    /// Create boot source from `self`.
    pub fn boot_source(&self) -> BootSource<'_> {
        BootSource {
            kernel_image_path: self.kernel_image_path.as_ref(),
            initrd_path: self.initrd_path.as_ref().map(AsRef::as_ref),
            boot_args: self.kernel_args.as_ref().map(AsRef::as_ref),
        }
    }
}

/// The boot source for the microVM.
#[derive(Debug, Serialize, Deserialize)]
pub struct BootSource<'b> {
    /// The kernel image path.
    #[serde(borrow)]
    pub kernel_image_path: &'b Path,
    /// The (optional) kernel command line.
    pub boot_args: Option<&'b str>,
    /// The (optional) initrd image path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initrd_path: Option<&'b Path>,
}

/// defines the verbosity of Firecracker logging.
#[derive(Derivative)]
#[derivative(Debug, Default)]
pub enum LogLevel {
    /// Error level logging.
    Error,
    /// Warning level logging.
    Warning,
    #[derivative(Default)]
    /// Info level logging.
    Info,
    /// Debug level logging.
    Debug,
}
