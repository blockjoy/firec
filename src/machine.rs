//! A VMM machine.

use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use crate::{
    config::{network, Config, JailerMode},
    Error,
};
use serde::Serialize;
use serde_json::json;
use tokio::{
    fs::{copy, DirBuilder},
    process::{Child, Command},
    time::sleep,
};
use uuid::Uuid;

use hyper::{Body, Client, Method, Request};
use hyperlocal::{UnixClientExt, UnixConnector, Uri};

// FIXME: Hardcoding for now. This should come from ChrootStrategy enum, when we've that.
const KERNEL_IMAGE_FILENAME: &str = "kernel";

/// A VMM machine.
#[derive(Debug)]
pub struct Machine<'m> {
    config: Config<'m>,
    child: Child,
    client: Client<UnixConnector>,
}

impl<'m> Machine<'m> {
    /// Create a new machine.
    ///
    /// The machine is not started yet.
    pub async fn new(mut config: Config<'m>) -> Result<Machine<'m>, Error> {
        if config.vm_id == None {
            config.vm_id = Some(Uuid::new_v4());
        }

        // TOOD: Validate other parts of config, e.g paths.

        // FIXME: Assuming jailer for now.
        let jailer = config.jailer_cfg.as_mut().expect("no jailer config");
        let (daemonize_arg, stdin, stdout, stderr) = match &mut jailer.mode {
            JailerMode::Daemon => (
                Some("--daemonize"),
                Stdio::null(),
                Stdio::null(),
                Stdio::null(),
            ),
            JailerMode::Attached(stdio) => (
                None,
                stdio.stdin.take().unwrap_or_else(Stdio::inherit),
                stdio.stdout.take().unwrap_or_else(Stdio::inherit),
                stdio.stderr.take().unwrap_or_else(Stdio::inherit),
            ),
        };

        // Assemble the path to the jailed root folder on the host.
        let exec_file_base = jailer
            .exec_file
            .file_name()
            .ok_or(Error::InvalidJailerExecPath)?;
        let id_str = jailer.id.to_string();
        let rootfs = jailer
            .chroot_base_dir
            .join(exec_file_base)
            .join(&id_str)
            .join("root");
        DirBuilder::new().recursive(true).create(&rootfs).await?;

        // Copy the kernel image to the rootfs.
        copy(config.kernel_image_path, rootfs.join(KERNEL_IMAGE_FILENAME)).await?;
        // Now the initrd, if specified.
        config.initrd_path = match config.initrd_path {
            Some(initrd_path) => {
                let initrd_filename = initrd_path
                    .file_name()
                    .ok_or(Error::InvalidInitrdPath)?
                    .to_owned();
                copy(initrd_path.as_os_str(), rootfs.join(&initrd_filename)).await?;

                Some(PathBuf::from(initrd_filename).into())
            }
            None => None,
        };

        // Copy all drives to the rootfs.
        for drive in &mut config.drives {
            let drive_filename = drive
                .path_on_host
                .file_name()
                .ok_or(Error::InvalidDrivePath)?;
            copy(&drive.path_on_host, rootfs.join(drive_filename)).await?;

            drive.path_on_host = PathBuf::from(drive_filename).into();
        }

        config.kernel_image_path = Path::new(KERNEL_IMAGE_FILENAME).into();

        // Adjust socket file path.
        let socket_path = config.socket_path;
        config.socket_path = rootfs.join(&socket_path).into();

        // TODO: Handle fifos. See https://github.com/firecracker-microvm/firecracker-go-sdk/blob/f0a967ef386caec37f6533dce5797038edf8c226/jailer.go#L435

        let mut cmd = Command::new(jailer.jailer_binary.as_os_str());
        let mut cmd = cmd
            .args(&[
                "--id",
                &id_str,
                "--exec-file",
                jailer
                    .exec_file
                    .to_str()
                    .ok_or(Error::InvalidJailerExecPath)?,
                "--uid",
                &jailer.uid.to_string(),
                "--gid",
                &jailer.gid.to_string(),
                "--chroot-base-dir",
                jailer
                    .chroot_base_dir
                    .to_str()
                    .ok_or(Error::InvalidChrootBasePath)?,
                // `firecracker` binary args.
                "--",
                "--socket",
                socket_path.to_str().ok_or(Error::InvalidSocketPath)?,
            ])
            .stdin(stdin)
            .stdout(stdout)
            .stderr(stderr);
        if let Some(daemonize_arg) = daemonize_arg {
            cmd = cmd.arg(daemonize_arg);
        }
        let child = cmd.spawn()?;

        // Give some time to the jailer to start up and create the socket.
        // FIXME: We should monitor the socket instead?
        sleep(Duration::from_secs(1)).await;

        // `request` doesn't provide API to connect to unix sockets so we we use the low-level
        // approach using hyper: https://github.com/seanmonstar/reqwest/issues/39
        let client = Client::unix();

        let mut machine = Self {
            config,
            child,
            client,
        };

        machine.setup_resources().await?;
        machine.setup_boot_source().await?;
        machine.setup_drives().await?;
        machine.setup_network().await?;

        Ok(machine)
    }

    /// Start the machine.
    pub async fn start(&mut self) -> Result<(), Error> {
        // Start the machine.
        self.send_action(Action::InstanceStart).await?;

        Ok(())
    }

    /// Stop the machine.
    pub async fn stop(&mut self) -> Result<(), Error> {
        self.child.kill().await?;

        Ok(())
    }

    /// Shutdown requests a clean shutdown of the VM by sending CtrlAltDelete on the virtual keyboard.
    pub async fn shutdown(&mut self) -> Result<(), Error> {
        self.send_action(Action::SendCtrlAltDel).await
    }

    /// Get the configuration of the machine.
    pub fn config(&self) -> &Config<'m> {
        &self.config
    }

