#![warn(clippy::all)]

//! # clipawl
//!
//! A minimal, effective clipboard crate for Rust with a portable async API.
//!
//! ## Supported Platforms
//!
//! - **Web (wasm32)** -> via `navigator.clipboard.readText/writeText`
//! - **Android** -> via JNI + ClipboardManager
//! - **Linux** -> via Wayland (wl-clipboard-rs) + X11 (clipboard_x11)
//!
//! ## Unsupported Platforms
//!
//! - **macOS** and **Windows** are not currently supported. On these platforms,
//!   `Clipboard::new()` returns `Err(Error::NotSupported)`. Contributions are welcome.
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
//! ## Feature Flags
//!
//! - `tokio` (default) -> run blocking platform calls via
//!   `tokio::task::spawn_blocking`. Use this if your app already runs a
//!   tokio runtime.
//! - `async-io` -> run blocking platform calls via the `blocking` crate's
//!   thread pool (the same one backing `async-io`/`smol`). Use this if
//!   you're on `async-io`/`smol` instead of tokio -> e.g. winit apps using
//!   `accesskit_winit`'s default `async-io` backend -> and want a smaller
//!   dependency footprint than pulling in tokio.
//! - `linux-wayland` / `linux-x11` (both default) -> enable the respective
//!   Linux backends.
//!
//! Exactly one of `tokio` / `async-io` must be enabled for non-wasm targets
//! (if both are enabled, `tokio` is used).
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
#[cfg(not(target_arch = "wasm32"))]
mod exec;
mod platform;

pub use error::Error;

/// Options for creating a clipboard handle.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct ClipboardOptions {
    /// Linux-specific options.
    pub linux: LinuxOptions,
}

/// Linux-specific clipboard options.
#[derive(Debug, Clone)]
#[non_exhaustive]
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
#[non_exhaustive]
pub enum LinuxSelection {
    /// CLIPBOARD selection (Ctrl+C/Ctrl+V).
    #[default]
    Clipboard,
    /// PRIMARY selection (mouse selection, middle-click paste).
    Primary,
}

/// Linux backend preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
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
///
/// **Thread safety:** `Clipboard` is `Send` and `Sync` on all supported
/// platforms (non-WASM targets use `Mutex` internally for shared state).
/// On WASM, `Clipboard` contains no JS handles and is trivially `Send + Sync`.
pub struct Clipboard {
    inner: platform::ClipboardImpl,
}

impl std::fmt::Debug for Clipboard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Clipboard").finish_non_exhaustive()
    }
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
    ///
    /// **Note:** Invalid UTF-8 bytes are replaced with U+FFFD (lossy).
    /// Use `read_as("text/plain")` for byte-exact round-trip fidelity.
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
    /// Returns `Err(Error::NotSupported)` if the platform does not support
    /// the requested MIME type at all.
    ///
    /// ## Supported MIME types per platform:
    /// - **Wayland**: any MIME type is accepted
    /// - **Web**: any MIME type the browser supports
    /// - **Android**: `text/plain` only
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
    ///
    /// **Web caveat:** On web, this calls `navigator.clipboard.read()`,
    /// which requires user activation and may trigger a permission prompt.
    /// It is not a cheap or side-effect-free query.
    pub async fn mime_types(&self) -> Result<Vec<String>, Error> {
        self.inner.mime_types().await
    }
}

/// Blocking API for non-wasm targets.
///
/// Works with either the `tokio` or `async-io` feature. Each call creates a
/// fresh `Clipboard` handle and drives the operation to completion on a
/// throwaway executor.
///
/// **Important (X11):** The handle is dropped at the end of each function,
/// which on X11 closes the display connection and relinquishes selection
/// ownership. After `write()` or `write_as()` returns, other apps may see
/// an empty clipboard. Keep a long-lived `Clipboard` instance (async) if
/// you need ownership to persist.
///
/// **Important (performance):** Each call creates a new clipboard handle.
/// On Linux this re-probes Wayland / reconnects X11 every time. For repeated
/// calls, prefer holding an async `Clipboard` across operations.
///
/// **Caveat (tokio):** Calling these functions from within a tokio async
/// context will panic (tokio disallows nested `block_on`). Use the async
/// `Clipboard` API directly inside async code.
#[cfg(any(feature = "tokio", feature = "async-io"))]
#[cfg(not(target_arch = "wasm32"))]
pub mod blocking {
    use super::*;

