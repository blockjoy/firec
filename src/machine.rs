//! A VMM machine.

use std::{process::Stdio, time::Duration};

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
    #[instrument(skip_all)]
    pub async fn create(mut config: Config<'m>) -> Result<Machine<'m>, Error> {
        let vm_id = *config.vm_id();
        info!("Creating new machine with VM ID `{vm_id}`");
        trace!("{vm_id}: Configuration: {:?}", config);

        let id_str = vm_id.to_string();

        let jailer_workspace_dir = config.jailer_workspace_dir.as_ref();
        info!(
            "{vm_id}: Ensuring Jailer workspace directory exist at `{}`",
            jailer_workspace_dir.display()
        );
        DirBuilder::new()
            .recursive(true)
            .create(jailer_workspace_dir)
            .await?;

        let dest = config.host_kernel_image_path.as_ref();
        trace!(
            "{vm_id}: Copying kernel image from `{}` to `{}`",
            config.kernel_image_path.display(),
            dest.display()
        );
        copy(config.kernel_image_path(), dest).await?;

        if let (Some(initrd_path), Some(host_initrd_path)) = (
            config.initrd_path.as_ref(),
            config.host_initrd_path.as_ref(),
        ) {
            trace!(
                "{vm_id}: Copying initrd from `{}` to `{}`",
                initrd_path.display(),
                host_initrd_path.display()
            );
            copy(initrd_path.as_os_str(), host_initrd_path.as_os_str()).await?;
        }

        for drive in &config.drives {
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
        }

        if let Some(socket_dir) = config.host_socket_path.parent() {
            trace!(
                "{vm_id}: Ensuring socket directory exist at `{}`",
                socket_dir.display()
            );
            DirBuilder::new().recursive(true).create(socket_dir).await?;
        }

        // TODO: Handle fifos. See https://github.com/firecracker-microvm/firecracker-go-sdk/blob/f0a967ef386caec37f6533dce5797038edf8c226/jailer.go#L435

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
                config
                    .socket_path
                    .to_str()
                    .ok_or(Error::InvalidSocketPath)?,
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

    /// Connect to already running machine.
    ///
    /// The machine should be created first via call to `create`.
    #[instrument(skip_all)]
    pub async fn connect(config: Config<'m>, pid: i32) -> Machine<'m> {
        let vm_id = *config.vm_id();
        info!("Connecting to machine with VM ID `{vm_id}`");
        trace!("{vm_id}: Configuration: {:?}, pid: {}", config, pid);

        let client = Client::unix();

        Self {
            config,
            pid,
            client,
        }
    }

    /// Start the machine.
    #[instrument(skip_all)]
    pub async fn start(&self) -> Result<(), Error> {
        let vm_id = self.config.vm_id();
        trace!("{vm_id}: Starting the VM...");
        // Start the machine.
        self.send_action(Action::InstanceStart).await?;
        trace!("{vm_id}: VM started successfully.");

        Ok(())
    }

    /// Stop the machine.
    #[instrument(skip_all)]
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
    #[instrument(skip_all)]
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

    /// Get the PID of the jailer/firecracker process
    pub fn pid(&self) -> i32 {
        self.pid
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

    #[instrument(skip_all)]
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

    #[instrument(skip_all)]
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

    #[instrument(skip_all)]
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

    #[instrument(skip_all)]
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
