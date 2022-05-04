//! A VMM machine.

use crate::{config::Config, Error};
use derivative::Derivative;

/// A VMM machine.
#[derive(Derivative, Default)]
#[derivative(Debug)]
pub struct Machine {
    // TODO:
}

impl Machine {
    /// Create a new machine.
    ///
    /// The machine is not started yet.
    pub async fn new(_config: Config<'_>) -> Result<Self, Error> {
        unimplemented!();
    }

    /// Start the machine.
    pub async fn start(&mut self) -> Result<(), Error> {
        unimplemented!();
    }

    /// Stop the machine.
    pub async fn stop(&mut self) -> Result<(), Error> {
        unimplemented!();
    }

    /// Shutdown requests a clean shutdown of the VM by sending CtrlAltDelete on the virtual keyboard.
    pub async fn shutdown(&mut self) -> Result<(), Error> {
        unimplemented!();
    }
}
