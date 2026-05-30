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

    pub(crate) async fn get_text(&self) -> Result<String, Error> {
        let selection = self.selection;
        tokio::task::block_in_place(|| {
            let inner = self.inner.lock().unwrap();
            let result = match selection {
                LinuxSelection::Clipboard => inner.read(),
                LinuxSelection::Primary => inner.read_primary(),
            };
            result.map_err(|e| Error::platform("linux/x11: read", e))
        })
    }

    pub(crate) async fn set_text(&self, text: &str) -> Result<(), Error> {
        let selection = self.selection;
        let text = text.to_owned();
        tokio::task::block_in_place(|| {
            let mut inner = self.inner.lock().unwrap();
            let result = match selection {
                LinuxSelection::Clipboard => inner.write(text),
                LinuxSelection::Primary => inner.write_primary(text),
            };
            result.map_err(|e| Error::platform("linux/x11: write", e))
        })
    }
}
