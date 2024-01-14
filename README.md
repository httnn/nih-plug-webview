# nih-plug-webview

**Warning: work in progress, not production-ready yet.**
Implements a webview editor for [nih-plug](https://github.com/robbert-vdh/nih-plug) using [a custom fork](https://github.com/maxjvh/wry) of [wry](https://github.com/tauri-apps/wry).
Built on top of [baseview](https://github.com/RustAudio/baseview).

## Current status

I've only been able to test this on macOS so far on which it has been working quite robustly.
Other contributors have occasionally helped with the Windows side but unfortunately I currently have no idea if it really works or not.
It would be great to get Windows support up to par, but someone familiar with Windows APIs would be needed to work on that.

On macOS there is an unresolved issue where pressing the escape key in Ableton Live will lead to a crash.
I've reported this to Ableton, and I'm currently mitigating this by consuming the escape keypress behind the scenes.

## Features
- send arbitrary JSON values back and forth to the webview using Serde
- resizable plug-in window
- drag and drop files with full paths
- callback for deciding which key events from DAW to consume 
- customisable background color for when the view is still loading (avoid initial flash of white)
- use devtools

## Usage

[Check out the example.](https://github.com/maxjvh/nih-plug-webview/blob/main/example/src/)

Build the example with `cargo xtask bundle gain` in the `example` folder.
