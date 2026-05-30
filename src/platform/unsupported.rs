//! Fallback implementation for unsupported platforms.

use crate::{ClipboardOptions, Error};

pub(crate) struct ClipboardImpl;

impl ClipboardImpl {
    pub(crate) fn new(_opts: &ClipboardOptions) -> Result<Self, Error> {
        Err(Error::NotSupported)
    }

    pub(crate) async fn mime_types(&self) -> Result<Vec<String>, Error> {
        Ok(Vec::new())
    }

    pub(crate) async fn read(&self, _mime_type: &str) -> Result<Vec<u8>, Error> {
        Err(Error::NotSupported)
    }

    pub(crate) async fn write(&self, _mime_type: &str, _data: &[u8]) -> Result<(), Error> {
        Err(Error::NotSupported)
    }
}
