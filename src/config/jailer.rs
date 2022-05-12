//! API to configure and interact with jailer.

use derivative::Derivative;
use std::{borrow::Cow, path::Path};
use uuid::Uuid;

/// Jailer specific configuration needed to execute the jailer.
// TODO: Provide a builder for `Jailer`.
#[derive(Derivative, Debug)]
#[derivative(Default)]
pub struct Jailer<'c> {
    /// GID the jailer switches to as it execs the target binary.
    #[derivative(Default(value = "users::get_effective_gid()"))]
    pub gid: u32,

    /// UID the jailer switches to as it execs the target binary.
    #[derivative(Default(value = "users::get_effective_uid()"))]
    pub uid: u32,

    /// The unique VM identification string, which may contain alphanumeric
    /// characters and hyphens. The maximum id length is currently 64 characters
    #[derivative(Default(value = "uuid::Uuid::new_v4()"))]
    pub id: Uuid,

    /// NumaNode represents the NUMA node the process gets assigned to.
    pub numa_node: Option<i32>,

    /// The path to the Firecracker binary that will be exec-ed by
    /// the jailer. The user can provide a path to any binary, but the interaction
    /// with the jailer is mostly Firecracker specific.
    pub exec_file: Cow<'c, Path>,

    /// Specifies the jailer binary to be used for setting up the
    /// Firecracker VM jail. If the value contains no path separators, it will
    /// use the PATH environment variable to get the absolute path of the binary.
    /// If the value contains path separators, the value will be used directly
    /// to exec the jailer. This follows the same conventions as Golang's
    /// os/exec.Command.
    //
    /// If not specified it defaults to "jailer".
    #[derivative(Default(value = "Path::new(\"jailer\").into()"))]
    pub jailer_binary: Cow<'c, Path>,

    /// represents the base folder where chroot jails are built. The
    /// default is `/srv/jailer`.
    #[derivative(Default(value = "Path::new(\"/srv/jailer\").into()"))]
    pub chroot_base_dir: Cow<'c, Path>,

    /// The mode of the jailer process.
    pub mode: JailerMode,
    // TODO: We need an equivalent of ChrootStrategy.
}

/// The mode of the jailer process.
#[derive(Derivative)]
#[derivative(Debug, Default)]
pub enum JailerMode {
    /// The jailer child process will run attached to the parent process.
    #[derivative(Default)]
    Attached(Stdio),
    /// Calls setsid() and redirect stdin, stdout, and stderr to /dev/null.
    Daemon,
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
