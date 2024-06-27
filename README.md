# nih-plug-webview

**Warning: work in progress, not production-ready yet.**
Experimental webview editor for [nih-plug](https://github.com/robbert-vdh/nih-plug) using [wry](https://github.com/tauri-apps/wry).
Built on top of [baseview](https://github.com/RustAudio/baseview).

## Platform Support

This project has been tested and works on Windows and macOS. 

## Issues

- On macOS there is an unresolved issue where pressing the escape key in Ableton Live will lead to a crash. 

## Features
- Send arbitrary JSON values back and forth to the webview using Serde
- Resizable plug-in window
- Drag and drop files with full paths
- Callback for deciding which key events from DAW to consume 
- Customisable background color for when the view is still loading (avoid initial flash of white)
- Use devtools

## Usage

### With a Web Framework
(Note: this section is basically just a huge extrapolation of [this discussion post](https://github.com/httnn/nih-plug-webview/discussions/10)).

Essentially every modern web framework should be supported *at least for development* (or if you just want to try this out). For this, you can use a [URL Source](https://github.com/httnn/nih-plug-webview/blob/32e10ccbcf90c8345a8ce3c53c0445fae03c3caa/src/lib.rs#L43C3-L43C22) corresponding to your dev server. **This will also support hot reloading on the frontend** for rapid GUI development.


In a production/release environment, things get more complicated because a dev server isn't available on user/client machines. So, the only option is **export the frontend to static content.** (By the way, if your framework somehow doesn't support exporting to static content, you probably won't be able to do this) 

This leaves two options:
1. Request assets with a custom protocol (Recommended)
2. Inline *everything* into one HTML file

#### 1. Request Assets with a Custom Protocol (Recommended)
**WARNING: Your framework may or may not be supported.**
This method involves configuring your framework's bundler to link assets with a custom path, that being a custom protocol.

| Framework     | Support      | Method   |
| ------------- | ------------- |---|
| [CRA](https://create-react-app.dev/) (React) | ✔️ | Add this to `.env`: `PUBLIC_URL = <protocol_url>` (see below for more info)  |  

`protocol_url` *is platform-dependent*:
- Windows: `http://<protocol name>.localhost/`
(Windows is the only platform I've tested)

After you've configured your framework/bundler to export with a custom protocol, you'll need to register the protocol in Rust.

If you don't have it, add the `include_dir` crate:

`cargo add include_dir`

Where you define your editor, add the following:
```rust

// replace this with the correct output path
static WEB_ASSETS: Dir<'_> = include_dir!("../ui/dist/assets");

// ...
fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
    // again, make sure this path is correct
    let editor = WebViewEditor::new(HTMLSource::String(include_str!("../ui/dist/assets/index.html")), (200, 200))
    // replace <protocol name> with your protocol name
    .with_custom_protocol("<protocol name>".to_owned(), |req| {
        if let Some(file) = WEB_ASSETS.get_file(req.uri().path().trim_start_matches("/")) {
            return Response::builder()
                .header(
                    "content-type",
                    match file.path().extension().unwrap().to_str().unwrap() {
                        "js" => "text/javascript",
                        "css" => "text/css",
                        "ttf" => "font/ttf",
                        _ => "",
                    },
                )
                .header("Access-Control-Allow-Origin", "*")
                .body(file.contents().into())
                .map_err(Into::into);
        }
        panic!("Web asset not found.")
    })
// ...
}
```

#### 2. Inline everything into one HTML file
This is, as far as I know, not natively supported by any web frameworks. You'll either have to find a third-party plugin/extension for your framework or do it yourself. This involves replacing every script and stylesheet tag in your main HTML file with the actual contents of those JS/CSS files. (As for other media like images, etc. I don't know if those are supported with this method). You'll probably want to use an automation tool like gulp to post-process your main HTML file and inline everything. Consider just writing your frontend in plain HTML/CSS/JS, and good luck. 

### With Plain HTML
[Check out the example.](https://github.com/maxjvh/nih-plug-webview/blob/main/example/src/)

Build the example with `cargo xtask bundle gain` in the `example` folder.
