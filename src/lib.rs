use nih_plug::prelude::{Editor, GuiContext, ParamSetter};
use parking_lot::Mutex;
use serde_json::Value;
use std::{
    borrow::Cow,
    path::PathBuf,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
};
use wry::{
    http::{Request, Response},
    webview::{FileDropEvent, WebView, WebViewBuilder, Window},
};

pub use wry::http;

struct Instance {
    context: Arc<Mutex<Context>>,
}

unsafe impl Send for Instance {}
unsafe impl Sync for Instance {}

impl Drop for Instance {
    fn drop(&mut self) {
        let mut context = self.context.lock();
        context.webview = None;
    }
}

#[derive(Clone)]
pub enum WebviewEvent {
    JSON(Value),
    FileDropped(Vec<PathBuf>),
}

pub struct Context {
    webview: Option<WebView>,
    pub gui_context: Option<Arc<dyn GuiContext>>,
    events: Vec<WebviewEvent>,
    pub width: Arc<AtomicU32>,
    pub height: Arc<AtomicU32>,
}

impl Context {
    pub fn resize(&mut self, width: u32, height: u32) {
        match self.gui_context.as_ref() {
            Some(gui_ctx) => {
                self.width.store(width, Ordering::Relaxed);
                self.height.store(height, Ordering::Relaxed);
                gui_ctx.request_resize();
            }
            _ => {}
        }
    }

    pub fn send_json(&mut self, json: Value) -> Result<(), String> {
        // TODO: proper error handling
        if let Some(view) = &self.webview {
            if let Ok(json_str) = serde_json::to_string(&json) {
                view.evaluate_script(&format!("onPluginMessageInternal(`{}`);", json_str))
                    .unwrap();
                return Ok(());
            } else {
                return Err("Can't convert JSON to string.".to_owned());
            }
        }
        Err("Webview not open.".to_owned())
    }

    pub fn consume_events(&mut self) -> Vec<WebviewEvent> {
        // TODO: there has to be a better way
        let msgs = self.events.clone();
        self.events.clear();
        msgs
    }
}

type EventLoopHandler = dyn Fn(&mut Context, ParamSetter) + 'static + Send + Sync;
type CustomProtocolHandler =
    dyn Fn(&Request<Vec<u8>>) -> wry::Result<Response<Cow<'static, [u8]>>> + 'static;

pub struct WebViewEditor {
    context: Arc<Mutex<Context>>,
    source: Arc<HTMLSource>,
    width: Arc<AtomicU32>,
    height: Arc<AtomicU32>,
    event_loop_callback: Option<Arc<EventLoopHandler>>,
    custom_protocol: Option<(String, Arc<CustomProtocolHandler>)>,
    developer_mode: bool,
    background_color: (u8, u8, u8, u8),
}

pub enum HTMLSource {
    String(&'static str),
    URL(&'static str),
}

impl WebViewEditor {
    pub fn new(source: HTMLSource, size: (u32, u32)) -> Self {
        let width = Arc::new(AtomicU32::new(size.0));
        let height = Arc::new(AtomicU32::new(size.1));
        Self {
            source: Arc::new(source),
            context: Arc::new(Mutex::new(Context {
                webview: None,
                gui_context: None,
                events: vec![],
                width: width.clone(),
                height: height.clone(),
            })),
            width,
            height,
            developer_mode: false,
            background_color: (255, 255, 255, 255),
            event_loop_callback: None,
            custom_protocol: None,
        }
    }

    pub fn with_background_color(mut self, background_color: (u8, u8, u8, u8)) -> Self {
        self.background_color = background_color;
        self
    }

    pub fn with_custom_protocol<F>(mut self, name: String, handler: F) -> Self
    where
        F: Fn(&Request<Vec<u8>>) -> wry::Result<Response<Cow<'static, [u8]>>> + 'static,
    {
        self.custom_protocol = Some((name, Arc::new(handler)));
        self
    }

    pub fn with_event_loop<F>(mut self, callback: F) -> Self
    where
        F: Fn(&mut Context, ParamSetter) + 'static + Send + Sync,
    {
        self.event_loop_callback = Some(Arc::new(callback));
        self
    }

    pub fn with_developer_mode(mut self, mode: bool) -> Self {
        self.developer_mode = mode;
        self
    }
}

unsafe impl Send for WebViewEditor {}
unsafe impl Sync for WebViewEditor {}

impl Editor for WebViewEditor {
    fn spawn(
        &self,
        parent: nih_plug::prelude::ParentWindowHandle,
        gui_context: Arc<dyn GuiContext>,
    ) -> Box<dyn std::any::Any + Send> {
        let mut context = self.context.lock();
        context.gui_context = Some(gui_context.clone());
        let file_drop_context = self.context.clone();
        let ipc_context = self.context.clone();
        let timer_context = self.context.clone();
        let gui_ctx = gui_context.clone();
        let event_loop_callback = self.event_loop_callback.clone();

        let mut webview_builder = WebViewBuilder::new(Window::new(parent.handle))
            .unwrap() // always returns Ok()
            .with_accept_first_mouse(true)
            .with_devtools(self.developer_mode)
            .with_initialization_script(include_str!("script.js"))
            .with_file_drop_handler(move |_: &Window, msg: FileDropEvent| {
                if let FileDropEvent::Dropped(path) = msg {
                    let mut context = file_drop_context.lock();
                    context.events.push(WebviewEvent::FileDropped(path));
                }
                false
            })
            .with_ipc_handler(move |_: &Window, msg: String| {
                let mut context = ipc_context.lock();
                if let Ok(json_value) = serde_json::from_str(&msg) {
                    context.events.push(WebviewEvent::JSON(json_value));
                } else {
                    panic!("Invalid JSON from web view: {}.", msg);
                }
            })
            .with_background_color(self.background_color);

        if let Some(cb) = event_loop_callback {
            webview_builder.webview.ui_timer = Some(Box::new(move || {
                let mut context = timer_context.lock();
                let setter = ParamSetter::new(&*gui_ctx);
                cb(&mut context, setter);
            }));
        }

        if let Some(custom_protocol) = self.custom_protocol.as_ref() {
            let handler = custom_protocol.1.clone();
            webview_builder.webview.custom_protocols.push((
                custom_protocol.0.to_owned(),
                Box::new(move |request| handler(request)),
            ));
        }

        let webview = match self.source.as_ref() {
            HTMLSource::String(html_str) => webview_builder.with_html(*html_str),
            HTMLSource::URL(url) => webview_builder.with_url(*url),
        }
        .unwrap()
        .build();

        context.webview = Some(webview.unwrap_or_else(|_| panic!("Failed to construct webview.")));

        Box::new(Instance {
            context: self.context.clone(),
        })
    }

    fn size(&self) -> (u32, u32) {
        (
            self.width.load(Ordering::Relaxed),
            self.height.load(Ordering::Relaxed),
        )
    }

    fn set_scale_factor(&self, _factor: f32) -> bool {
        // TODO: implement for Windows and Linux
        return false;
    }

    fn param_values_changed(&self) {}

    fn param_value_changed(&self, id: &str, normalized_value: f32) {}

    fn param_modulation_changed(&self, _id: &str, _modulation_offset: f32) {}
}
