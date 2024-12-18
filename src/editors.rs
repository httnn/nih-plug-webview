use crate::{HTMLSource, WebViewEditor};
use std::{path::Path, sync::Arc};
use wry::http::Response;

impl WebViewEditor {
    /// This function does 2 things:
    /// 1. Overwrites the current `WebViewEditor`'s source to use a URL
    /// 2. Registers a protocol to serve static assets from an output directory
    ///
    /// This function is intended to be used when you have a set of static assets from a web framework.
    ///
    /// **NOTE**: This helper function assumes your "main" HTML file is "index.html" located at `root_dir`. You may have issues if this isn't the case for you.
    ///
    /// ## Parameters
    /// - `asset_dir`: The root of your asset directory.
    /// - `protocol_name`: If you're registering other protocols and want to ensure there are no collisions, specify your own protocol name here.
    pub fn with_asset_dir_protocol<P: AsRef<Path> + Sync + Send + 'static>(
        mut self,
        asset_dir: P,
        protocol_name: Option<String>,
    ) -> Self {
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

        self.source = Arc::new(src);

        self = self.with_custom_protocol(protocol_name, move |req| {
            let path = req.uri().path();
            let file = if path == "/" {
                "index.html"
            } else {
                &path[1..]
            };
            let full_path = asset_dir.as_ref().join(file);
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
        self
    }
}
