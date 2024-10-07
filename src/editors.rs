use crate::{HTMLSource, WebViewEditor};
use std::path::PathBuf;
use wry::http::Response;
///
/// Registers a protocol to an existing `WebViewEditor` to work with web frameworks.
///
/// **NOTE**: This helper function assumes your "main" HTML file is "index.html" located at `root_dir`. You may have issues if this isn't the case for you.
///
/// ## Parameters
/// - `editor`: Pass in an existing `WebViewEditor`.
/// - `protocol_name`: If you want to be 100% sure this doesn't conflict with any other custom protocols, specify a name here. Otherwise, supply `None`.
///
fn add_asset_dir_protocol(
    editor: &mut WebViewEditor,
    root_dir: PathBuf,
    protocol_name: Option<String>,
) {
    // if one is not specified, the default protocol will be named "assets"
    let protocol_name = protocol_name.unwrap_or("assets".to_owned());
    // IMPORTANT!!
    // on windows, the custom protocol URL scheme is different
    // (for some awful reason)
    #[cfg(target_os = "windows")]
    let url_scheme = format!("http://{}.localhost", protocol_name);
    // TODO:
    // needs to be tested
    #[cfg(not(target_os = "windows"))]
    let url_scheme = format!("{}://localhost", protocol_name);
    let src = HTMLSource::URL(url_scheme);
    editor = &mut editor.with_custom_protocol(protocol_name, move |req| {
        let path = req.uri().path();
        let file = if path == "/" {
            "index.html"
        } else {
            &path[1..]
        };
        let full_path = root_dir.join(file);
        // mime guess is awesome!
        let mime_type = mime_guess::from_path(&full_path)
            .first_or_text_plain() // TODO: fix _or_...
            .to_string();
        if let Ok(content) = std::fs::read(&full_path) {
            return Response::builder()
                .header("content-type", mime_type)
                .header("Access-Control-Allow-Origin", "*")
                .body(content.into())
                .map_err(Into::into);
        }
        panic!("Web asset not found.")
    });
}
