use clipawl::Clipboard;

fn init_logger() {
    let _ = env_logger::try_init();
}

#[tokio::main]
async fn main() -> Result<(), clipawl::Error> {
    init_logger();

    let cb = Clipboard::new()?;

    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><circle cx="50" cy="50" r="40"/></svg>"#;

    cb.write_as("image/svg+xml", svg.as_bytes()).await?;
    println!("wrote image/svg+xml ({} bytes)", svg.len());

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let result = cb.read_as("image/svg+xml").await?;
    let text = String::from_utf8_lossy(&result);
    println!("read back ({} bytes): {}", result.len(), text);

    Ok(())
}
