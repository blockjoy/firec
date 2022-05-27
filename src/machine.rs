//! A VMM machine.

use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use crate::{
    config::{Config, JailerMode},
    Error,
};
use serde::Serialize;
use serde_json::json;
use sysinfo::{Pid, ProcessExt, ProcessRefreshKind, System, SystemExt};
use tokio::{
    fs::{copy, DirBuilder},
    process::Command,
    task,
    time::sleep,
};
use tracing::{info, instrument, trace};

use hyper::{Body, Client, Method, Request};
use hyperlocal::{UnixClientExt, UnixConnector, Uri};

// FIXME: Hardcoding for now. This should come from ChrootStrategy enum, when we've that.
const KERNEL_IMAGE_FILENAME: &str = "kernel";

/// A VMM machine.
#[derive(Debug)]
pub struct Machine<'m> {
    config: Config<'m>,
    pid: i32,
    client: Client<UnixConnector>,
}

impl<'m> Machine<'m> {
    /// Create a new machine.
    ///
    /// The machine is not started yet.
    #[instrument]
    pub async fn create(mut config: Config<'m>) -> Result<Machine<'m>, Error> {
        let vm_id = *config.vm_id();
        info!("Creating new machine with VM ID `{vm_id}`");
        trace!("{vm_id}: Configuration: {:?}", config);

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
            .exec_file()
            .file_name()
            .ok_or(Error::InvalidJailerExecPath)?;
        let id_str = jailer.id().to_string();
        let jailer_workspace_dir = jailer
            .chroot_base_dir()
            .join(exec_file_base)
            .join(&id_str)
            .join("root");
        info!(
            "{vm_id}: Jailer workspace dir: {}",
            jailer_workspace_dir.display()
        );
        DirBuilder::new()
            .recursive(true)
            .create(&jailer_workspace_dir)
            .await?;

        // Copy the kernel image to the rootfs.
        let dest = jailer_workspace_dir.join(KERNEL_IMAGE_FILENAME);
        trace!(
            "{vm_id}: Copying kernel image from `{}` to `{}`",
            config.kernel_image_path.display(),
            dest.display()
        );
        copy(config.kernel_image_path, dest).await?;
        // Now the initrd, if specified.
        config.initrd_path = match config.initrd_path {
            Some(initrd_path) => {
                let initrd_filename = initrd_path
                    .file_name()
                    .ok_or(Error::InvalidInitrdPath)?
                    .to_owned();
                let dest = jailer_workspace_dir.join(&initrd_filename);
                trace!(
                    "{vm_id}: Copying initrd from `{}` to `{}`",
                    initrd_path.display(),
                    dest.display()
                );
                copy(initrd_path.as_os_str(), dest).await?;

                Some(PathBuf::from(initrd_filename).into())
            }
            None => None,
        };

        // Copy all drives to the rootfs.
        for drive in &mut config.drives {
            let drive_filename = drive
                .path_on_host()
                .file_name()
                .ok_or(Error::InvalidDrivePath)?;
            let dest = jailer_workspace_dir.join(drive_filename);
            trace!(
                "{vm_id}: Copying drive `{}` from `{}` to `{}`",
                drive.drive_id(),
                drive.path_on_host().display(),
                dest.display()
            );
            copy(&drive.path_on_host(), dest).await?;

            drive.path_on_host = PathBuf::from(drive_filename).into();
        }

        config.kernel_image_path = Path::new(KERNEL_IMAGE_FILENAME).into();

        // Adjust socket file path.
        let socket_path = config.socket_path;
        let relative_path = socket_path.strip_prefix("/").unwrap_or(&socket_path);
        config.socket_path = jailer_workspace_dir.join(relative_path).into();
        info!(
            "{vm_id}: Host socket path: `{}`",
            config.socket_path.display()
        );
        info!("{vm_id}: Guest socket path: `{}`", socket_path.display());
        if let Some(socket_dir) = config.socket_path.parent() {
            trace!(
                "{vm_id}: Ensuring socket directory exist at `{}`",
                socket_dir.display()
            );
            DirBuilder::new().recursive(true).create(socket_dir).await?;
        }

        // TODO: Handle fifos. See https://github.com/firecracker-microvm/firecracker-go-sdk/blob/f0a967ef386caec37f6533dce5797038edf8c226/jailer.go#L435

        let mut cmd = &mut Command::new(jailer.jailer_binary().as_os_str());
        if let Some(daemonize_arg) = daemonize_arg {
            cmd = cmd.arg(daemonize_arg);
        }
        let cmd = cmd
            .args(&[
                "--id",
                &id_str,
                "--exec-file",
                jailer
                    .exec_file()
                    .to_str()
                    .ok_or(Error::InvalidJailerExecPath)?,
                "--uid",
                &jailer.uid().to_string(),
                "--gid",
                &jailer.gid().to_string(),
                "--chroot-base-dir",
                jailer
                    .chroot_base_dir()
                    .to_str()
                    .ok_or(Error::InvalidChrootBasePath)?,
                // `firecracker` binary args.
                "--",
                "--api-sock",
                socket_path.to_str().ok_or(Error::InvalidSocketPath)?,
            ])
            .stdin(stdin)
            .stdout(stdout)
            .stderr(stderr);
        trace!("{vm_id}: Running command: {:?}", cmd);
        let mut child = cmd.spawn()?;
        let pid = match child.id() {
            Some(id) => id.try_into()?,
            None => {
                let exit_status = child.wait().await?;
                return Err(Error::ProcessExitedImmediatelly { exit_status });
            }
        };

        // Give some time to the jailer to start up and create the socket.
        // FIXME: We should monitor the socket instead?
        info!("{vm_id}: Waiting for the jailer to start up...");
        sleep(Duration::from_secs(10)).await;

        // `request` doesn't provide API to connect to unix sockets so we we use the low-level
        // approach using hyper: https://github.com/seanmonstar/reqwest/issues/39
        let client = Client::unix();

        let machine = Self {
            config,
            pid,
            client,
        };

        info!("{vm_id}: Setting the VM...");
        machine.setup_resources().await?;
        machine.setup_boot_source().await?;
        machine.setup_drives().await?;
        machine.setup_network().await?;
        info!("{vm_id}: VM successfully setup.");

        Ok(machine)
    }

    /// Start the machine.
    #[instrument]
    pub async fn start(&self) -> Result<(), Error> {
        let vm_id = self.config.vm_id();
        trace!("{vm_id}: Starting the VM...");
        // Start the machine.
        self.send_action(Action::InstanceStart).await?;
        trace!("{vm_id}: VM started successfully.");

        Ok(())
    }

    /// Stop the machine.
    #[instrument]
    pub async fn stop(&self) -> Result<(), Error> {
        let vm_id = self.config.vm_id();
        trace!("{vm_id}: Killing VM...");

        let pid = self.pid;
        let killed = task::spawn_blocking(move || {
            let mut sys = System::new();
            if sys.refresh_process_specifics(Pid::from(pid), ProcessRefreshKind::new()) {
                match sys.process(Pid::from(pid)) {
                    Some(process) => Ok(process.kill()),
                    None => Err(Error::ProcessNotRunning(pid)),
                }
            } else {
                Err(Error::ProcessNotRunning(pid))
            }
        })
        .await??;

        if !killed {
            return Err(Error::ProcessNotKilled(pid));
        }

        trace!("{vm_id}: VM sent KILL signal successfully.");

        Ok(())
    }

    /// Shutdown requests a clean shutdown of the VM by sending CtrlAltDelete on the virtual keyboard.
    #[instrument]
    pub async fn shutdown(&self) -> Result<(), Error> {
        let vm_id = self.config.vm_id();
        trace!("{vm_id}: Sending CTRL+ALT+DEL to VM...");
        self.send_action(Action::SendCtrlAltDel).await?;
        trace!("{vm_id}: CTRL+ALT+DEL sent to VM successfully.");

        Ok(())
    }

    /// Get the configuration of the machine.
    pub fn config(&self) -> &Config<'m> {
        &self.config
    }

