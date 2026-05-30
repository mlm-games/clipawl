//! Linux clipboard implementation with Wayland + X11 runtime detection.

use crate::{ClipboardOptions, Error, LinuxBackend};

#[cfg(feature = "linux-wayland")]
mod wayland;

#[cfg(feature = "linux-x11")]
mod x11;

pub(crate) struct ClipboardImpl {
    inner: Inner,
}

enum Inner {
    #[cfg(feature = "linux-wayland")]
    Wayland(wayland::WaylandClipboard),

    #[cfg(feature = "linux-x11")]
    X11(Box<x11::X11Clipboard>),

    #[cfg(not(any(feature = "linux-wayland", feature = "linux-x11")))]
    None,
}

impl ClipboardImpl {
    pub(crate) fn new(opts: &ClipboardOptions) -> Result<Self, Error> {
        let backend_pref = opts.linux.backend;

        match backend_pref {
            LinuxBackend::Wayland => {
                #[cfg(feature = "linux-wayland")]
                {
                    Self::try_wayland()
                }
                #[cfg(not(feature = "linux-wayland"))]
                {
                    return Err(Error::NotSupported);
                }
            }
            LinuxBackend::X11 => {
                #[cfg(feature = "linux-x11")]
                {
                    Self::try_x11(opts)
                }
                #[cfg(not(feature = "linux-x11"))]
                {
                    return Err(Error::NotSupported);
                }
            }
            LinuxBackend::Auto => {
                // Try Wayland first (if WAYLAND_DISPLAY is set), then X11
                #[cfg(feature = "linux-wayland")]
                if std::env::var_os("WAYLAND_DISPLAY").is_some() {
                    if let Ok(cb) = Self::try_wayland() {
                        log::debug!("clipawl: using Wayland backend");
                        return Ok(cb);
                    }
                }

                #[cfg(feature = "linux-x11")]
                if std::env::var_os("DISPLAY").is_some() {
                    if let Ok(cb) = Self::try_x11(opts) {
                        log::debug!("clipawl: using X11 backend");
                        return Ok(cb);
                    }
                }

                Err(Error::NotSupported)
            }
        }
    }

    #[cfg(feature = "linux-wayland")]
    fn try_wayland() -> Result<Self, Error> {
        let wl = wayland::WaylandClipboard::new()?;
        Ok(Self {
            inner: Inner::Wayland(wl),
        })
    }

    #[cfg(feature = "linux-x11")]
    fn try_x11(opts: &ClipboardOptions) -> Result<Self, Error> {
        let x11 = x11::X11Clipboard::new(opts)?;
        Ok(Self {
            inner: Inner::X11(Box::new(x11)),
        })
    }

    pub(crate) async fn get_text(&self) -> Result<String, Error> {
        match &self.inner {
            #[cfg(feature = "linux-wayland")]
            Inner::Wayland(w) => w.get_text().await,

            #[cfg(feature = "linux-x11")]
            Inner::X11(x) => x.get_text().await,

            #[cfg(not(any(feature = "linux-wayland", feature = "linux-x11")))]
            Inner::None => Err(Error::NotSupported),
        }
    }

    pub(crate) async fn set_text(&self, text: &str) -> Result<(), Error> {
        match &self.inner {
            #[cfg(feature = "linux-wayland")]
            Inner::Wayland(w) => w.set_text(text).await,

            #[cfg(feature = "linux-x11")]
            Inner::X11(x) => x.set_text(text).await,

            #[cfg(not(any(feature = "linux-wayland", feature = "linux-x11")))]
            Inner::None => Err(Error::NotSupported),
        }
    }
}
