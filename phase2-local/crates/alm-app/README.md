# alm-app (Tauri development trial)

Rust renders every screen's HTML from `alm-core` results; `ui/glue.js` only sends the
clicked action and form fields and shows the returned HTML. Screens read the seeded
synthetic fixture (`alm-core` feature `demo-fixture`) and are development-only.

```sh
cargo run --locked --offline -p alm-app
cargo tauri android build --debug --apk --target aarch64
```

## Generic glue

`ui/glue.js` has no screen code. Rust markup drives four generic behaviours:
`<template id="screen-modal">` opens the overlay (its × carries the closing command);
`form[data-live]` sends its fields plus `_changed` after typing and replaces the
`data-target` fragment; `input[data-search]` lists Rust options whose `data-fill` query
string fills the form (`data-submit` submits an intent); `button[data-copy]` writes the
command's text to the clipboard. Tests: `node --test tests/*.test.cjs`.

## Platform notes

- **Android safe area.** The system-bar and display-cutout padding is applied natively in
  `gen/android/app/src/main/java/in/alm/trial/MainActivity.kt` (a
  `setOnApplyWindowInsetsListener` on the content view that pads it and passes the remaining
  insets, including the IME, to the WebView). `tauri android init` regenerates `gen/android`
  and overwrites this file: reapply the listener after any regeneration and re-check that the
  header clears the status bar and the footer clears the navigation bar.
- **Android printing and sharing** go through the same `MainActivity.kt` (`AlmAndroid.print`
  via `PrintManager`, `AlmAndroid.share` via `ACTION_SEND`); they are lost on regeneration too.
- **macOS printing.** WKWebView ignores `window.print()`, so the page calls the
  `print_page` command (`WebviewWindow::print()`).
- **Device date.** The page sends the device's local calendar date once at start (`start`
  command); Rust parses it and owns every date rule.
