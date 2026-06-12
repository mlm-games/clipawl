# clipawl

A clipboard crate for Rust with a portable async API.

**Supported platforms:**
- **Web** (wasm32) via `navigator.clipboard`
- **Android** via JNI + ClipboardManager
- **Linux** via Wayland (wl-clipboard-rs) + X11 (clipboard_x11) with runtime detection

## Features

- **Async-first API** — works naturally with web's Promise-based clipboard
- **Platform detection** — automatically picks Wayland or X11 on Linux
- **Arbitrary MIME types** — read/write any format via `read_as("image/png")` etc.
- **Bare `read()`/`write()` defaults to text** — no MIME type needed for plain text
- **Documented pitfalls** — explicit about Linux selection ownership, web permissions, etc.

## Quick Start

```rust
use clipawl::{Clipboard, Error};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let clipboard = Clipboard::new()?;

    clipboard.write("Hello from clipawl!").await?;
    let text = clipboard.read().await?;
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
often must continue serving the data. If your app exits immediately after `write()`,
the clipboard may appear empty to other apps.

**Workarounds:**
- Keep the `Clipboard` instance alive longer
- Use a clipboard manager (e.g., `clipman`, `wl-clipboard`)

**Wayland:** Requires compositor support for `wlr-data-control` or `ext-data-control`
protocols. If unavailable, clipawl falls back to X11 (XWayland).

## Options

```rust
use clipawl::{Clipboard, ClipboardOptions, LinuxBackend, LinuxOptions, LinuxSelection};

let opts = ClipboardOptions {
    linux: LinuxOptions {
        selection: LinuxSelection::Primary,  // Use PRIMARY selection (middle-click paste)
        backend: LinuxBackend::X11,          // Force X11 backend
    },
};

let clipboard = Clipboard::new_with_options(opts)?;
```

## Arbitrary Formats

Read and write any MIME type via `read_as()` / `write_as()`:

```rust
use clipawl::{Clipboard, Error};

async fn example(clipboard: &Clipboard) -> Result<(), Error> {
    // PNG images
    clipboard.write_as("image/png", &png_bytes).await?;
    let png = clipboard.read_as("image/png").await?;

    // Any custom MIME type
    clipboard.write_as("text/uri-list", b"https://example.com").await?;
    Ok(())
}
```

**Platform support:**
| MIME type        | Wayland | Web | Android | X11 |
|------------------|---------|-----|---------|-----|
| `text/plain`     | ✅      | ✅  | ✅      | ✅  |
| `*/*` (any)      | ✅      | ✅  | ❌       | ❌   |

## Cargo Features

- `linux-wayland` (default) — Enable Wayland backend
- `linux-x11` (default) — Enable X11 backend

Disable defaults to reduce dependencies:
```toml
clipawl = { version = "0.3", default-features = false, features = ["linux-x11"] }
```

## License

Licensed under either of:
- MIT license ([LICENSE-MIT](LICENSE-MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

at your option.
