//! A VMM machine.

use std::{io::ErrorKind, path::Path, process::Stdio, time::Duration};

use crate::{
    config::{Config, JailerMode},
    Error,
};
use futures_util::TryFutureExt;
use serde::Serialize;
use serde_json::json;
use sysinfo::{Pid, PidExt, ProcessExt, ProcessRefreshKind, System, SystemExt};
use tokio::{
    fs::{self, copy, DirBuilder},
    process::Command,
    task,
    time::sleep,
};
use tracing::{info, instrument, trace, warn};

use hyper::{Body, Client, Method, Request};
use hyperlocal::{UnixClientExt, UnixConnector, Uri};

const JAILER_START_TIMEOUT: Duration = Duration::from_secs(10);

/// A VMM machine.
#[derive(Debug)]
pub struct Machine<'m> {
    config: Config<'m>,
    /// Pid of a started jailer/firecracker process, or None if not started yet
    pid: Option<u32>,
    client: Client<UnixConnector>,
}

/// VM state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MachineState {
    /// Machine is not started or already shut down
    SHUTOFF,
    /// Machine is running
    RUNNING,
}

impl<'m> Machine<'m> {
    /// Create a new machine.
    ///
    /// The machine is not started yet.
    #[instrument(skip_all)]
    pub async fn create(config: Config<'m>) -> Result<Machine<'m>, Error> {
        let vm_id = *config.vm_id();
        info!("Creating new machine with VM ID `{vm_id}`");
        trace!("{vm_id}: Configuration: {:?}", config);

        let jailer_workspace_dir = config.jailer().workspace_dir();
        trace!(
            "{vm_id}: Ensuring Jailer workspace directory exist at `{}`",
            jailer_workspace_dir.display()
        );
        DirBuilder::new()
            .recursive(true)
            .create(jailer_workspace_dir)
            .await?;

        let dest = config.kernel_image_path();
        if dest.exists() {
            trace!(
                "{vm_id}: Skipping existing kernel image at `{}`",
                dest.display()
            );
        } else {
            trace!(
                "{vm_id}: Copying kernel image from `{}` to `{}`",
                config.src_kernel_image_path.display(),
                dest.display()
            );
            copy(config.src_kernel_image_path(), dest).await?;
        }

        if let (Some(src_initrd_path), Some(initrd_path)) =
            (config.src_initrd_path(), config.initrd_path()?)
        {
            if initrd_path.exists() {
                trace!(
                    "{vm_id}: Skipping existing initrd at `{}`",
                    initrd_path.display()
                );
            } else {
                trace!(
                    "{vm_id}: Copying initrd from `{}` to `{}`",
                    src_initrd_path.display(),
                    initrd_path.display()
                );
                copy(src_initrd_path, initrd_path).await?;
            }
        }

        for drive in &config.drives {
            let drive_filename = drive
                .src_path()
                .file_name()
                .ok_or(Error::InvalidDrivePath)?;
            let dest = jailer_workspace_dir.join(drive_filename);
            if dest.exists() {
                trace!("{vm_id}: Skipping existing drive at `{}`", dest.display());
            } else {
                trace!(
                    "{vm_id}: Copying drive `{}` from `{}` to `{}`",
                    drive.drive_id(),
                    drive.src_path().display(),
                    dest.display()
                );
                copy(&drive.src_path(), dest).await?;
            }
        }

        if let Some(socket_dir) = config.host_socket_path().parent() {
            trace!(
                "{vm_id}: Ensuring socket directory exist at `{}`",
                socket_dir.display()
            );
            DirBuilder::new().recursive(true).create(socket_dir).await?;
        }

        // TODO: Handle fifos. See https://github.com/firecracker-microvm/firecracker-go-sdk/blob/f0a967ef386caec37f6533dce5797038edf8c226/jailer.go#L435

        // `request` doesn't provide API to connect to unix sockets so we we use the low-level
        // approach using hyper: https://github.com/seanmonstar/reqwest/issues/39
        let client = Client::unix();

        let machine = Self {
            config,
            pid: None,
            client,
        };

        Ok(machine)
    }

    /// Connect to already created machine.
    ///
    /// The machine should be created first via call to `create`
    #[instrument(skip_all)]
    pub async fn connect(config: Config<'m>, pid: Option<u32>) -> Machine<'m> {
        let vm_id = *config.vm_id();
        info!("Connecting to machine with VM ID `{vm_id}`");
        trace!("{vm_id}: Configuration: {:?}, pid: {:?}", config, pid);

        let client = Client::unix();

        Self {
            config,
            pid,
            client,
        }
    }

    /// Start the machine.
    #[instrument(skip_all)]
    pub async fn start(&mut self) -> Result<(), Error> {
        if self.state() == MachineState::RUNNING {
            return Err(Error::ProcessAlreadyRunning);
        }
        let vm_id = self.config.vm_id().to_string();
        info!("Starting machine with VM ID `{vm_id}`");

        self.cleanup_before_starting().await?;

        // FIXME: Assuming jailer for now.
        let jailer = self.config.jailer_cfg.as_mut().expect("no jailer config");
        let jailer_bin = jailer.jailer_binary().to_owned();
        let jailer_exec_path = jailer
            .exec_file()
            .to_str()
            .ok_or(Error::InvalidJailerExecPath)?
            .to_owned();
        let (mut cmd, daemonize_arg, stdin, stdout, stderr) = match &mut jailer.mode {
            JailerMode::Daemon => (
                Command::new(jailer.jailer_binary()),
                Some("--daemonize"),
                Stdio::null(),
                Stdio::null(),
                Stdio::null(),
            ),
            JailerMode::Attached(stdio) => (
                Command::new(jailer_bin),
                None,
                stdio.stdin.take().unwrap_or_else(Stdio::inherit),
                stdio.stdout.take().unwrap_or_else(Stdio::inherit),
                stdio.stderr.take().unwrap_or_else(Stdio::inherit),
            ),
            JailerMode::Tmux(session_name) => {
                let session_name = session_name
                    .clone()
                    .unwrap_or_else(|| vm_id.to_string().into());
                let mut cmd = Command::new("tmux");
                cmd.args([
                    "new-session",
                    "-d",
                    "-s",
                    &session_name,
                    jailer.jailer_binary().to_str().unwrap(),
                ]);

                (cmd, None, Stdio::null(), Stdio::null(), Stdio::null())
            }
        };

        if let Some(daemonize_arg) = daemonize_arg {
            cmd.arg(daemonize_arg);
        }
        let cmd = cmd
            .args([
                "--id",
                &vm_id,
                "--exec-file",
                &jailer_exec_path,
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
                self.config
                    .socket_path
                    .to_str()
                    .ok_or(Error::InvalidSocketPath)?,
            ])
            .stdin(stdin)
            .stdout(stdout)
            .stderr(stderr);
        trace!("{vm_id}: Running command: {:?}", cmd);
        let mut child = cmd.spawn()?;
        if child.id().is_none() {
            let exit_status = child.wait().await?;
            return Err(Error::ProcessExitedImmediatelly { exit_status });
        }
        self.pid = Some(self.wait_for_jailer(&jailer_exec_path).await?);

        if let Err(e) = self
            .setup_vm()
            .and_then(|_| async {
                trace!("{vm_id}: Booting the VM instance...");

                self.send_action(Action::InstanceStart).await
            })
            .await
        {
            warn!(
                "{vm_id}: Failed to boot VM instance: {}. Force shutting down..",
                e
            );
            self.force_shutdown().await.unwrap_or_else(|e| {
                // We want to return to original error so only log the error from shutdown.
                warn!("{vm_id}: Failed to force shutdown: {}", e);
            });

            return Err(e);
        }

        trace!("{vm_id}: VM started successfully.");

        Ok(())
    }

    /// Forcefully shutdown the machine.
    ///
    /// This will be done by killing VM process.
    #[instrument(skip_all)]
    pub async fn force_shutdown(&mut self) -> Result<(), Error> {
        let vm_id = self.config.vm_id();
        info!("{vm_id}: Killing VM...");

        let pid = self.pid.ok_or(Error::ProcessNotStarted)?;
        match self.config.jailer_cfg().expect("no jailer config").mode() {
            JailerMode::Daemon | JailerMode::Attached(_) => {
                let killed = task::spawn_blocking(move || {
                    let mut sys = System::new();
                    if sys.refresh_process_specifics(Pid::from_u32(pid), ProcessRefreshKind::new())
                    {
                        match sys.process(Pid::from_u32(pid)) {
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
                trace!("{vm_id}: Successfully sent KILL signal to VM (pid: `{pid}`).");
            }
            JailerMode::Tmux(session_name) => {
                let session_name = session_name
                    .clone()
                    .unwrap_or_else(|| vm_id.to_string().into());
                // In case of tmux, we need to kill the tmux session.
                let cmd = &mut Command::new("tmux");
                cmd.args(["kill-session", "-t", &session_name]);
                trace!("{vm_id}: Running command: {:?}", cmd);
                cmd.spawn()?.wait().await?;
            }
        }
        self.pid = None;
        Ok(())
    }

    /// Shutdown requests a clean shutdown of the VM by sending CtrlAltDelete on the virtual keyboard.
    #[instrument(skip_all)]
    pub async fn shutdown(&self) -> Result<(), Error> {
        let vm_id = self.config.vm_id();
        info!("{vm_id}: Sending CTRL+ALT+DEL to VM...");
        self.send_action(Action::SendCtrlAltDel).await?;
        trace!("{vm_id}: CTRL+ALT+DEL sent to VM successfully.");
        Ok(())
    }

    /// Delete the machine.
    ///
    /// Deletes the machine, cleaning up all associated resources.
    ///
    /// If machine is running, it is shut down before resources are deleted.
    #[instrument(skip_all)]
    pub async fn delete(mut self) -> Result<(), Error> {
        let vm_id = self.config.vm_id().to_string();
        info!("{vm_id}: Deleting VM...");

        let jailer_workspace_dir = self.config.jailer_cfg().unwrap().workspace_dir().to_owned();

        if MachineState::RUNNING == self.state() {
            if let Err(err) = self.shutdown().await {
                warn!("{vm_id}: Shutdown error: {err}");
            } else {
                info!("{vm_id}: Waiting for the VM process to shut down...");
                sleep(Duration::from_secs(10)).await;
            }

            if let Err(err) = self.force_shutdown().await {
                warn!("{vm_id}: Forced shutdown error: {err}");
            }
        }

        trace!("{vm_id}: Deleting VM resources...");
        // The jailer workspace dir is `root` dir under the VM dir and we want to delete everything
        // related to the VM so we need to delete the VM dir, and not just the workspace dir under
        // it.
        let vm_dir = jailer_workspace_dir
            .parent()
            .expect("VM workspace dir must have a parent");
        trace!(
            "{vm_id}: Deleting VM jailer directory at `{}`",
            vm_dir.display()
        );
        fs::remove_dir_all(vm_dir).await?;
        trace!("{vm_id}: VM deleted successfully.");

        Ok(())
    }

    /// Get the configuration of the machine.
    pub fn config(&self) -> &Config<'m> {
        &self.config
    }

    /// Checks the machine actual state
    ///
    /// Returns SHUTOFF is machine is not running
    pub fn state(&self) -> MachineState {
        if let Some(pid) = self.pid {
            let mut sys = System::new();
            // TODO set self.pid=None somewhere if process doesn't exists anymore
            if sys.refresh_process_specifics(Pid::from_u32(pid), ProcessRefreshKind::new()) {
                if sys.process(Pid::from_u32(pid)).is_some() {
                    MachineState::RUNNING
                } else {
                    MachineState::SHUTOFF
                }
            } else {
                MachineState::SHUTOFF
            }
        } else {
            MachineState::SHUTOFF
        }
    }

    #[instrument(skip_all)]
    async fn wait_for_jailer(&self, jailer_exec_path: &str) -> Result<u32, Error> {
        let vm_id = self.config.vm_id();
        // Wait jailer to start up and create the socket.
        info!("{vm_id}: Waiting for the jailer to start up...");

        // get try to get FC version to verify if jailer already started
        let request = || {
            Request::builder()
                .method(Method::GET)
                .uri(Uri::new(self.config.host_socket_path(), "/version"))
                .header("Accept", "application/json")
                .header("Content-Type", "application/json")
                .body(Body::empty())
        };
        let start = std::time::Instant::now();
        let elapsed = || std::time::Instant::now() - start;
        while !self.client.request(request()?).await?.status().is_success() {
            if elapsed() < JAILER_START_TIMEOUT {
                sleep(Duration::from_millis(100)).await;
            } else {
                return Err(Error::JailerStartTimedOut);
            }
        }
        // get PID of started firecracker
        let mut sys = System::new();
        sys.refresh_specifics(
            sysinfo::RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
        );
        let processes: Vec<_> = sys
            .processes_by_name(jailer_exec_path)
            .filter(|&process| process.cmd().contains(&vm_id.to_string()))
            .collect();

        match processes.len() {
            1 => Ok(processes[0].pid().as_u32()),
            _ => Err(Error::FailedToStart),
        }
    }

    #[instrument(skip_all)]
    async fn send_request(&self, url: hyper::Uri, body: String) -> Result<(), Error> {
        let vm_id = self.config.vm_id();
        trace!("{vm_id}: sending request to url={url}, body={body}");

        let request = Request::builder()
            .method(Method::PUT)
            .uri(url.clone())
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .body(Body::from(body))?;

        let resp = self.client.request(request).await?;

        let status = resp.status();
        if status.is_success() {
            trace!("{vm_id}: request to url={url} successful");
        } else {
            let body = hyper::body::to_bytes(resp.into_body()).await?;
            let body = if body.is_empty() {
                trace!("{vm_id}: request to url={url} failed: status={status}");
                None
            } else {
                let body = String::from_utf8_lossy(&body).into_owned();
                trace!("{vm_id}: request to url={url} failed: status={status}, body={body}");
                Some(body)
            };
            return Err(Error::FirecrackerAPIError { status, body });
        }

        Ok(())
    }

    async fn send_action(&self, action: Action) -> Result<(), Error> {
        let url: hyper::Uri = Uri::new(self.config.host_socket_path(), "/actions").into();
        let json = serde_json::to_string(&action)?;
        self.send_request(url, json).await?;

        Ok(())
    }

    /// Prepare the machine for running.
    #[instrument(skip_all)]
    async fn setup_vm(&self) -> Result<(), Error> {
        let vm_id = self.config.vm_id();
        info!("{vm_id}: Setting the VM...");
        self.setup_resources().await?;
        self.setup_boot_source().await?;
        self.setup_drives().await?;
        self.setup_network().await?;
        self.setup_vsock().await?;
        trace!("{vm_id}: VM successfully setup.");

        Ok(())
    }

    #[instrument(skip_all)]
    async fn setup_resources(&self) -> Result<(), Error> {
        let vm_id = self.config.vm_id();
        trace!("{vm_id}: Configuring machine resources...");
        let json = serde_json::to_string(self.config.machine_cfg())?;
        let url: hyper::Uri = Uri::new(self.config.host_socket_path(), "/machine-config").into();
        self.send_request(url, json).await?;
        trace!("{vm_id}: Machine resources configured successfully.");

        Ok(())
    }

    #[instrument(skip_all)]
    async fn setup_boot_source(&self) -> Result<(), Error> {
        let vm_id = self.config.vm_id();
        trace!("{vm_id}: Configuring boot source...");
        let boot_source = self.config.boot_source()?;
        let json = serde_json::to_string(&boot_source)?;
        let url: hyper::Uri = Uri::new(self.config.host_socket_path(), "/boot-source").into();
        self.send_request(url, json).await?;
        trace!("{vm_id}: Boot source configured successfully.");

        Ok(())
    }

    #[instrument(skip_all)]
    async fn setup_drives(&self) -> Result<(), Error> {
        let vm_id = self.config.vm_id();
        trace!("{vm_id}: Configuring drives...");
        for drive in &self.config.drives {
            let path = format!("/drives/{}", drive.drive_id());
            let url: hyper::Uri = Uri::new(self.config.host_socket_path(), &path).into();
            // Send modified drive object, with drive file in chroot location
            let mut drive_obj = drive.clone();
            let drive_filename = drive
                .src_path()
                .file_name()
                .ok_or(Error::InvalidDrivePath)?;
            drive_obj.src_path = Path::new(&drive_filename).into();
            let json = serde_json::to_string(&drive_obj)?;
            self.send_request(url, json).await?;
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
        let url: hyper::Uri = Uri::new(self.config.host_socket_path(), &path).into();
        self.send_request(url, json).await?;
        trace!("{vm_id}: Network configured successfully.");

        Ok(())
    }

    #[instrument(skip_all)]
    async fn setup_vsock(&self) -> Result<(), Error> {
        let vsock_cfg = match self.config.vsock_cfg() {
            Some(vsock) => vsock,
            None => return Ok(()),
        };
        let vm_id = self.config.vm_id();
        trace!("{vm_id}: Configuring vsock...");
        let url: hyper::Uri = Uri::new(self.config.host_socket_path(), "/vsock").into();
        let json = serde_json::to_string(vsock_cfg)?;
        self.send_request(url, json).await?;
        trace!("{vm_id}: vsock configured successfully.");

        Ok(())
    }

    #[instrument(skip_all)]
    async fn cleanup_before_starting(&self) -> Result<(), Error> {
        let vm_id = self.config.vm_id();
        trace!("{vm_id}: Deleting intermediate VM resources before starting...");
        let socket_path = self.config.host_socket_path();
        trace!("{vm_id}: Removing socket file {}...", socket_path.display());
        match fs::remove_file(&socket_path).await {
            Ok(_) => trace!("{vm_id}: Deleted `{}`", socket_path.display()),
            Err(e) if e.kind() == ErrorKind::NotFound => {
                trace!("{vm_id}: `{}` not found", socket_path.display())
            }
            Err(e) => return Err(e.into()),
        }

        let jailer_workspace_dir = self.config.jailer().workspace_dir();

        // Remove the vsock socket file if it exists.
        if let Some(path) = self.config.vsock_cfg().map(|v| v.uds_path()) {
            let relative_path = path.strip_prefix("/").unwrap_or(path);
            let path = jailer_workspace_dir.join(relative_path);
            trace!("{vm_id}: Removing vsock socket file {}...", path.display());
            match fs::remove_file(&path).await {
                Ok(_) => trace!("{vm_id}: Deleted `{}`", path.display()),
                Err(e) if e.kind() == ErrorKind::NotFound => {
                    trace!("{vm_id}: `{}` not found", path.display())
                }
                Err(e) => return Err(e.into()),
            }
        }

        let dev_dir = jailer_workspace_dir.join("dev");
        trace!("{vm_id}: Deleting `{}`", dev_dir.display());
        match fs::remove_dir_all(&dev_dir).await {
            Ok(_) => trace!("{vm_id}: Deleted `{}`", dev_dir.display()),
            Err(e) if e.kind() == ErrorKind::NotFound => {
                trace!("{vm_id}: `{}` not found", dev_dir.display())
            }
            Err(e) => return Err(e.into()),
        }

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