    #[cfg(feature = "tokio")]
    fn block_on<F: std::future::Future>(fut: F) -> Result<F::Output, Error> {
        // Panic early with a clear message if called from within tokio.
        if tokio::runtime::Handle::try_current().is_ok() {
            return Err(Error::Unavailable(
                "blocking API called from tokio async context; use the async Clipboard API instead",
            ));
        }
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| Error::Platform {
                context: "blocking: failed to create tokio runtime",
                source: Box::new(e),
            })?;
        Ok(rt.block_on(fut))
    }

    #[cfg(all(feature = "async-io", not(feature = "tokio")))]
    fn block_on<F: std::future::Future>(fut: F) -> Result<F::Output, Error> {
        Ok(async_io::block_on(fut))
    }

    /// Read text from the clipboard (blocking).
    ///
    /// See [module docs](self) for caveats about X11 selection ownership
    /// and repeated-call performance.
    pub fn read() -> Result<String, Error> {
        let clipboard = Clipboard::new()?;
        block_on(clipboard.read())?
    }

    /// Write text to the clipboard (blocking).
    ///
    /// See [module docs](self) for caveats about X11 selection ownership
    /// and repeated-call performance.
    pub fn write(text: &str) -> Result<(), Error> {
        let clipboard = Clipboard::new()?;
        let text = text.to_owned();
        block_on(async move { clipboard.write(&text).await })?
    }

    /// Read clipboard content in the given MIME type (blocking).
    ///
    /// See [module docs](self) for caveats about X11 selection ownership
    /// and repeated-call performance.
    pub fn read_as(mime_type: &str) -> Result<Vec<u8>, Error> {
        let clipboard = Clipboard::new()?;
        let mime_type = mime_type.to_owned();
        block_on(async move { clipboard.read_as(&mime_type).await })?
    }

    /// Write clipboard content in the given MIME type (blocking).
    ///
    /// See [module docs](self) for caveats about X11 selection ownership
    /// and repeated-call performance.
    pub fn write_as(mime_type: &str, data: &[u8]) -> Result<(), Error> {
        let clipboard = Clipboard::new()?;
        let mime_type = mime_type.to_owned();
        let data = data.to_vec();
        block_on(async move { clipboard.write_as(&mime_type, &data).await })?
    }

    /// List available MIME types on the clipboard (blocking).
    ///
    /// See [module docs](self) for caveats about X11 selection ownership
    /// and repeated-call performance.
    pub fn mime_types() -> Result<Vec<String>, Error> {
        let clipboard = Clipboard::new()?;
        block_on(clipboard.mime_types())?
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

    /// Writes binary data with a custom MIME type, reads it back, and verifies
    /// the MIME type appears in `mime_types()`.
    #[tokio::test]
    #[cfg(all(target_os = "linux", feature = "linux-wayland"))]
    async fn golden_read_write_as_roundtrip() {
        if std::env::var_os("WAYLAND_DISPLAY").is_none() {
            eprintln!("[golden] skipping: WAYLAND_DISPLAY not set");
            return;
        }

        let cb = Clipboard::new_with_options(ClipboardOptions {
            linux: LinuxOptions {
                backend: LinuxBackend::Wayland,
                ..Default::default()
            },
        })
        .expect("create Wayland clipboard");

        let mime = "application/x-clipawl-golden";
        let data = b"hello golden world";

        cb.write_as(mime, data).await.expect("write_as");

        // read_as - best-effort (wayland doesn't support self-paste)
        match cb.read_as(mime).await {
            Ok(contents) if contents.is_empty() => {
                eprintln!("[golden] read_as returned empty (self-paste unsupported)");
            }
            Ok(contents) => assert_eq!(contents, data, "round-trip data mismatch"),
            Err(e) => panic!("read_as error: {e}"),
        }

        match cb.mime_types().await {
            Ok(types) => assert!(
                types.iter().any(|t| t == mime),
                "expected '{mime}' in mime_types, got {types:?}",
            ),
            Err(e) => {
                eprintln!("[golden] mime_types failed (self-query unsupported): {e}");
            }
        }
    }
}
