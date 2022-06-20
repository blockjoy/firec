//! VMM configuration.

use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use derivative::Derivative;
use serde::{Deserialize, Serialize};

mod drive;
mod jailer;
mod machine;
/// Network configuration.
pub mod network;

pub use drive::*;
pub use jailer::*;
pub use machine::*;
use uuid::Uuid;

use crate::Error;

// FIXME: Hardcoding for now. This should come from ChrootStrategy enum, when we've that.
const KERNEL_IMAGE_FILENAME: &str = "kernel";

/// VMM configuration.
#[derive(Debug)]
pub struct Config<'c> {
    pub(crate) socket_path: Cow<'c, Path>,
    log_path: Option<Cow<'c, Path>>,
    log_fifo: Option<Cow<'c, Path>>,
    log_level: Option<LogLevel>,
    metrics_path: Option<Cow<'c, Path>>,
    metrics_fifo: Option<Cow<'c, Path>>,
    pub(crate) src_kernel_image_path: Cow<'c, Path>,
    pub(crate) src_initrd_path: Option<Cow<'c, Path>>,
    kernel_args: Option<Cow<'c, str>>,
    pub(crate) drives: Vec<Drive<'c>>,

    // FIXME: Can't use trait object here because it's make `Config` non-Send, which is problematic
    // for async/await.
    //// Used to redirect the contents of the fifo log to the writer.
    //#[derivative(Debug = "ignore")]
    //pub fifo_log_writer: Option<Box<dyn AsyncWrite>>,
    machine_cfg: Machine<'c>,
    pub(crate) jailer_cfg: Option<Jailer<'c>>,
    vm_id: Uuid,
    net_ns: Option<Cow<'c, str>>,
    network_interfaces: Vec<network::Interface<'c>>,
    /* TODO:


          Other fields.

    */
}

impl<'c> Config<'c> {
    /// Create a new `Builder` instance.
    ///
    /// # Arguments
    ///
    /// `vm_id` - The ID of the VM. It's used as the Firecracker's instance ID. Pass `None` to
    ///           generate a random ID.
    /// `src_kernel_image_path`: The path to the kernel image, that must be an uncompressed ELF image.
    pub fn builder<P>(vm_id: Option<Uuid>, src_kernel_image_path: P) -> Builder<'c>
    where
        P: Into<Cow<'c, Path>>,
    {
        Builder(Self {
            socket_path: Path::new("/run/firecracker.socket").into(),
            log_path: None,
            log_fifo: None,
            log_level: None,
            metrics_path: None,
            metrics_fifo: None,
            src_kernel_image_path: src_kernel_image_path.into(),
            src_initrd_path: None,
            kernel_args: None,
            drives: Vec::new(),
            machine_cfg: Machine::default(),
            jailer_cfg: None,
            vm_id: vm_id.unwrap_or_else(Uuid::new_v4),
            net_ns: None,
            network_interfaces: Vec::new(),
        })
    }

    /// Create boot source from `self`.
    pub(crate) fn boot_source(&self) -> Result<BootSource, Error> {
        let relative_kernel_image_path = Path::new("/").join(KERNEL_IMAGE_FILENAME);

        let relative_initrd_path: Result<Option<PathBuf>, Error> =
            match self.src_initrd_path.as_ref() {
                Some(initrd_path) => {
                    let initrd_filename =
                        initrd_path.file_name().ok_or(Error::InvalidInitrdPath)?;
                    Ok(Some(Path::new("/").join(initrd_filename)))
                }
                None => Ok(None),
            };

        Ok(BootSource {
            kernel_image_path: relative_kernel_image_path,
            initrd_path: relative_initrd_path?,
            boot_args: self.kernel_args.as_ref().map(AsRef::as_ref).map(Into::into),
        })
    }

    /// The socket path.
    pub fn socket_path(&self) -> &Path {
        self.socket_path.as_ref()
    }

    /// The socket path in chroot location.
    pub fn host_socket_path(&self) -> PathBuf {
        let socket_path = self.socket_path.as_ref();
        let relative_path = socket_path.strip_prefix("/").unwrap_or(socket_path);
        self.jailer().workspace_dir().join(relative_path)
    }

    /// The log path.
    pub fn log_path(&self) -> Option<&Path> {
        self.log_path.as_ref().map(AsRef::as_ref)
    }

