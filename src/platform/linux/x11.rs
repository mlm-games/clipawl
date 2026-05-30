//! X11 clipboard backend using clipboard_x11.

use std::sync::Mutex;

use crate::{ClipboardOptions, Error, LinuxSelection};

pub(crate) struct X11Clipboard {
    selection: LinuxSelection,
    inner: Mutex<clipboard_x11::Clipboard>,
}

impl X11Clipboard {
    pub(crate) fn new(opts: &ClipboardOptions) -> Result<Self, Error> {
        let inner = clipboard_x11::Clipboard::connect()
            .map_err(|e| Error::platform("linux/x11: connect", e))?;

        Ok(Self {
            selection: opts.linux.selection,
            inner: Mutex::new(inner),
        })
    }

    pub(crate) async fn mime_types(&self) -> Result<Vec<String>, Error> {
        Ok(vec!["text/plain".to_string()])
    }

    pub(crate) async fn read(&self, mime_type: &str) -> Result<Vec<u8>, Error> {
        if mime_type == "text/plain" {
            let selection = self.selection;
            tokio::task::block_in_place(|| {
                let inner = self.inner.lock().unwrap();
                let result = match selection {
                    LinuxSelection::Clipboard => inner.read(),
                    LinuxSelection::Primary => inner.read_primary(),
                };
                result
                    .map(|s| s.into_bytes())
                    .map_err(|e| Error::platform("linux/x11: read", e))
            })
        } else {
            Err(Error::NotSupported)
        }
    }

    pub(crate) async fn write(&self, mime_type: &str, data: &[u8]) -> Result<(), Error> {
        if mime_type == "text/plain" {
            let text =
                std::str::from_utf8(data).map_err(|e| Error::platform("linux/x11: utf-8", e))?;
            let text = text.to_owned();
            let selection = self.selection;
            tokio::task::block_in_place(|| {
                let mut inner = self.inner.lock().unwrap();
                let result = match selection {
                    LinuxSelection::Clipboard => inner.write(text),
                    LinuxSelection::Primary => inner.write_primary(text),
                };
                result.map_err(|e| Error::platform("linux/x11: write", e))
            })
        } else {
            Err(Error::NotSupported)
        }
    }
}
