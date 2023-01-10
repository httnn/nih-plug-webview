use nih_plug::prelude::{Editor, GuiContext, ParamSetter};
use parking_lot::Mutex;
use serde_json::Value;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
};
use wry::webview::{FileDropEvent, WebView, WebViewBuilder, Window};

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

type MessageCallback = dyn Fn(&mut Context, ParamSetter) + 'static + Send + Sync;
 
#[derive(Default)]
pub struct WebViewEditorBuilder {
    source: Option<Arc<HTMLSource>>,
    size: Option<(u32, u32)>,
    event_loop_callback: Option<Arc<MessageCallback>>,
    developer_mode: bool,
    background_color: Option<(u8, u8, u8, u8)>,
}

impl WebViewEditorBuilder {
    pub fn new() -> Self { Default::default() }

    pub fn with_background_color(&mut self, background_color: (u8, u8, u8, u8)) -> &mut Self {
        self.background_color = Some(background_color);
        self
    }

    pub fn with_source(&mut self, source: HTMLSource) -> &mut Self {
        self.source = Some(Arc::new(source));
        self
    }

    pub fn with_size(&mut self, width: u32, height: u32) -> &mut Self {
        self.size = Some((width, height));
        self
    }

    pub fn with_event_loop<F>(&mut self, callback: F) -> &mut Self
    where
        F: Fn(&mut Context, ParamSetter) + 'static + Send + Sync,
    {
        self.event_loop_callback = Some(Arc::new(callback));
        self
    }

    pub fn with_developer_mode(&mut self, mode: bool) -> &mut Self {
        self.developer_mode = mode;
        self
    }

    pub fn build(&self) -> Result<WebViewEditor, ()> {
        WebViewEditor::new(&self)
    }
}

pub struct WebViewEditor {
    context: Arc<Mutex<Context>>,
    source: Arc<HTMLSource>,
    width: Arc<AtomicU32>,
    height: Arc<AtomicU32>,
    event_loop_callback: Arc<MessageCallback>,
    developer_mode: bool,
    background_color: Option<(u8, u8, u8, u8)>,
}

pub enum HTMLSource {
    String(&'static str),
    URL(&'static str),
}

impl WebViewEditor {
    pub fn new(builder: &WebViewEditorBuilder) -> Result<Self, ()> {
        match (builder.source.clone(), builder.event_loop_callback.clone(), builder.size) {
            (Some(source), Some(event_loop_callback), Some(size)) => {
                let width = Arc::new(AtomicU32::new(size.0));
                let height = Arc::new(AtomicU32::new(size.1));
                Ok(Self {
                    source,
                    context: Arc::new(Mutex::new(Context {
                        webview: None,
                        gui_context: None,
                        events: vec![],
                        width: width.clone(),
                        height: height.clone(),
                    })),
                    width,
                    height,
                    developer_mode: builder.developer_mode,
                    background_color: builder.background_color,
                    event_loop_callback,
                })
            }
            _ => Err(()),
        }
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
            .unwrap()
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
            .with_ui_timer(move || {
                let mut context = timer_context.lock();
                let setter = ParamSetter::new(&*gui_ctx);
                event_loop_callback(&mut context, setter);
            });

        if let Some(color) = self.background_color {
            webview_builder = webview_builder.with_background_color(color);
        }

        context.webview = Some(
            match self.source.as_ref() {
                HTMLSource::String(html_str) => webview_builder.with_html(*html_str),
                HTMLSource::URL(url) => webview_builder.with_url(*url),
            }
            .unwrap()
            .build()
            .unwrap(),
        );

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

    fn param_values_changed(&self) {
        // TODO: decide if this should do something.
        // might not be that useful if there's no info about which parameter changed?
    }
}
