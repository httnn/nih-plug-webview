# nih-plug-webview

**Warning: work in progress, not production-ready yet.**
Implements a webview editor for [nih-plug](https://github.com/robbert-vdh/nih-plug) using [a custom fork](https://github.com/maxjvh/wry) of [wry](https://github.com/tauri-apps/wry).

## Current status

I've only been able to test the webview on macOS so far, and while some contributors have helped with the Windows side, it still requires a bit more love (and someone more familiar with Windows API's, so **help is needed**).

On macOS there is an unresolved issue where pressing the escape key in Ableton Live will lead to a crash.
This can be mitigated by consuming the escape key in `performKeyEquivalent` but that's not a real fix to the underlying issue.
The macOS leak checker also indicates that some `WKWebView` related resources are leaking, which might be related (or not).
All of this might be caused by the `wry` fork not using `tao` for windowing, but this would need to be investigated further.

## Features
- send arbitrary JSON values back and forth to the webview using Serde
- resizable plug-in window
- drag and drop files with full paths
- specify list of keys and modifiers to be consumed (not implemented for Windows yet)
- customisable background color for when the view is still loading (avoid initial flash of white)
- use devtools

## Usage

[Check out the example.](https://github.com/maxjvh/nih-plug-webview/blob/main/example/src/)

Build the example with `cargo xtask bundle gain` in the `example` folder.