    /// The log fifo path.
    pub fn log_fifo(&self) -> Option<&Path> {
        self.log_fifo.as_ref().map(AsRef::as_ref)
    }

    /// The metrics path.
    pub fn metrics_path(&self) -> Option<&Path> {
        self.metrics_path.as_ref().map(AsRef::as_ref)
    }

    /// The metrics fifo path.
    pub fn metrics_fifo(&self) -> Option<&Path> {
        self.metrics_fifo.as_ref().map(AsRef::as_ref)
    }

    /// The source kernel image path.
    ///
    /// This is the path given by the application. It's transfered to the chroot directory by
    /// [`crate::Machine::create`]. The path inside the chroot can be queried using
    /// [`Config::kernel_image_path`].
    pub fn src_kernel_image_path(&self) -> &Path {
        self.src_kernel_image_path.as_ref()
    }

    /// The kernel image path in chroot location.
    pub fn kernel_image_path(&self) -> PathBuf {
        self.jailer().workspace_dir().join(KERNEL_IMAGE_FILENAME)
    }

    /// The source initrd path.
    ///
    /// This is the path given by the application. It's transfered to the chroot directory by
    /// [`crate::Machine::create`]. The path inside the chroot can be queried using
    /// [`Config::initrd_image_path`].
    pub fn src_initrd_path(&self) -> Option<&Path> {
        self.src_initrd_path.as_ref().map(AsRef::as_ref)
    }

    /// The initrd path in chroot location.
    pub fn initrd_path(&self) -> Result<Option<PathBuf>, Error> {
        match self.src_initrd_path.as_ref() {
            Some(initrd_path) => {
                let initrd_filename = initrd_path
                    .file_name()
                    .ok_or(Error::InvalidInitrdPath)?
                    .to_owned();
                Ok(Some(self.jailer().workspace_dir().join(&initrd_filename)))
            }
            None => Ok(None),
        }
    }

    /// The kernel arguments.
    pub fn kernel_args(&self) -> Option<&str> {
        self.kernel_args.as_ref().map(AsRef::as_ref)
    }

    /// The drives.
    pub fn drives(&self) -> &[Drive<'c>] {
        &self.drives
    }

    /// The machine configuration.
    pub fn machine_cfg(&self) -> &Machine<'c> {
        &self.machine_cfg
    }

    /// The jailer configuration.
    pub fn jailer_cfg(&self) -> Option<&Jailer<'c>> {
        self.jailer_cfg.as_ref()
    }

    /// The VM ID.
    pub fn vm_id(&self) -> &Uuid {
        &self.vm_id
    }

    /// The network namespace path.
    pub fn net_ns(&self) -> Option<&str> {
        self.net_ns.as_ref().map(AsRef::as_ref)
    }

    /// The network interfaces.
    pub fn network_interfaces(&self) -> &[network::Interface<'c>] {
        &self.network_interfaces
    }

    pub(crate) fn jailer(&self) -> &Jailer {
        // FIXME: Assuming jailer for now.
        self.jailer_cfg.as_ref().expect("no jailer config")
    }
}

/// The boot source for the microVM.
#[derive(Debug, Serialize, Deserialize)]
pub struct BootSource<'b> {
    /// The kernel image path.
    pub kernel_image_path: PathBuf,
    /// The (optional) kernel command line.
    pub boot_args: Option<&'b str>,
    /// The (optional) initrd image path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initrd_path: Option<PathBuf>,
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

/// Configuration builder.
#[derive(Debug)]
pub struct Builder<'c>(Config<'c>);

impl<'c> Builder<'c> {
    /// Set the file path where the Firecracker control socket should be created.
    pub fn socket_path<P>(mut self, socket_path: P) -> Self
    where
        P: Into<Cow<'c, Path>>,
    {
        self.0.socket_path = socket_path.into();
        self
    }

    /// Set the Firecracker log path.
    pub fn log_path<P>(mut self, log_path: P) -> Self
    where
        P: Into<Cow<'c, Path>>,
    {
        self.0.log_path = Some(log_path.into());
        self
    }

    /// Set the Firecracker log named-pipe path.
    pub fn log_fifo<P>(mut self, log_fifo: P) -> Self
    where
        P: Into<Cow<'c, Path>>,
    {
        self.0.log_fifo = Some(log_fifo.into());
        self
    }

