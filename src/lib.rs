//! # clipawl
//!
//! A minimal, effective clipboard crate for Rust with a portable async API.
//!
//! ## Supported Platforms
//!
//! - **Web (wasm32)** — via `navigator.clipboard.readText/writeText`
//! - **Android** — via JNI + ClipboardManager
//! - **Linux** — via Wayland (wl-clipboard-rs) + X11 (clipboard_x11)
//!
//! ## Platform Caveats
//!
//! ### Web
//! - Requires secure context (HTTPS) and user activation (click/keypress)
//! - Methods are async (Promise-based under the hood)
//! - Not available in Web Workers
//!
//! ### Android
//! - `getPrimaryClip()` may return null if app lacks focus or isn't the default IME
//!
//! ### Linux
//! - **Selection ownership**: the setting app often must keep serving data.
//!   If your process exits immediately, clipboard may appear empty.
//! - **Wayland**: requires compositor support for data-control protocols.
//!   Falls back to X11 (XWayland) if unavailable.
//!
//! ## Example
//!
//! ```rust,no_run
//! use clipawl::{Clipboard, Error};
//!
//! async fn example() -> Result<(), Error> {
//!     let clipboard = Clipboard::new()?;
//!     clipboard.write("Hello!").await?;
//!     let text = clipboard.read().await?;
//!     println!("{}", text);
//!     Ok(())
//! }
//! ```

mod error;
mod platform;

pub use error::Error;

/// Options for creating a clipboard handle.
#[derive(Debug, Clone, Default)]
pub struct ClipboardOptions {
    /// Linux-specific options.
    pub linux: LinuxOptions,
}

/// Linux-specific clipboard options.
#[derive(Debug, Clone)]
pub struct LinuxOptions {
    /// Which X11 selection to use.
    pub selection: LinuxSelection,
    /// Backend preference.
    pub backend: LinuxBackend,
}

impl Default for LinuxOptions {
    fn default() -> Self {
        Self {
            selection: LinuxSelection::Clipboard,
            backend: LinuxBackend::Auto,
        }
    }
}

/// X11 selection type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LinuxSelection {
    /// CLIPBOARD selection (Ctrl+C/Ctrl+V).
    #[default]
    Clipboard,
    /// PRIMARY selection (mouse selection, middle-click paste).
    Primary,
}

/// Linux backend preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LinuxBackend {
    /// Try Wayland first, fall back to X11.
    #[default]
    Auto,
    /// Force Wayland backend.
    Wayland,
    /// Force X11 backend.
    X11,
}

/// A cross-platform clipboard handle.
///
/// **Important (Linux):** Keep this alive while you expect clipboard data to be
/// available. Due to selection ownership semantics, dropping this too soon after
/// `write()` may cause the clipboard to appear empty.
pub struct Clipboard {
    inner: platform::ClipboardImpl,
}

impl Clipboard {
    /// Create a new clipboard handle with default options.
    pub fn new() -> Result<Self, Error> {
        Self::new_with_options(ClipboardOptions::default())
    }

    /// Create a new clipboard handle with custom options.
    pub fn new_with_options(opts: ClipboardOptions) -> Result<Self, Error> {
        Ok(Self {
            inner: platform::ClipboardImpl::new(&opts)?,
        })
    }

    /// Read text from the clipboard.
    ///
    /// Shortcut for `read_as("text/plain")`. Returns an empty string if no
    /// text content is available.
    pub async fn read(&self) -> Result<String, Error> {
        let buf = self.inner.read("text/plain").await?;
        Ok(String::from_utf8_lossy(&buf).into_owned())
    }

    /// Write text to the clipboard.
    ///
    /// Shortcut for `write_as("text/plain", text.as_bytes())`.
    pub async fn write(&self, text: &str) -> Result<(), Error> {
        self.inner.write("text/plain", text.as_bytes()).await
    }

