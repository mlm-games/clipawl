//! Fallback implementation for unsupported platforms.

use crate::{ClipboardOptions, Error};

pub(crate) struct ClipboardImpl;

impl ClipboardImpl {
    pub(crate) fn new(_opts: &ClipboardOptions) -> Result<Self, Error> {
        Err(Error::NotSupported)
    }

    pub(crate) async fn get_text(&self) -> Result<String, Error> {
        Err(Error::NotSupported)
    }

    pub(crate) async fn set_text(&self, _text: &str) -> Result<(), Error> {
        Err(Error::NotSupported)
    }
}
