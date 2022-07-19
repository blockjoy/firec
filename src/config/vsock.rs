use derivative::Derivative;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, path::Path};

/// VSock configuration.
///
/// For information about VSOCK, please refer to its [manpage]. For details on how to use VSOCK with
/// Firecracker, please refer to the relevant [Firecracker documentation].
///
/// [manpage]: https://man7.org/linux/man-pages/man7/vsock.7.html
/// [Firecracker documentation]: https://github.com/firecracker-microvm/firecracker/blob/main/docs/vsock.md
#[derive(Derivative, Debug, Serialize, Deserialize)]
pub struct VSock<'v> {
    pub(crate) guest_cid: u32,
    pub(crate) uds_path: Cow<'v, Path>,
}

impl VSock<'_> {
    /// The Context ID.
    pub fn guest_cid(&self) -> u32 {
        self.guest_cid
    }

    /// The path to the Unix socket file.
    ///
    /// For guest-initialiated connections, a `_PORT` suffix is expected in the actual socket
    /// filename.
    pub fn uds_path(&self) -> &Path {
        &self.uds_path
    }
}
