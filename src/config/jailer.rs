//! API to configure and interact with jailer.

use derivative::Derivative;
use tokio::io::{AsyncRead, AsyncWrite};

/// Jailer specific configuration needed to execute the jailer.
// TODO: Provide a builder for `Jailer`.
#[derive(Derivative, Default)]
#[derivative(Debug)]
pub struct Jailer<'c> {
    /// GID the jailer switches to as it execs the target binary.
    pub gid: i32,

    /// UID the jailer switches to as it execs the target binary.
    pub uid: i32,

    /// The unique VM identification string, which may contain alphanumeric
    /// characters and hyphens. The maximum id length is currently 64 characters
    pub id: &'c str,

    /// NumaNode represents the NUMA node the process gets assigned to.
    pub numa_node: Option<i32>,

    /// The path to the Firecracker binary that will be exec-ed by
    /// the jailer. The user can provide a path to any binary, but the interaction
    /// with the jailer is mostly Firecracker specific.
    pub exec_file: &'c str,

    /// Specifies the jailer binary to be used for setting up the
    /// Firecracker VM jail. If the value contains no path separators, it will
    /// use the PATH environment variable to get the absolute path of the binary.
    /// If the value contains path separators, the value will be used directly
    /// to exec the jailer. This follows the same conventions as Golang's
    /// os/exec.Command.
    //
    /// If not specified it defaults to "jailer".
    pub jailer_binary: Option<&'c str>,

    /// ChrootBaseDir represents the base folder where chroot jails are built. The
    /// default is /srv/jailer
    pub chroot_base_dir: Option<&'c str>,

    /// The mode of the jailer process.
    pub mode: Mode,
    // TODO: We need an equivalent of ChrootStrategy.
}

/// The mode of the jailer process.
#[derive(Derivative)]
#[derivative(Debug, Default)]
pub enum Mode {
    /// The jailer child process will run attached to the parent process.
    #[derivative(Default)]
    Attached(Stdio),
    /// Calls setsid() and redirect stdin, stdout, and stderr to /dev/null.
    Daemon,
}

#[derive(Derivative)]
#[derivative(Debug, Default)]

/// The standard IO handlers.
pub struct Stdio {
    /// Stdout specifies the IO writer for STDOUT to use when spawning the jailer.
    #[derivative(Debug = "ignore")]
    pub stdout: Option<Box<dyn AsyncWrite>>,
    /// Stderr specifies the IO writer for STDERR to use when spawning the jailer.
    #[derivative(Debug = "ignore")]
    pub stderr: Option<Box<dyn AsyncWrite>>,
    /// Stdin specifies the IO reader for STDIN to use when spawning the jailer.
    #[derivative(Debug = "ignore")]
    pub stdin: Option<Box<dyn AsyncRead>>,
}
