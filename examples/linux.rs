//! Example: read and write clipboard on Linux.

use clipawl::{Clipboard, Error};

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Read current clipboard
    let mut clipboard = Clipboard::new()?;

    match clipboard.get_text().await {
        Ok(text) if text.is_empty() => println!("Clipboard is empty"),
        Ok(text) => println!("Current clipboard: {}", text),
        Err(e) => println!("Could not read clipboard: {}", e),
    }

    // Write new content
    let new_text = "Hello from clipawl!";
    clipboard.set_text(new_text).await?;
    println!("Set clipboard to: {}", new_text);

    // Read back
    let read_back = clipboard.get_text().await?;
    println!("Read back: {}", read_back);

    // Keep alive for a moment so the clipboard can be served
    println!("Keeping clipboard alive for 5 seconds...");
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    Ok(())
}
