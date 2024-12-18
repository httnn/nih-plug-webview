# nih-plug-webview

**Warning: work in progress, not production-ready yet.**
Experimental webview editor for [nih-plug](https://github.com/robbert-vdh/nih-plug) using [wry](https://github.com/tauri-apps/wry).
Built on top of [baseview](https://github.com/RustAudio/baseview).

## Compatability

Currently, Windows and macOS are supported, and have been tentatively tested. Linux is minimally supported, with more full-fledged support in progress.

## Issues

On macOS there is an unresolved issue where pressing the escape key in Ableton Live will lead to a crash.
I've reported this to Ableton, and I'm currently mitigating this by consuming the escape keypress behind the scenes.

## Features

- Send arbitrary JSON values back and forth to the webview using Serde
- Resizable plug-in window
- Drag and drop files with full paths
- Callback for deciding which key events from DAW to consume
- Customisable background color for when the view is still loading (avoid initial flash of white)
- Use devtools

## Usage

There are two main ways to set up a project, depending on your preferences; you can use a web framework, such as React or Next.js, or write plain HTML/CSS/JS. The following sections will discuss some ways to set up a project using each method.

### Web Framework (Recommended)

**Note**: I'll provide basic examples using Next.js, since it's the framework I'm most familiar with. If you're not sure how to do something, always check your framework's documentation! (RTFM)

#### Prerequisites

- An [nih-plug](https://github.com/robbert-vdh/nih-plug) project
- A GUI directory for your web framework
  (read your framework's documentation on creating a project; For example, Next.js uses `npx create-next-app@latest`)
- Ensure that your web framework can output static content

#### Setup

First, configure your framework to output static content. Here's an example for Next.js.

```mjs
// next.config.mjs

/** @type {import('next').NextConfig} */
const nextConfig = {
  output: "export",
};

export default nextConfig;
```

In NIH-plug's `editor` function (in `impl Plugin`), you might want something like:

```rust
let size = (750, 500);
// for development, use localhost
let src = HTMLSource::URL("http://localhost:3000".to_owned());
let mut editor = WebViewEditor::new(src, size);

// for release, use static assets
#[cfg(not(debug_assertions))]
{
    editor = editor.with_asset_dir_protocol(
        include_dir!("path/to/gui/output/dir")
            .path(),
        None,
    );
}
// add any other editor configuration here:
editor = editor
    .with_developer_mode(true)
    // ...
```

**NOTE**: Make sure to replace the path(s) in the example!

### Plain HTML/CSS/JS

Similarly to using a web framework, you should probably have an existing NIH-plug project set up before starting with this.

If you want to write your GUI in plain HTML, CSS, and JS, there are a few options.

One option is to write and inline everything (_all_ CSS/JS scripts) in one HTML file and reference it with `HTMLSource::String`. Check out [the example](https://github.com/maxjvh/nih-plug-webview/blob/main/example/src/), which does this. This means you'll be writing and shipping all of your GUI code in one (massive) file.

If you want to write and ship separate HTML, JS, and CSS, files, you'll need to register a custom protocol.

One final option is to write separate files and use a bundler to **automatically** inline everything and ship one file. One of the best examples is with this [vite plugin](https://github.com/richardtallent/vite-plugin-singlefile)
