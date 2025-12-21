//! Wayland clipboard backend using wl-clipboard-rs.

use crate::Error;
use std::io::Read;
use std::thread::JoinHandle;

pub(crate) struct WaylandClipboard {
    /// Handle to the serving thread (for set_text).
    /// Dropping this detaches the thread, which is fine — when another app
    /// takes clipboard ownership, the serve operation ends.
    _serving: Option<JoinHandle<()>>,
}

impl WaylandClipboard {
    pub(crate) fn new() -> Result<Self, Error> {
        // Probe Wayland availability by attempting a paste
        // (this will fail fast if protocols are unavailable)
        use wl_clipboard_rs::paste::{get_contents, ClipboardType, MimeType, Seat};

        // Don't fail on "empty clipboard", only on "can't connect"
        let _ = get_contents(ClipboardType::Regular, Seat::Unspecified, MimeType::Text);

        Ok(Self { _serving: None })
    }

    pub(crate) async fn get_text(&mut self) -> Result<String, Error> {
        use wl_clipboard_rs::paste::{get_contents, ClipboardType, MimeType, Seat};

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
            | Err(wl_clipboard_rs::paste::Error::NoMimeType) => {
                // Treat as empty clipboard
                Ok(String::new())
            }
            Err(e) => Err(Error::platform("linux/wayland: get_contents", e)),
        }
    }

    pub(crate) async fn set_text(&mut self, text: &str) -> Result<(), Error> {
        use wl_clipboard_rs::copy::{MimeType, Options, Source};

        let bytes: Vec<u8> = text.as_bytes().to_vec();

        // Spawn a thread to serve the clipboard.
        // The wl-clipboard-rs copy operation blocks while serving requests
        // until another app takes ownership.
        let handle = std::thread::spawn(move || {
            let opts = Options::new();
            if let Err(e) = opts.copy(Source::Bytes(bytes.into()), MimeType::Text) {
                log::warn!("clipawl wayland: copy serve error: {}", e);
            }
        });

        self._serving = Some(handle);
        Ok(())
    }
}