    async fn send_action(&mut self, action: Action) -> Result<(), Error> {
        let url: hyper::Uri = Uri::new(&self.config.socket_path, "/actions").into();
        let json = serde_json::to_string(&action)?;
        let request = Request::builder()
            .method(Method::PUT)
            .uri(url)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .body(Body::from(json))?;
        self.client.request(request).await?;

        Ok(())
    }

    async fn setup_resources(&mut self) -> Result<(), Error> {
        let json = serde_json::to_string(&self.config.machine_cfg)?;
        let url: hyper::Uri = Uri::new(&self.config.socket_path, "/machine-config").into();
        let request = Request::builder()
            .method(Method::PUT)
            .uri(url)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .body(Body::from(json))?;
        self.client.request(request).await?;

        Ok(())
    }

    async fn setup_boot_source(&mut self) -> Result<(), Error> {
        let boot_source = self.config.boot_source();
        let json = serde_json::to_string(&boot_source)?;
        let url: hyper::Uri = Uri::new(&self.config.socket_path, "/boot-source").into();
        let request = Request::builder()
            .method(Method::PUT)
            .uri(url)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .body(Body::from(json))?;
        self.client.request(request).await?;

        Ok(())
    }

    async fn setup_drives(&mut self) -> Result<(), Error> {
        for drive in &self.config.drives {
            let path = format!("/drive/{}", drive.drive_id);
            let url: hyper::Uri = Uri::new(&self.config.socket_path, &path).into();
            let json = serde_json::to_string(&drive)?;

            let request = Request::builder()
                .method(Method::PUT)
                .uri(url)
                .header("Accept", "application/json")
                .header("Content-Type", "application/json")
                .body(Body::from(json))?;
            self.client.request(request).await?;
        }

        Ok(())
    }

    async fn setup_network(&mut self) -> Result<(), Error> {
        // TODO: check for at least one interface.
        let network = &self.config.network_interfaces[0];
        let network::Interface::Cni(cni) = network;
        let iface_id = cni.vm_if_name.as_ref().unwrap_or(&cni.network_name);
        let json = json!({
            "iface_id": iface_id,
            "host_dev_name": cni.if_name,
        });
        let json = serde_json::to_string(&json)?;
        let path = format!("/network-interfaces/{}", iface_id);
        let url: hyper::Uri = Uri::new(&self.config.socket_path, &path).into();
        let request = Request::builder()
            .method(Method::PUT)
            .uri(url)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .body(Body::from(json))?;
        self.client.request(request).await?;

        Ok(())
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "action_type", rename_all = "PascalCase")]
enum Action {
    InstanceStart,
    SendCtrlAltDel,
    #[allow(unused)]
    FlushMetrics,
}
