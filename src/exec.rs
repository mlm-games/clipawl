//! Internal, runtime-agnostic helper for running blocking platform calls
//! (JNI, X11 protocol round-trips, Wayland pipe reads) from async context.
//!
//! clipawl supports two mutually-exclusive execution backends for non-wasm
//! targets:
//!
//! - `tokio` (default): uses [`tokio::task::spawn_blocking`].
//! - `async-io`: uses the `blocking` crate's thread pool, the same one used
//!   by `async-io`/`smol`. Enable this instead of `tokio` if your
//!   application already drives its event loop with `async-io`/`smol`
//!   (e.g. via `accesskit_winit`'s default `async-io` feature) and you'd
//!   rather not pull in a full tokio runtime.
//!
//! If both features are enabled, `tokio` is used.

#[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
pub(crate) async fn unblock<T, F>(f: F) -> T
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    match tokio::task::spawn_blocking(f).await {
        Ok(v) => v,
        Err(e) => {
            if e.is_panic() {
                std::panic::resume_unwind(e.into_panic());
            } else {
                panic!("clipawl internal error: spawn_blocking join failed: {e}");
            }
        }
    }
}

#[cfg(all(
    feature = "async-io",
    not(feature = "tokio"),
    not(target_arch = "wasm32")
))]
pub(crate) async fn unblock<T, F>(f: F) -> T
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    blocking::unblock(f).await
}

#[cfg(all(
    not(feature = "tokio"),
    not(feature = "async-io"),
    not(target_arch = "wasm32")
))]
compile_error!(
    "clipawl requires exactly one runtime backend feature on this target: \
     enable either the `tokio` feature (default) or the `async-io` feature."
);
