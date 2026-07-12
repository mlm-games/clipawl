use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::{ClipboardOptions, Error, LinuxSelection};

pub(crate) struct X11Clipboard {
    selection: LinuxSelection,
    inner: Arc<Mutex<x11_clipboard::Clipboard>>,
}

impl X11Clipboard {
    pub(crate) fn new(opts: &ClipboardOptions) -> Result<Self, Error> {
        let inner = x11_clipboard::Clipboard::new()
            .map_err(|e| Error::platform("linux/x11: connect", e))?;

        Ok(Self {
            selection: opts.linux.selection,
            inner: Arc::new(Mutex::new(inner)),
        })
    }

    pub(crate) async fn read(&self, mime_type: &str) -> Result<Vec<u8>, Error> {
        let selection = self.selection;
        let mime_type = mime_type.to_owned();
        let inner = Arc::clone(&self.inner);

        crate::exec::unblock(move || {
            let cb = inner.lock().unwrap_or_else(|e| e.into_inner());
            let atoms = &cb.getter.atoms;

            let selection_atom = match selection {
                LinuxSelection::Clipboard => atoms.clipboard,
                LinuxSelection::Primary => atoms.primary,
            };

            let target = if mime_type == "text/plain" {
                atoms.utf8_string
            } else {
                cb.getter
                    .get_atom(&mime_type)
                    .map_err(|e| Error::platform("linux/x11: intern target atom", e))?
            };

            cb.load(
                selection_atom,
                target,
                atoms.property,
                Duration::from_secs(3),
            )
            .map_err(|e| Error::platform("linux/x11: read", e))
        })
        .await
    }

    pub(crate) async fn write(&self, mime_type: &str, data: &[u8]) -> Result<(), Error> {
        let selection = self.selection;
        let mime_type = mime_type.to_owned();
        let data = data.to_vec();
        let inner = Arc::clone(&self.inner);

        crate::exec::unblock(move || {
            let cb = inner.lock().unwrap_or_else(|e| e.into_inner());
            let atoms = &cb.setter.atoms;

            let selection_atom = match selection {
                LinuxSelection::Clipboard => atoms.clipboard,
                LinuxSelection::Primary => atoms.primary,
            };

            let target = if mime_type == "text/plain" {
                atoms.utf8_string
            } else {
                cb.setter
                    .get_atom(&mime_type)
                    .map_err(|e| Error::platform("linux/x11: intern target atom", e))?
            };

            cb.store(selection_atom, target, data)
                .map_err(|e| Error::platform("linux/x11: write", e))
        })
        .await
    }

    pub(crate) async fn mime_types(&self) -> Result<Vec<String>, Error> {
        Ok(Vec::new())
    }
}