    /// Read clipboard content in the given MIME type.
    ///
    /// Returns an empty `Vec` if no content is available in that type.
    ///
    /// ## Supported MIME types per platform:
    /// - **Wayland**: any MIME type is accepted
    /// - **Web**: any MIME type the browser supports
    /// - **Android**: `text/plain`
    /// - **X11**: `text/plain` only
    pub async fn read_as(&self, mime_type: &str) -> Result<Vec<u8>, Error> {
        self.inner.read(mime_type).await
    }

    /// Write clipboard content in the given MIME type.
    pub async fn write_as(&self, mime_type: &str, data: &[u8]) -> Result<(), Error> {
        self.inner.write(mime_type, data).await
    }

    /// List available MIME types currently on the clipboard.
    ///
    /// Returns an empty `Vec` if the clipboard is empty or if the platform
    /// does not support format enumeration.
    pub async fn mime_types(&self) -> Result<Vec<String>, Error> {
        self.inner.mime_types().await
    }
}

/// Blocking API for non-wasm targets.
#[cfg(not(target_arch = "wasm32"))]
pub mod blocking {
    use super::*;

    fn new_runtime() -> Result<tokio::runtime::Runtime, Error> {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| Error::Platform {
                context: "blocking: failed to create runtime",
                source: Box::new(e),
            })
    }

    /// Read text from the clipboard (blocking).
    pub fn read() -> Result<String, Error> {
        let rt = new_runtime()?;
        let clipboard = Clipboard::new()?;
        rt.block_on(clipboard.read())
    }

    /// Write text to the clipboard (blocking).
    pub fn write(text: &str) -> Result<(), Error> {
        let rt = new_runtime()?;
        let clipboard = Clipboard::new()?;
        let text = text.to_owned();
        rt.block_on(clipboard.write(&text))
    }

    /// Read clipboard content in the given MIME type (blocking).
    pub fn read_as(mime_type: &str) -> Result<Vec<u8>, Error> {
        let rt = new_runtime()?;
        let clipboard = Clipboard::new()?;
        let mime_type = mime_type.to_owned();
        rt.block_on(clipboard.read_as(&mime_type))
    }

    /// Write clipboard content in the given MIME type (blocking).
    pub fn write_as(mime_type: &str, data: &[u8]) -> Result<(), Error> {
        let rt = new_runtime()?;
        let clipboard = Clipboard::new()?;
        let mime_type = mime_type.to_owned();
        let data = data.to_vec();
        rt.block_on(clipboard.write_as(&mime_type, &data))
    }

    /// List available MIME types on the clipboard (blocking).
    pub fn mime_types() -> Result<Vec<String>, Error> {
        let rt = new_runtime()?;
        let clipboard = Clipboard::new()?;
        rt.block_on(clipboard.mime_types())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clipboard_options_default() {
        let opts = ClipboardOptions::default();
        assert_eq!(opts.linux.selection, LinuxSelection::Clipboard);
        assert_eq!(opts.linux.backend, LinuxBackend::Auto);
    }

    #[test]
    fn linux_selection_default() {
        assert_eq!(LinuxSelection::default(), LinuxSelection::Clipboard);
    }

    #[test]
    fn linux_backend_default() {
        assert_eq!(LinuxBackend::default(), LinuxBackend::Auto);
    }

    #[test]
    fn error_display_not_supported() {
        assert_eq!(
            Error::NotSupported.to_string(),
            "clipboard not supported on this platform"
        );
    }

    #[test]
    fn error_display_permission_denied() {
        assert_eq!(
            Error::PermissionDenied("test reason").to_string(),
            "clipboard permission denied: test reason"
        );
    }

    #[test]
    fn error_display_unavailable() {
        assert_eq!(
            Error::Unavailable("empty").to_string(),
            "clipboard unavailable: empty"
        );
    }

    #[test]
    fn error_display_platform() {
        let inner = Error::platform("test ctx", std::io::Error::other("boom"));
        assert_eq!(inner.to_string(), "test ctx: boom");
    }

    #[test]
    fn error_is_std_error() {
        use std::error::Error as _;
        let e = Error::NotSupported;
        assert!(e.source().is_none());

        let e = Error::platform("ctx", std::io::Error::other("x"));
        assert!(e.source().is_some());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn blocking_runtime_construction() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();
        assert!(rt.is_ok());
    }
}
