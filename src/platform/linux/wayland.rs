//! Wayland clipboard backend using wl-clipboard-rs.

use crate::{ClipboardOptions, Error, LinuxSelection};
use std::io::Read;
use std::thread::JoinHandle;
use wl_clipboard_rs::paste::ClipboardType;

pub(crate) struct WaylandClipboard {
    selection: LinuxSelection,
    serving: std::sync::Mutex<Option<JoinHandle<Result<(), Error>>>>,
    last_error: std::sync::Mutex<Option<Error>>,
}

impl WaylandClipboard {
    pub(crate) fn new(opts: &ClipboardOptions) -> Result<Self, Error> {
        // Probe Wayland availability by attempting a paste.
        // Only ClipboardEmpty / NoMimeType are acceptable (protocol works).
        // This is intentionally a side-effect-free check; it does not modify
        // clipboard contents or trigger a permission dialog on most compositors.
        use wl_clipboard_rs::paste::{ClipboardType, MimeType, Seat, get_contents};

        let result = get_contents(ClipboardType::Regular, Seat::Unspecified, MimeType::Text);
        match result {
            Ok(_)
            | Err(wl_clipboard_rs::paste::Error::ClipboardEmpty)
            | Err(wl_clipboard_rs::paste::Error::NoMimeType) => {}
            Err(e) => {
                return Err(Error::platform("linux/wayland: probe failed", e));
            }
        }

        Ok(Self {
            selection: opts.linux.selection,
            serving: std::sync::Mutex::new(None),
            last_error: std::sync::Mutex::new(None),
        })
    }

    fn clipboard_type(&self) -> ClipboardType {
        match self.selection {
            LinuxSelection::Clipboard => ClipboardType::Regular,
            LinuxSelection::Primary => ClipboardType::Primary,
        }
    }

    pub(crate) async fn mime_types(&self) -> Result<Vec<String>, Error> {
        use wl_clipboard_rs::paste::{Seat, get_mime_types};

        let ct = self.clipboard_type();
        tokio::task::block_in_place(|| {
            get_mime_types(ct, Seat::Unspecified)
                .map(|set| set.into_iter().collect())
                .map_err(|e| Error::platform("linux/wayland: get_mime_types", e))
        })
    }

    pub(crate) async fn read(&self, mime_type: &str) -> Result<Vec<u8>, Error> {
        use wl_clipboard_rs::paste::{Seat, get_contents};

        if let Some(e) = self.last_error.lock().unwrap().take() {
            log::warn!("clipawl wayland: previous write error: {}", e);
        }

        let mime = if mime_type == "text/plain" {
            wl_clipboard_rs::paste::MimeType::Text
        } else if mime_type.starts_with("text/") {
            wl_clipboard_rs::paste::MimeType::TextWithPriority(mime_type)
        } else {
            wl_clipboard_rs::paste::MimeType::Specific(mime_type)
        };

        let ct = self.clipboard_type();
        tokio::task::block_in_place(|| {
            let result = get_contents(ct, Seat::Unspecified, mime);

            match result {
                Ok((mut pipe, _)) => {
                    let mut buf = Vec::new();
                    pipe.read_to_end(&mut buf)
                        .map_err(|e| Error::platform("linux/wayland: read pipe", e))?;
                    Ok(buf)
                }
                Err(wl_clipboard_rs::paste::Error::NoSeats)
                | Err(wl_clipboard_rs::paste::Error::ClipboardEmpty)
                | Err(wl_clipboard_rs::paste::Error::NoMimeType) => Ok(Vec::new()),
                Err(e) => Err(Error::platform("linux/wayland: get_contents", e)),
            }
        })
    }

    pub(crate) async fn write(&self, mime_type: &str, data: &[u8]) -> Result<(), Error> {
        use wl_clipboard_rs::copy::{ClipboardType, Options, Source};

        if let Some(e) = self.last_error.lock().unwrap().take() {
            log::warn!("clipawl wayland: previous write error: {}", e);
        }

        let mime = if mime_type == "text/plain" {
            wl_clipboard_rs::copy::MimeType::Text
        } else {
            wl_clipboard_rs::copy::MimeType::Specific(mime_type.to_owned())
        };

        let bytes = data.to_vec();
        let ct = match self.selection {
            LinuxSelection::Clipboard => ClipboardType::Regular,
            LinuxSelection::Primary => ClipboardType::Primary,
        };

        let old = self.serving.lock().unwrap().take();
        if let Some(old) = old {
            tokio::task::block_in_place(|| match old.join() {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    log::warn!("clipawl wayland: previous write error: {}", e);
                }
                Err(_) => {
                    log::warn!("clipawl wayland: previous write thread panicked");
                }
            });
        }

        let handle = std::thread::spawn(move || {
            let mut opts = Options::new();
            opts.clipboard(ct);
            let result = opts
                .copy(Source::Bytes(bytes.into()), mime)
                .map_err(|e| Error::platform("linux/wayland: copy serve", e));
            if let Err(ref e) = result {
                log::warn!("clipawl wayland: copy serve error: {}", e);
            }
            result
        });

        *self.serving.lock().unwrap() = Some(handle);
        Ok(())
    }
}
