# clipawl

A minimal, effective clipboard crate for Rust with a portable async API.

**Supported platforms:**
- **Web** (wasm32) via `navigator.clipboard`
- **Android** via JNI + ClipboardManager
- **Linux** via Wayland (wl-clipboard-rs) + X11 (clipboard_x11) with runtime detection

## Features

- **Async-first API** — works naturally with web's Promise-based clipboard
- **Platform detection** — automatically picks Wayland or X11 on Linux
- **Documented pitfalls** — explicit about Linux selection ownership, web permissions, etc.

## Quick Start

```rust
use clipawl::{Clipboard, Error};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let mut clipboard = Clipboard::new()?;

    // Write
    clipboard.set_text("Hello from clipawl!").await?;

    // Read
    let text = clipboard.get_text().await?;
    println!("Clipboard: {}", text);

    Ok(())
}
```

## Platform Notes

### Web (wasm32)

- Requires **secure context** (HTTPS or localhost)
- Requires **user activation** (click/keypress) for most browsers
- Not available in Web Workers

### Android

- Uses `ClipboardManager` via JNI
- `getPrimaryClip()` may return null if app lacks input focus or isn't the default IME
- Requires `ndk-context` to be initialized (handled by most Android frameworks)

### Linux

**Selection ownership model:** On X11 and Wayland, the app that sets the clipboard
often must continue serving the data. If your app exits immediately after `set_text()`,
the clipboard may appear empty to other apps.

**Workarounds:**
- Keep the `Clipboard` instance alive longer
- Use a clipboard manager (e.g., `clipman`, `wl-clipboard`)
- Future: `set_text_persistent()` (planned)

**Wayland:** Requires compositor support for `wlr-data-control` or `ext-data-control`
protocols. If unavailable, clipawl falls back to X11 (XWayland).

## Options

```rust
use clipawl::{Clipboard, ClipboardOptions, LinuxBackend, LinuxSelection};

let opts = ClipboardOptions {
    linux: LinuxOptions {
        selection: LinuxSelection::Primary,  // Use PRIMARY selection (middle-click paste)
        backend: LinuxBackend::X11,          // Force X11 backend
    },
};

let clipboard = Clipboard::new_with_options(opts)?;
```

## Cargo Features

- `linux-wayland` (default) — Enable Wayland backend
- `linux-x11` (default) — Enable X11 backend

Disable defaults to reduce dependencies:
```toml
clipawl = { version = "0.1", default-features = false, features = ["linux-x11"] }
```

## Roadmap

- [x] v0.1: Text read/write on Web, Android, Linux
- [ ] v0.2: Linux reliability knobs (wait-for-paste, better error messages)
- [ ] v0.3: Persistent copy for CLI workflows
- [ ] v0.4: Additional formats (HTML, image)

## License

Licensed under either of:
- MIT license ([LICENSE-MIT](LICENSE-MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

at your option.
