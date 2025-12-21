//! Example: blocking API (non-wasm only).

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use clipawl::blocking;

        // Read
        match blocking::get_text() {
            Ok(text) => println!("Clipboard: {}", text),
            Err(e) => println!("Error: {}", e),
        }

        // Write
        if let Err(e) = blocking::set_text("Hello from blocking API!") {
            println!("Error: {}", e);
        } else {
            println!("Clipboard set!");
        }
    }

    #[cfg(target_arch = "wasm32")]
    {
        println!("Blocking API not available on wasm32");
    }
}
