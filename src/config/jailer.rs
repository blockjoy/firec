//! API to configure and interact with jailer.

use derivative::Derivative;
use std::{borrow::Cow, path::Path};

use super::Builder;

/// Jailer specific configuration needed to execute the jailer.
#[derive(Debug)]
pub struct Jailer<'j> {
    gid: u32,
    uid: u32,
    numa_node: Option<i32>,
    exec_file: Cow<'j, Path>,
    jailer_binary: Cow<'j, Path>,
    chroot_base_dir: Cow<'j, Path>,
    workspace_dir: Cow<'j, Path>,
    pub(crate) mode: JailerMode<'j>,
    // TODO: We need an equivalent of ChrootStrategy.
}

impl<'j> Jailer<'j> {
    /// GID the jailer switches to as it execs the target binary.
    pub fn gid(&self) -> u32 {
        self.gid
    }

    /// UID the jailer switches to as it execs the target binary.
    pub fn uid(&self) -> u32 {
        self.uid
    }

    /// The NUMA node the process gets assigned to.
    pub fn numa_node(&self) -> Option<i32> {
        self.numa_node
    }

    /// The path to the Firecracker binary that will be exec-ed by the jailer.
    pub fn exec_file(&self) -> &Path {
        &self.exec_file
    }

    /// Specifies the jailer binary to be used for setting up the Firecracker VM jail.
    pub fn jailer_binary(&self) -> &Path {
        &self.jailer_binary
    }

    /// The base folder where chroot jails are built.
    pub fn chroot_base_dir(&self) -> &Path {
        &self.chroot_base_dir
    }

    /// The mode of the jailer process.
    pub fn mode(&self) -> &JailerMode {
        &self.mode
    }

    /// The path to the jailer workspace.
    pub fn workspace_dir(&self) -> &Path {
        &self.workspace_dir
    }
}

/// The mode of the jailer process.
#[derive(Derivative)]
#[derivative(Debug, Default)]
pub enum JailerMode<'j> {
    /// The jailer child process will run attached to the parent process.
    #[derivative(Default)]
    Attached(Stdio),
    /// Calls setsid() and redirect stdin, stdout, and stderr to /dev/null.
    Daemon,
    /// Launch the jailer in a tmux session.
    ///
    /// If the session name is not provided, `<VM_ID>` is used as the session name. tmux will be
    /// launched in detached mode.
    Tmux(Option<Cow<'j, str>>),
}

/// The standard IO handlers.
#[derive(Derivative)]
#[derivative(Debug, Default)]
pub struct Stdio {
    /// Stdout specifies the IO writer for STDOUT to use when spawning the jailer.
    pub stdout: Option<std::process::Stdio>,
    /// Stderr specifies the IO writer for STDERR to use when spawning the jailer.
    pub stderr: Option<std::process::Stdio>,
    /// Stdin specifies the IO reader for STDIN to use when spawning the jailer.
    pub stdin: Option<std::process::Stdio>,
}

/// Builder for `Jailer` instances.
#[derive(Debug)]
pub struct JailerBuilder<'j> {
    jailer: Jailer<'j>,
    config_builder: Builder<'j>,
}

impl<'j> JailerBuilder<'j> {
    pub(crate) fn new(config_builder: Builder<'j>) -> Self {
        Self {
            config_builder,
            jailer: Jailer {
                gid: users::get_effective_gid(),
                uid: users::get_effective_uid(),
                numa_node: None,
                exec_file: Path::new("/usr/bin/firecracker").into(),
                jailer_binary: Path::new("jailer").into(),
                chroot_base_dir: Path::new("/srv/jailer").into(),
                workspace_dir: Path::new("/srv/jailer/firecracker/root").into(),
                mode: JailerMode::default(),
            },
        }
    }

    /// GID the jailer switches to as it execs the target binary.
    pub fn gid(mut self, gid: u32) -> Self {
        self.jailer.gid = gid;
        self
    }

    /// UID the jailer switches to as it execs the target binary.
    pub fn uid(mut self, uid: u32) -> Self {
        self.jailer.uid = uid;
        self
    }

    /// NumaNode represents the NUMA node the process gets assigned to.
    pub fn numa_node(mut self, numa_node: i32) -> Self {
        self.jailer.numa_node = Some(numa_node);
        self
    }

    /// The path to the Firecracker binary that will be exec-ed by the jailer.
    ///
    /// The user can provide a path to any binary, but the interaction
    /// with the jailer is mostly Firecracker specific.
    pub fn exec_file<P>(mut self, exec_file: P) -> Self
    where
        P: Into<Cow<'j, Path>>,
    {
        self.jailer.exec_file = exec_file.into();
        self
    }

    /// Specifies the jailer binary to be used for setting up the Firecracker VM jail.
    ///
    /// If the value contains no path separators, it will use the PATH environment variable to get
    /// the absolute path of the binary. If the value contains path separators, the value will be
    /// used directly to exec the jailer. This follows the same conventions as Golang's
    /// os/exec.Command.
    //
    /// If not specified it defaults to "jailer".
    pub fn jailer_binary<P>(mut self, jailer_binary: P) -> Self
    where
        P: Into<Cow<'j, Path>>,
    {
        self.jailer.jailer_binary = jailer_binary.into();
        self
    }

    /// The base folder where chroot jails are built.
    ///
    /// The default is `/srv/jailer`.
    pub fn chroot_base_dir<P>(mut self, chroot_base_dir: P) -> Self
    where
        P: Into<Cow<'j, Path>>,
    {
        self.jailer.chroot_base_dir = chroot_base_dir.into();
        self
    }

    /// The mode of the jailer process.
    pub fn mode(mut self, mode: JailerMode<'j>) -> Self {
        self.jailer.mode = mode;
        self
    }

    /// Build the `Jailer` instance.
    ///
    /// Returns the main configuration builder with new jailer.
    pub fn build(mut self) -> Builder<'j> {
        let exec_file_base = self
            .jailer
            .exec_file()
            .file_name()
            // FIXME: Check `exec_file` in the `exec_file` method so we can just assume it to
            // have a proper filename here.
            .expect("invalid jailer exec file path");
        let id_str = self.config_builder.0.vm_id().to_string();
        self.jailer.workspace_dir = self
            .jailer
            .chroot_base_dir()
            .join(exec_file_base)
            .join(id_str)
            .join("root")
            .into();
        self.config_builder.0.jailer_cfg = Some(self.jailer);

        self.config_builder
    }
}