    /// Set the verbosity of Firecracker logging.
    pub fn log_level(mut self, log_level: LogLevel) -> Self {
        self.0.log_level = Some(log_level);
        self
    }

    /// Set the Firecracker metrics path.
    pub fn metrics_path<P>(mut self, metrics_path: P) -> Self
    where
        P: Into<Cow<'c, Path>>,
    {
        self.0.metrics_path = Some(metrics_path.into());
        self
    }

    /// Set the Firecracker metrics named-pipe path.
    pub fn metrics_fifo<P>(mut self, metrics_fifo: P) -> Self
    where
        P: Into<Cow<'c, Path>>,
    {
        self.0.metrics_fifo = Some(metrics_fifo.into());
        self
    }

    /// Set the initrd image path.
    pub fn initrd_path<P>(mut self, initrd_path: P) -> Self
    where
        P: Into<Cow<'c, Path>>,
    {
        self.0.src_initrd_path = Some(initrd_path.into());
        self
    }

    /// Set the command-line arguments that should be passed to the kernel.
    pub fn kernel_args<P>(mut self, kernel_args: P) -> Self
    where
        P: Into<Cow<'c, str>>,
    {
        self.0.kernel_args = Some(kernel_args.into());
        self
    }

    /// Add a drive.
    pub fn add_drive<I, P>(self, drive_id: I, src_path: P) -> DriveBuilder<'c>
    where
        I: Into<Cow<'c, str>>,
        P: Into<Cow<'c, Path>>,
    {
        DriveBuilder::new(self, drive_id, src_path)
    }

    /// Set the Firecracker microVM process configuration builder.
    pub fn machine_cfg(self) -> MachineBuilder<'c> {
        MachineBuilder::new(self)
    }

    /// Create the jailer process configuration builder.
    pub fn jailer_cfg(self) -> JailerBuilder<'c> {
        JailerBuilder::new(self)
    }

    /// Set the path to a network namespace handle.
    ///
    /// If specified, the application will use this to join the associated network namespace.
    pub fn net_ns<N>(mut self, net_ns: N) -> Self
    where
        N: Into<Cow<'c, str>>,
    {
        self.0.net_ns = Some(net_ns.into());
        self
    }

    /// Add a network interface.
    ///
    /// Add a tap device that should be made available to the microVM.
    pub fn add_network_interface(mut self, network_interface: network::Interface<'c>) -> Self {
        self.0.network_interfaces.push(network_interface);
        self
    }

    /// Build the configuration.
    pub fn build(self) -> Config<'c> {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Uuid;

    #[test]
    fn config_host_values() {
        let id = Uuid::new_v4();

        let config = Config::builder(Some(id), Path::new("/tmp/kernel.path"))
            .jailer_cfg()
            .chroot_base_dir(Path::new("/chroot"))
            .exec_file(Path::new("/usr/bin/firecracker"))
            .mode(JailerMode::Daemon)
            .build()
            .initrd_path(Path::new("/tmp/initrd.img"))
            .add_drive("root", Path::new("/tmp/debian.ext4"))
            .is_root_device(true)
            .build()
            .socket_path(Path::new("/firecracker.socket"))
            .build();

        assert_eq!(
            config.src_initrd_path.as_ref().unwrap().as_os_str(),
            "/tmp/initrd.img"
        );
        assert_eq!(
            config
                .initrd_path()
                .unwrap()
                .unwrap()
                .as_os_str()
                .to_string_lossy(),
            format!("/chroot/firecracker/{}/root/initrd.img", id)
        );

        assert_eq!(
            config.src_kernel_image_path.as_ref().as_os_str(),
            "/tmp/kernel.path"
        );
        assert_eq!(
            config.kernel_image_path().as_os_str().to_string_lossy(),
            format!("/chroot/firecracker/{}/root/kernel", id)
        );
        assert_eq!(
            config.socket_path.as_ref().as_os_str(),
            "/firecracker.socket"
        );
        assert_eq!(
            config.host_socket_path().as_os_str().to_string_lossy(),
            format!("/chroot/firecracker/{}/root/firecracker.socket", id)
        );

        let boot_source = config.boot_source().unwrap();
        assert_eq!(boot_source.boot_args, None);
        assert_eq!(boot_source.kernel_image_path.as_os_str(), "/kernel");
        assert_eq!(boot_source.initrd_path.unwrap().as_os_str(), "/initrd.img");
    }
}
