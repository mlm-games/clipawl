//! Demonstrates using clipawl with the `async-io` feature instead of tokio,
//! e.g. for apps built around winit + AccessKit's `async-io`/`smol` backend.
//!
//! Run with: `cargo run --example async_io --no-default-features \
//!     --features "async-io,linux-wayland,linux-x11"`

fn main() -> Result<(), clipawl::Error> {
    async_io::block_on(async {
        let clipboard = clipawl::Clipboard::new()?;
        clipboard.write("hello from async-io").await?;
        let text = clipboard.read().await?;
        println!("{text}");
        Ok(())
    })
}
