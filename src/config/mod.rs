//! VMM configuration.

use derivative::Derivative;
use tokio::io::AsyncWrite;

mod drive;
mod jailer;
mod machine;

pub use drive::*;
pub use jailer::*;
pub use machine::*;

/// VMM configuration.
// TODO: Provide a builder for `Config`.
#[derive(Derivative, Default)]
#[derivative(Debug)]
pub struct Config<'c> {
    /// defines the file path where the Firecracker control socket
    /// should be created.
    pub socket_path: &'c str,

    /// defines the file path where the Firecracker log is located.
    pub log_path: Option<&'c str>,

    /// defines the file path where the Firecracker log named-pipe should
    /// be located.
    pub log_fifo: Option<&'c str>,

    /// defines the verbosity of Firecracker logging.  Valid values are
    /// "Error", "Warning", "Info", and "Debug", and are case-sensitive.
    pub log_level: Option<LogLevel>,

    /// defines the file path where the Firecracker metrics is located.
    pub metrics_path: &'c str,

    /// defines the file path where the Firecracker metrics
    /// named-pipe should be located.
    pub metrics_fifo: &'c str,

    /// defines the file path where the kernel image is located.
    /// The kernel image must be an uncompressed ELF image.
    pub kernel_image_path: &'c str,

    /// defines the file path where initrd image is located.
    ///
    /// This parameter is optional.
    pub initrd_path: Option<&'c str>,

    /// defines the command-line arguments that should be passed to
    /// the kernel.
    pub kernel_args: Option<&'c str>,

    /// specifies BlockDevices that should be made available to the
    /// microVM.
    pub drives: &'c [Drive<'c>],

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
    pub vm_id: Option<&'c str>,

    /// represents the path to a network namespace handle. If present, the
    /// application will use this to join the associated network namespace
    pub net_ns: Option<&'c str>,
    /* TODO:

         /// specifies the tap devices that should be made available
         /// to the microVM.
        pub network_interfaces: &'c [NetworkInterface],

          Other fields.

    */
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
