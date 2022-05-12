use std::{borrow::Cow, path::Path};

use serde::{Deserialize, Serialize};

/// Drive configuration.
// TODO: Provide a builder for `Drive`.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Drive<'d> {
    /// drive id
    pub drive_id: Cow<'d, str>,

    /// is read only
    pub is_read_only: bool,

    /// is root device
    pub is_root_device: bool,

    /// Represents the unique id of the boot partition of this device.
    ///
    /// It is optional and it will be taken into account only if the is_root_device field is true.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part_uuid: Option<Cow<'d, str>>,

    /// Host level path for the guest drive
    pub path_on_host: Cow<'d, Path>,
    /* TODO:

    /// rate limiter
    #[serde(skip_serializing_if = "Option::is_none")]
    rate_limiter: Option<RateLimiter>,

    */
}
