pub(crate) fn local_navigation(url: &tauri::Url) -> bool {
    url.username().is_empty()
        && url.password().is_none()
        && url.port().is_none()
        && matches!(
            (url.scheme(), url.host_str()),
            ("tauri", Some("localhost")) | ("http" | "https", Some("tauri.localhost"))
        )
}

#[cfg(test)]
mod tests {
    use super::local_navigation;

    #[test]
    fn local_navigation_accepts_only_app_origins() {
        for url in [
            "tauri://localhost/index.html",
            "http://tauri.localhost/index.html",
            "https://tauri.localhost/index.html",
        ] {
            assert!(local_navigation(&url.parse().unwrap()), "{url}");
        }
        for url in [
            "https://example.com/",
            "https://tauri.localhost.example.com/",
            "http://tauri.localhost:8080/",
            "file:///tmp/example.html",
            "data:text/plain,example",
        ] {
            assert!(!local_navigation(&url.parse().unwrap()), "{url}");
        }
    }
}
