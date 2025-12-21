//! X11 clipboard backend using clipboard_x11.

use crate::{ClipboardOptions, Error, LinuxSelection};

pub(crate) struct X11Clipboard {
    selection: LinuxSelection,
    inner: clipboard_x11::Clipboard,
}

impl X11Clipboard {
    pub(crate) fn new(opts: &ClipboardOptions) -> Result<Self, Error> {
        let inner = clipboard_x11::Clipboard::connect()
            .map_err(|e| Error::platform("linux/x11: connect", e))?;

        Ok(Self {
            selection: opts.linux.selection,
            inner,
        })
    }

    pub(crate) async fn get_text(&mut self) -> Result<String, Error> {
        let text = match self.selection {
            LinuxSelection::Clipboard => self
                .inner
                .read()
                .map_err(|e| Error::platform("linux/x11: read CLIPBOARD", e))?,
            LinuxSelection::Primary => self
                .inner
                .read_primary()
                .map_err(|e| Error::platform("linux/x11: read PRIMARY", e))?,
        };
        Ok(text)
    }

    pub(crate) async fn set_text(&mut self, text: &str) -> Result<(), Error> {
        match self.selection {
            LinuxSelection::Clipboard => self
                .inner
                .write(text.to_owned())
                .map_err(|e| Error::platform("linux/x11: write CLIPBOARD", e))?,
            LinuxSelection::Primary => self
                .inner
                .write_primary(text.to_owned())
                .map_err(|e| Error::platform("linux/x11: write PRIMARY", e))?,
        }
        Ok(())
    }
}
