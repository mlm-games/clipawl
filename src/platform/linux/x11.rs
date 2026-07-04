//! X11 clipboard backend using clipboard_x11.

use std::sync::{Arc, Mutex};

use crate::{ClipboardOptions, Error, LinuxSelection};

// Static assertion: clipboard_x11::Clipboard must be Send to be wrapped in Mutex.
#[allow(dead_code)]
fn _assert_send()
where
    clipboard_x11::Clipboard: Send,
{
}

pub(crate) struct X11Clipboard {
    selection: LinuxSelection,
    inner: Arc<Mutex<clipboard_x11::Clipboard>>,
}

impl X11Clipboard {
    pub(crate) fn new(opts: &ClipboardOptions) -> Result<Self, Error> {
        let inner = clipboard_x11::Clipboard::connect()
            .map_err(|e| Error::platform("linux/x11: connect", e))?;

        Ok(Self {
            selection: opts.linux.selection,
            inner: Arc::new(Mutex::new(inner)),
        })
    }

    pub(crate) async fn mime_types(&self) -> Result<Vec<String>, Error> {
        Ok(vec!["text/plain".to_string()])
    }

    pub(crate) async fn read(&self, mime_type: &str) -> Result<Vec<u8>, Error> {
        if mime_type != "text/plain" {
            return Err(Error::NotSupported);
        }
        let selection = self.selection;
        let inner = Arc::clone(&self.inner);
        crate::exec::unblock(move || {
            let inner = inner.lock().unwrap();
            let result = match selection {
                LinuxSelection::Clipboard => inner.read(),
                LinuxSelection::Primary => inner.read_primary(),
            };
            result
                .map(|s| s.into_bytes())
                .map_err(|e| Error::platform("linux/x11: read", e))
        })
        .await
    }

    pub(crate) async fn write(&self, mime_type: &str, data: &[u8]) -> Result<(), Error> {
        if mime_type != "text/plain" {
            return Err(Error::NotSupported);
        }
        let text =
            std::str::from_utf8(data).map_err(|e| Error::platform("linux/x11: utf-8", e))?;
        let text = text.to_owned();
        let selection = self.selection;
        let inner = Arc::clone(&self.inner);
        crate::exec::unblock(move || {
            let mut inner = inner.lock().unwrap();
            let result = match selection {
                LinuxSelection::Clipboard => inner.write(text),
                LinuxSelection::Primary => inner.write_primary(text),
            };
            result.map_err(|e| Error::platform("linux/x11: write", e))
        })
        .await
    }
}
