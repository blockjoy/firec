use derivative::Derivative;
use serde::{Deserialize, Serialize};

/// Drive configuration.
// TODO: Provide a builder for `Drive`.
#[derive(Derivative, Serialize, Deserialize, Default)]
#[derivative(Debug)]
pub struct Drive<'d> {
    /// drive id
    drive_id: &'d str,

    /// is read only
    is_read_only: bool,

    /// is root device
    is_root_device: bool,

    /// Represents the unique id of the boot partition of this device.
    ///
    /// It is optional and it will be taken into account only if the is_root_device field is true.
    #[serde(skip_serializing_if = "Option::is_none")]
    part_uuid: Option<&'d str>,

    /// Host level path for the guest drive
    path_on_host: &'d str,
    /* TODO:

    /// rate limiter
    #[serde(skip_serializing_if = "Option::is_none")]
    rate_limiter: Option<RateLimiter>,

    */
}
