use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        mod web;
        pub(crate) use web::ClipboardImpl;
    } else if #[cfg(target_os = "android")] {
        mod android;
        pub(crate) use android::ClipboardImpl;
    } else if #[cfg(target_os = "linux")] {
        mod linux;
        pub(crate) use linux::ClipboardImpl;
    } else {
        mod unsupported;
        pub(crate) use unsupported::ClipboardImpl;
    }
}
