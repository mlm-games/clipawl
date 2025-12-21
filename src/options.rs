use std::time::Duration;

/// Options for `set_text_with`.
///
/// Cross-platform rule: unsupported fields are ignored on platforms where they
/// can’t apply.
///
/// - Linux: `wait_for_paste` / `wait_timeout` supported (selection ownership model).
/// - Linux + Android: `sensitive` is supported.
/// - Web: options are currently ignored (browser security model dominates).
#[derive(Clone, Debug, Default)]
pub struct SetTextOptions {
    /// On Linux, wait until another app has requested (and thus captured) the clipboard
    /// contents (or until timeout).
    ///
    /// Useful for short-lived CLI programs.
    pub wait_for_paste: bool,

    /// If set, bound `wait_for_paste` by this timeout.
    pub wait_timeout: Option<Duration>,

    /// Mark clipboard content as sensitive.
    ///
    /// - Linux: hints clipboard managers not to store in history.
    /// - Android: marks copied content as sensitive so the system UI avoids previews.
    pub sensitive: bool,
}
