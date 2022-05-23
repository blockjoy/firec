use std::{borrow::Cow, path::Path};

use serde::{Deserialize, Serialize};

/// Drive configuration.
#[derive(Debug, Serialize, Deserialize)]
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

impl<'d> Drive<'d> {
    /// Create a new `DriveBuilder` instance.
    pub fn builder<I, P>(drive_id: I, path_on_host: P) -> DriveBuilder<'d>
    where
        I: Into<Cow<'d, str>>,
        P: Into<Cow<'d, Path>>,
    {
        DriveBuilder(Drive {
            drive_id: drive_id.into(),
            is_read_only: false,
            is_root_device: false,
            part_uuid: None,
            path_on_host: path_on_host.into(),
        })
    }
}

/// Builder for `Drive`.
#[derive(Debug)]
pub struct DriveBuilder<'d>(Drive<'d>);

impl<'d> DriveBuilder<'d> {
    /// If to-be-created `Drive` will be read-only.
    pub fn is_read_only(mut self, is_read_only: bool) -> Self {
        self.0.is_read_only = is_read_only;
        self
    }

    /// If to-be-created `Drive` will be the root device.
    pub fn is_root_device(mut self, is_root_device: bool) -> Self {
        self.0.is_root_device = is_root_device;
        self
    }

    /// Set the unique id of the boot partition of this device.
    ///
    /// It is optional and it will be taken into account only if its root device.
    pub fn part_uuid<U>(mut self, part_uuid: Option<U>) -> Self
    where
        U: Into<Cow<'d, str>>,
    {
        self.0.part_uuid = part_uuid.map(Into::into);
        self
    }

    /// Build the `Drive`.
    pub fn build(self) -> Drive<'d> {
        self.0
    }
}
