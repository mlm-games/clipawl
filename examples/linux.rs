//! Example: read and write clipboard on Linux.

use clipawl::{Clipboard, Error};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let clipboard = Clipboard::new()?;

    // Read current clipboard
    match clipboard.read().await {
        Ok(text) if text.is_empty() => println!("Clipboard is empty"),
        Ok(text) => println!("Current clipboard: {}", text),
        Err(e) => println!("Could not read clipboard: {}", e),
    }

    // Write new content
    let new_text = "Hello from clipawl!";
    clipboard.write(new_text).await?;
    println!("Set clipboard to: {}", new_text);

    // Read back
    let read_back = clipboard.read().await?;
    println!("Read back: {}", read_back);

    // Keep alive for a moment so the clipboard can be served
    println!("Keeping clipboard alive for 5 seconds...");
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    Ok(())
}