    async fn send_action(&self, action: Action) -> Result<(), Error> {
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

    #[instrument]
    async fn setup_resources(&self) -> Result<(), Error> {
        let vm_id = self.config.vm_id();
        trace!("{vm_id}: Configuring machine resources...");
        let json = serde_json::to_string(self.config.machine_cfg())?;
        let url: hyper::Uri = Uri::new(&self.config.socket_path, "/machine-config").into();
        let request = Request::builder()
            .method(Method::PUT)
            .uri(url)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .body(Body::from(json))?;
        self.client.request(request).await?;
        trace!("{vm_id}: Machine resources configured successfully.");

        Ok(())
    }

    #[instrument]
    async fn setup_boot_source(&self) -> Result<(), Error> {
        let vm_id = self.config.vm_id();
        trace!("{vm_id}: Configuring boot source...");
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
        trace!("{vm_id}: Boot source configured successfully.");

        Ok(())
    }

    #[instrument]
    async fn setup_drives(&self) -> Result<(), Error> {
        let vm_id = self.config.vm_id();
        trace!("{vm_id}: Configuring drives...");
        for drive in &self.config.drives {
            let path = format!("/drives/{}", drive.drive_id());
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
        trace!("{vm_id}: Drives configured successfully.");

        Ok(())
    }

    #[instrument]
    async fn setup_network(&self) -> Result<(), Error> {
        let vm_id = self.config.vm_id();
        trace!("{vm_id}: Configuring network...");
        // TODO: check for at least one interface.
        let network = &self.config.network_interfaces()[0];
        let json = json!({
            "iface_id": network.vm_if_name(),
            "host_dev_name": network.host_if_name(),
        });
        let json = serde_json::to_string(&json)?;
        let path = format!("/network-interfaces/{}", network.vm_if_name());
        let url: hyper::Uri = Uri::new(&self.config.socket_path, &path).into();
        let request = Request::builder()
            .method(Method::PUT)
            .uri(url)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .body(Body::from(json))?;
        self.client.request(request).await?;
        trace!("{vm_id}: Network configured successfully.");

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
