use std::{borrow::Cow, path::Path};

use serde::{Deserialize, Serialize};

use super::Builder;

/// Drive configuration.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Drive<'d> {
    drive_id: Cow<'d, str>,
    is_read_only: bool,
    is_root_device: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    part_uuid: Option<Cow<'d, str>>,
    #[serde(rename = "path_on_host")]
    pub(crate) src_path: Cow<'d, Path>,
    /* TODO:

    /// rate limiter
    #[serde(skip_serializing_if = "Option::is_none")]
    rate_limiter: Option<RateLimiter>,

    */
}

impl<'d> Drive<'d> {
    /// The drive ID.
    pub fn drive_id(&self) -> &str {
        &self.drive_id
    }

    /// If the drive is read-only.
    pub fn is_read_only(&self) -> bool {
        self.is_read_only
    }

    /// If the drive is the root device.
    pub fn is_root_device(&self) -> bool {
        self.is_root_device
    }

    /// The unique id of the boot partition of this device.
    pub fn part_uuid(&self) -> Option<&str> {
        self.part_uuid.as_deref()
    }

    /// The source path for the guest drive.
    ///
    /// This is the path given by the application. The drive is transfered to the chroot directory
    /// by [`crate::Machine::create`].
    pub fn src_path(&self) -> &Path {
        &self.src_path
    }
}

/// Builder for `Drive`.
#[derive(Debug)]
pub struct DriveBuilder<'d> {
    config_builder: Builder<'d>,
    drive: Drive<'d>,
}

impl<'d> DriveBuilder<'d> {
    pub(crate) fn new<I, P>(config_builder: Builder<'d>, drive_id: I, src_path: P) -> Self
    where
        I: Into<Cow<'d, str>>,
        P: Into<Cow<'d, Path>>,
    {
        Self {
            config_builder,
            drive: Drive {
                drive_id: drive_id.into(),
                is_read_only: false,
                is_root_device: false,
                part_uuid: None,
                src_path: src_path.into(),
            },
        }
    }

    /// If to-be-created `Drive` will be read-only.
    pub fn is_read_only(mut self, is_read_only: bool) -> Self {
        self.drive.is_read_only = is_read_only;
        self
    }

    /// If to-be-created `Drive` will be the root device.
    pub fn is_root_device(mut self, is_root_device: bool) -> Self {
        self.drive.is_root_device = is_root_device;
        self
    }

    /// Set the unique id of the boot partition of this device.
    ///
    /// It is optional and it will be taken into account only if its root device.
    pub fn part_uuid<U>(mut self, part_uuid: Option<U>) -> Self
    where
        U: Into<Cow<'d, str>>,
    {
        self.drive.part_uuid = part_uuid.map(Into::into);
        self
    }

    /// Build the `Drive`.
    ///
    /// Returns the main configuration builder with the new drive added to it.
    pub fn build(mut self) -> Builder<'d> {
        self.config_builder.0.drives.push(self.drive);

        self.config_builder
    }
}
