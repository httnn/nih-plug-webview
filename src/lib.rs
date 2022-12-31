use nih_plug::prelude::{Editor, GuiContext, ParamSetter};
use raw_window_handle::RawWindowHandle;
use serde_json::Value;
use std::{
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    }, path::PathBuf,
};
use wry::{
    webview::{WebView, WebViewBuilder, Window, FileDropEvent},
};

use parking_lot::Mutex;

struct Instance {
    context: Arc<Mutex<Context>>,
}

unsafe impl Send for Instance {}
unsafe impl Sync for Instance {}

impl Drop for Instance {
    fn drop(&mut self) {
        self.context.lock().webview = None;
    }
}

#[derive(Clone)]
pub enum WebviewMessage {
    JSON(Value),
    FileDropped(Vec<PathBuf>)
}

pub struct Context {
    webview: Option<WebView>,
    pub gui_context: Option<Arc<dyn GuiContext>>,
    pub width: Arc<AtomicU32>,
    pub height: Arc<AtomicU32>,
}

impl Context {
    pub fn resize(&mut self, size: (u32, u32)) {
        match self.gui_context.as_ref() {
            Some(gui_ctx) => {
                self.width.store(size.0, Ordering::Relaxed);
                self.height.store(size.1, Ordering::Relaxed);
                gui_ctx.request_resize();
            }
            _ => {}
        }
    }

    pub fn send_json(&mut self, json: Value) -> Result<(), Option<Value>> {
        if let Some(view) = &self.webview {
            if let Ok(json_str) = serde_json::to_string(&json) {
                view.evaluate_script(&format!("onPluginMessageInternal(`{}`);", json_str)).unwrap();
                return Ok(());
            } else {
                return Err(Some(json));
            }
        }
        Err(None)
    }
}

type MessageCallback = dyn Fn(WebviewMessage, &mut Context) + 'static + Send + Sync;

pub struct WebViewEditorBuilder {
    source: Option<Arc<HTMLSource>>,
    size: Option<(u32, u32)>,
    cb: Option<Arc<MessageCallback>>,
    developer_mode: bool,
}

impl WebViewEditorBuilder {
    pub fn new() -> Self {
        Self {
            source: None,
            size: None,
            cb: None,
            developer_mode: false,
        }
    }

    pub fn with_source(&mut self, source: HTMLSource) -> &mut Self {
        self.source = Some(Arc::new(source));
        self
    }

    pub fn with_size(&mut self, width: u32, height: u32) -> &mut Self {
        self.size = Some((width, height));
        self
    }

    pub fn with_callback<F>(&mut self, callback: F) -> &mut Self
    where
        F: Fn(WebviewMessage, &mut Context) + 'static + Send + Sync,
    {
        self.cb = Some(Arc::new(callback));
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
    cb: Arc<MessageCallback>,
    developer_mode: bool,
}

pub enum HTMLSource {
    String(&'static str),
    URL(&'static str),
}

impl WebViewEditor {
    pub fn new(builder: &WebViewEditorBuilder) -> Result<Self, ()> {
        match (builder.source.clone(), builder.cb.clone(), builder.size) {
            (Some(source), Some(cb), Some(size)) => {
                let width = Arc::new(AtomicU32::new(size.0));
                let height = Arc::new(AtomicU32::new(size.1));
                Ok(Self {
                    source,
                    context: Arc::new(Mutex::new(Context {
                        webview: None,
                        gui_context: None,
                        width: width.clone(),
                        height: height.clone(),
                    })),
                    width,
                    height,
                    developer_mode: builder.developer_mode,
                    cb,
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
        // setup native web view
        {
            let mut context = self.context.lock();
            context.gui_context = Some(gui_context.clone());
            let file_drop_cb = self.cb.clone();
            let ipc_cb = self.cb.clone();

            let file_drop_ctx = self.context.clone();
            let ipc_ctx = self.context.clone();

            let mut webview_builder = match parent.handle {
                #[cfg(target_os = "macos")]
                RawWindowHandle::AppKit(handle) => WebViewBuilder::new(Window { ns_view: handle.ns_view }).unwrap(),
                #[cfg(target_os = "windows")]
                RawWindowHandle::Win32(_handle) => WebViewBuilder::new(Window { hwnd: _handle.hwnd}).unwrap(),
                _ => panic!(),
            };

            webview_builder = webview_builder
            .with_accept_first_mouse(true)
            .with_devtools(self.developer_mode)
            .with_initialization_script(include_str!("script.js"))
            .with_file_drop_handler(move |_: &Window, msg: FileDropEvent| {
                if let FileDropEvent::Dropped(path) = msg {
                    let mut ctx = file_drop_ctx.lock();
                    (*file_drop_cb)(WebviewMessage::FileDropped(path), &mut ctx);
                }
                false
            })
            .with_ipc_handler(move |_: &Window, msg: String| {
                if let Ok(json_value) = serde_json::from_str(&msg) {
                    let mut ctx = ipc_ctx.lock();
                    (*ipc_cb)(WebviewMessage::JSON(json_value), &mut ctx);
                } else {
                    panic!("Invalid JSON from web view: {}.", msg);
                }
            });

            context.webview = Some(
                match self.source.as_ref() {
                    HTMLSource::String(html_str) => webview_builder.with_html(*html_str),
                    HTMLSource::URL(url) => webview_builder.with_url(*url),
                    
                }.unwrap().build().unwrap()
            );
        }
        
        Box::new(Instance {
            context: self.context.clone()
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
