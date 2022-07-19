use derivative::Derivative;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, path::Path};

/// VSock configuration.
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
