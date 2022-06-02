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
    pub(crate) jailer_workspace_dir: Cow<'c, Path>,
    pub(crate) kernel_image_path: Cow<'c, Path>,
    pub(crate) initrd_path: Option<Cow<'c, Path>>,
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
    /// `kernel_image_path: The path to the kernel image, that must be an uncompressed ELF image.
    pub fn builder<P>(kernel_image_path: P) -> Builder<'c>
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
            jailer_workspace_dir: Path::new("/srv/jailer/firecracker/root").into(),
            kernel_image_path: kernel_image_path.into(),
            initrd_path: None,
            kernel_args: None,
            drives: Vec::new(),
            machine_cfg: Machine::builder().build(),
            jailer_cfg: None,
            vm_id: Uuid::new_v4(),
            net_ns: None,
            network_interfaces: Vec::new(),
        })
    }

    /// Create boot source from `self`.
    pub fn boot_source(&self) -> Result<BootSource<'_>, Error> {
        let initrd_filename = match &self.initrd_path {
            Some(initrd_path) => {
                let initrd_filename = initrd_path.file_name().ok_or(Error::InvalidInitrdPath)?;
                Some(Path::new(initrd_filename))
            }
            None => None,
        };

        let kernel_image_file = Path::new(KERNEL_IMAGE_FILENAME);

        Ok(BootSource {
            kernel_image_path: kernel_image_file,
            initrd_path: initrd_filename,
            boot_args: self.kernel_args.as_ref().map(AsRef::as_ref),
        })
    }

    /// The socket path.
    pub fn socket_path(&self) -> &Path {
        self.socket_path.as_ref()
    }

    /// The socket path in chroot location.
    pub fn guest_socket_path(&self) -> PathBuf {
        let socket_path = self.socket_path.as_ref();
        let relative_path = socket_path.strip_prefix("/").unwrap_or(socket_path);
        self.jailer_workspace_dir.join(relative_path)
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

    /// The kernel image path.
    pub fn kernel_image_path(&self) -> &Path {
        self.kernel_image_path.as_ref()
    }

    /// The kernel image path in chroot location.
    pub fn guest_kernel_image_path(&self) -> PathBuf {
        self.jailer_workspace_dir.join(KERNEL_IMAGE_FILENAME)
    }

    /// The initrd path.
    pub fn initrd_path(&self) -> Option<&Path> {
        self.initrd_path.as_ref().map(AsRef::as_ref)
    }

    /// The initrd path in chroot location.
    pub fn guest_initrd_path(&self) -> Result<Option<PathBuf>, Error> {
        match self.initrd_path.as_ref() {
            Some(initrd_path) => {
                let initrd_filename = initrd_path
                    .file_name()
                    .ok_or(Error::InvalidInitrdPath)?
                    .to_owned();
                Ok(Some(self.jailer_workspace_dir.join(&initrd_filename)))
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
    pub fn log_path<P>(mut self, log_path: Option<P>) -> Self
    where
        P: Into<Cow<'c, Path>>,
    {
        self.0.log_path = log_path.map(Into::into);
        self
    }

    /// Set the Firecracker log named-pipe path.
    pub fn log_fifo<P>(mut self, log_fifo: Option<P>) -> Self
    where
        P: Into<Cow<'c, Path>>,
    {
        self.0.log_fifo = log_fifo.map(Into::into);
        self
    }

    /// Set the verbosity of Firecracker logging.
    pub fn log_level(mut self, log_level: Option<LogLevel>) -> Self {
        self.0.log_level = log_level;
        self
    }

    /// Set the Firecracker metrics path.
    pub fn metrics_path<P>(mut self, metrics_path: Option<P>) -> Self
    where
        P: Into<Cow<'c, Path>>,
    {
        self.0.metrics_path = metrics_path.map(Into::into);
        self
    }

    /// Set the Firecracker metrics named-pipe path.
    pub fn metrics_fifo<P>(mut self, metrics_fifo: Option<P>) -> Self
    where
        P: Into<Cow<'c, Path>>,
    {
        self.0.metrics_fifo = metrics_fifo.map(Into::into);
        self
    }

    /// Set the initrd image path.
    pub fn initrd_path<P>(mut self, initrd_path: Option<P>) -> Self
    where
        P: Into<Cow<'c, Path>>,
    {
        self.0.initrd_path = initrd_path.map(Into::into);
        self
    }

    /// Set the command-line arguments that should be passed to the kernel.
    pub fn kernel_args<P>(mut self, kernel_args: Option<P>) -> Self
    where
        P: Into<Cow<'c, str>>,
    {
        self.0.kernel_args = kernel_args.map(Into::into);
        self
    }

    /// Add a drive.
    pub fn add_drive<D>(mut self, drive: D) -> Self
    where
        D: Into<Drive<'c>>,
    {
        self.0.drives.push(drive.into());
        self
    }

    /// Set the Firecracker microVM process configuration.
    pub fn machine_cfg(mut self, machine_cfg: Machine<'c>) -> Self {
        self.0.machine_cfg = machine_cfg;
        self
    }

    /// Set the jailer process configuration.
    pub fn jailer_cfg(mut self, jailer_cfg: Option<Jailer<'c>>) -> Self {
        self.0.jailer_cfg = jailer_cfg;
        self
    }

    /// Set a unique identifier for this VM.
    ///
    /// It's set to a random uuid if not provided by the user. It's used as the Firecracker's
    /// instance ID.
    pub fn vm_id(mut self, vm_id: Uuid) -> Self {
        self.0.vm_id = vm_id;
        self
    }

    /// Set the path to a network namespace handle.
    ///
    /// If specified, the application will use this to join the associated network namespace.
    pub fn net_ns<N>(mut self, net_ns: Option<N>) -> Self
    where
        N: Into<Cow<'c, str>>,
    {
        self.0.net_ns = net_ns.map(Into::into);
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
    pub fn build(mut self) -> Result<Config<'c>, Error> {
        // TODO: Validate other parts of config, e.g paths.

        // FIXME: Assuming jailer for now.
        let jailer = self.0.jailer_cfg.as_ref().expect("no jailer config");

        let exec_file_base = jailer
            .exec_file()
            .file_name()
            .ok_or(Error::InvalidJailerExecPath)?;
        let id_str = self.0.vm_id().to_string();
        self.0.jailer_workspace_dir = jailer
            .chroot_base_dir()
            .join(exec_file_base)
            .join(&id_str)
            .join("root")
            .into();

        Ok(self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Uuid;

    #[test]
    fn config_host_values() {
        let id = Uuid::new_v4();

        let jailer = Jailer::builder()
            .chroot_base_dir(Path::new("/chroot"))
            .exec_file(Path::new("/usr/bin/firecracker"))
            .mode(JailerMode::Daemon)
            .build();

        let root_drive = Drive::builder("root", Path::new("/tmp/debian.ext4"))
            .is_root_device(true)
            .build();

        let config = Config::builder(Path::new("/tmp/kernel.path"))
            .vm_id(id)
            .jailer_cfg(Some(jailer))
            .initrd_path(Some(Path::new("/tmp/initrd.img")))
            .add_drive(root_drive)
            .socket_path(Path::new("/firecracker.socket"))
            .build()
            .unwrap();

        assert_eq!(
            config.initrd_path.as_ref().unwrap().as_os_str(),
            "/tmp/initrd.img"
        );
        assert_eq!(
            config
                .guest_initrd_path()
                .unwrap()
                .unwrap()
                .as_os_str()
                .to_string_lossy(),
            format!("/chroot/firecracker/{}/root/initrd.img", id)
        );

        assert_eq!(
            config.kernel_image_path.as_ref().as_os_str(),
            "/tmp/kernel.path"
        );
        assert_eq!(
            config
                .guest_kernel_image_path()
                .as_os_str()
                .to_string_lossy(),
            format!("/chroot/firecracker/{}/root/kernel", id)
        );
        assert_eq!(
            config.socket_path.as_ref().as_os_str(),
            "/firecracker.socket"
        );
        assert_eq!(
            config.guest_socket_path().as_os_str().to_string_lossy(),
            format!("/chroot/firecracker/{}/root/firecracker.socket", id)
        );

        let boot_source = config.boot_source().unwrap();
        assert_eq!(boot_source.boot_args, None);
        assert_eq!(boot_source.kernel_image_path.as_os_str(), "kernel");
        assert_eq!(boot_source.initrd_path, Some(Path::new("initrd.img")));
    }
}
