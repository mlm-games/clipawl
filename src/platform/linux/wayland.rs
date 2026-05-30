//! Wayland clipboard backend using wl-clipboard-rs.

use crate::Error;
use std::io::Read;
use std::thread::JoinHandle;

pub(crate) struct WaylandClipboard {
    /// Handle to the serving thread (for write).
    /// Uses interior mutability so Clipboard methods can take &self.
    serving: std::sync::Mutex<Option<JoinHandle<()>>>,
}

impl WaylandClipboard {
    pub(crate) fn new() -> Result<Self, Error> {
        // Probe Wayland availability by attempting a paste.
        // Only ClipboardEmpty / NoMimeType are acceptable (protocol works).
        use wl_clipboard_rs::paste::{ClipboardType, MimeType, Seat, get_contents};

        let result = get_contents(ClipboardType::Regular, Seat::Unspecified, MimeType::Text);
        match result {
            Ok(_)
            | Err(wl_clipboard_rs::paste::Error::ClipboardEmpty)
            | Err(wl_clipboard_rs::paste::Error::NoMimeType) => {}
            Err(e) => {
                // NoSeats or connection errors → Wayland unavailable
                return Err(Error::platform("linux/wayland: probe failed", e));
            }
        }

        Ok(Self {
            serving: std::sync::Mutex::new(None),
        })
    }

    pub(crate) async fn mime_types(&self) -> Result<Vec<String>, Error> {
        use wl_clipboard_rs::paste::{ClipboardType, Seat, get_mime_types};

        tokio::task::block_in_place(|| {
            get_mime_types(ClipboardType::Regular, Seat::Unspecified)
                .map(|set| set.into_iter().collect())
                .map_err(|e| Error::platform("linux/wayland: get_mime_types", e))
        })
    }

    pub(crate) async fn read(&self, mime_type: &str) -> Result<Vec<u8>, Error> {
        use wl_clipboard_rs::paste::{ClipboardType, Seat, get_contents};

        let mime = if mime_type == "text/plain" {
            wl_clipboard_rs::paste::MimeType::Text
        } else if mime_type.starts_with("text/") {
            wl_clipboard_rs::paste::MimeType::TextWithPriority(mime_type)
        } else {
            wl_clipboard_rs::paste::MimeType::Specific(mime_type)
        };

        tokio::task::block_in_place(|| {
            let result = get_contents(ClipboardType::Regular, Seat::Unspecified, mime);

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
        use wl_clipboard_rs::copy::{Options, Source};

        let mime = if mime_type == "text/plain" {
            wl_clipboard_rs::copy::MimeType::Text
        } else {
            wl_clipboard_rs::copy::MimeType::Specific(mime_type.to_owned())
        };

        let bytes = data.to_vec();

        let old = self.serving.lock().unwrap().take();
        if let Some(old) = old {
            tokio::task::block_in_place(|| {
                let _ = old.join();
            });
        }

        let handle = std::thread::spawn(move || {
            let opts = Options::new();
            if let Err(e) = opts.copy(Source::Bytes(bytes.into()), mime) {
                log::warn!("clipawl wayland: copy serve error: {}", e);
            }
        });

        *self.serving.lock().unwrap() = Some(handle);
        Ok(())
    }
}
