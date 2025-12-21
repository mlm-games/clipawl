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
//!     let mut clipboard = Clipboard::new()?;
//!     clipboard.set_text("Hello!").await?;
//!     let text = clipboard.get_text().await?;
//!     println!("{}", text);
//!     Ok(())
//! }
//! ```

mod error;
mod options;
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
/// `set_text()` may cause the clipboard to appear empty.
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
    /// Returns an empty string if the clipboard is empty or has no text
    /// representation (browser behavior varies).
    pub async fn get_text(&mut self) -> Result<String, Error> {
        self.inner.get_text().await
    }

    /// Write text to the clipboard.
    pub async fn set_text(&mut self, text: &str) -> Result<(), Error> {
        self.inner.set_text(text).await
    }
}

/// Blocking API for non-wasm targets.
#[cfg(not(target_arch = "wasm32"))]
pub mod blocking {
    use super::*;

    /// Read text from the clipboard (blocking).
    pub fn get_text() -> Result<String, Error> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| Error::Platform {
                context: "blocking: failed to create runtime",
                source: Box::new(e),
            })?;
        let mut clipboard = Clipboard::new()?;
        rt.block_on(clipboard.get_text())
    }

    /// Write text to the clipboard (blocking).
    pub fn set_text(text: &str) -> Result<(), Error> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| Error::Platform {
                context: "blocking: failed to create runtime",
                source: Box::new(e),
            })?;
        let mut clipboard = Clipboard::new()?;
        rt.block_on(clipboard.set_text(text))
    }
}
