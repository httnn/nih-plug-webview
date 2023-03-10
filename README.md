# nih-plug-webview

Implements a webview editor for [nih-plug](https://github.com/robbert-vdh/nih-plug) using [a custom fork](https://github.com/maxjvh/wry) of [wry](https://github.com/tauri-apps/wry).
It currently works on macOS and Windows.

Contributions are welcome!

## Features
- send arbitrary JSON values back and forth between the webview using Serde
- resize the plug-in window
- drag and drop files with full paths
- specify intercepted keys with modifiers (not implemented for Windows yet)
- customisable background color for when the view is still loading (avoid initial flash of white)
- use devtools

## Usage

[Check out the example.](https://github.com/maxjvh/nih-plug-webview/blob/main/example/src/)

Build the example with `cargo xtask bundle gain` in the `example` folder.

## TODO
- [ ] Linux support (if at all possible)
- [ ] docs
- [ ] testing
