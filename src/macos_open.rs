//! Open URLs in the user's default browser or mail client (macOS `open`).

/// Opens an `https://`, `mailto:`, or other URL via the system handler.
pub fn open_url(url: &str) {
    #[cfg(target_os = "macos")]
    {
        if url.is_empty() {
            return;
        }
        let _ = std::process::Command::new("open").arg(url).spawn();
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = url;
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn open_url_is_noop_off_macos() {
        super::open_url("https://example.com");
    }
}
