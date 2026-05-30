//! Wayland clipboard backend using wl-clipboard-rs.

use crate::Error;
use std::io::Read;
use std::thread::JoinHandle;

pub(crate) struct WaylandClipboard {
    /// Handle to the serving thread (for set_text).
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

    pub(crate) async fn get_text(&self) -> Result<String, Error> {
        use wl_clipboard_rs::paste::{ClipboardType, MimeType, Seat, get_contents};

        tokio::task::block_in_place(|| {
            let result = get_contents(ClipboardType::Regular, Seat::Unspecified, MimeType::Text);

            match result {
                Ok((mut pipe, _mime)) => {
                    let mut buf = Vec::new();
                    pipe.read_to_end(&mut buf)
                        .map_err(|e| Error::platform("linux/wayland: read pipe", e))?;
                    Ok(String::from_utf8_lossy(&buf).into_owned())
                }
                Err(wl_clipboard_rs::paste::Error::NoSeats)
                | Err(wl_clipboard_rs::paste::Error::ClipboardEmpty)
                | Err(wl_clipboard_rs::paste::Error::NoMimeType) => Ok(String::new()),
                Err(e) => Err(Error::platform("linux/wayland: get_contents", e)),
            }
        })
    }

    pub(crate) async fn set_text(&self, text: &str) -> Result<(), Error> {
        use wl_clipboard_rs::copy::{MimeType, Options, Source};

        let bytes: Vec<u8> = text.as_bytes().to_vec();

        // Wait for previous serving thread to finish (compositor will have
        // cancelled its data source when the new offer was made).
        let old = self.serving.lock().unwrap().take();
        if let Some(old) = old {
            tokio::task::block_in_place(|| {
                let _ = old.join();
            });
        }

        // Spawn a thread to serve the clipboard.
        let handle = std::thread::spawn(move || {
            let opts = Options::new();
            if let Err(e) = opts.copy(Source::Bytes(bytes.into()), MimeType::Text) {
                log::warn!("clipawl wayland: copy serve error: {}", e);
            }
        });

        *self.serving.lock().unwrap() = Some(handle);
        Ok(())
    }
}
