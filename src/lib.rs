use baseview::{Event, Size, Window, WindowHandle, WindowOpenOptions, WindowScalePolicy};
use nih_plug::prelude::{Editor, GuiContext, ParamSetter};
use serde_json::Value;
use std::{
    borrow::Cow,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
};
use wry::{
    http::{Request, Response},
    WebContext, WebView, WebViewBuilder,
};

use crossbeam::channel::{unbounded, Receiver};

pub use wry::http;

pub use baseview::{DropData, DropEffect, EventStatus, MouseEvent};
pub use keyboard_types::*;

type EventLoopHandler = dyn Fn(&WindowHandler, ParamSetter, &mut Window) + Send + Sync;
type KeyboardHandler = dyn Fn(KeyboardEvent) -> bool + Send + Sync;
type MouseHandler = dyn Fn(MouseEvent) -> EventStatus + Send + Sync;
type CustomProtocolHandler =
    dyn Fn(&Request<Vec<u8>>) -> wry::Result<Response<Cow<'static, [u8]>>> + Send + Sync;

pub struct WebViewEditor {
    source: Arc<HTMLSource>,
    width: Arc<AtomicU32>,
    height: Arc<AtomicU32>,
    event_loop_handler: Arc<EventLoopHandler>,
    keyboard_handler: Arc<KeyboardHandler>,
    mouse_handler: Arc<MouseHandler>,
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
            width,
            height,
            developer_mode: false,
            background_color: (255, 255, 255, 255),
            event_loop_handler: Arc::new(|_, _, _| {}),
            keyboard_handler: Arc::new(|_| false),
            mouse_handler: Arc::new(|_| EventStatus::Ignored),
            custom_protocol: None,
        }
    }

    pub fn with_background_color(mut self, background_color: (u8, u8, u8, u8)) -> Self {
        self.background_color = background_color;
        self
    }

    pub fn with_custom_protocol<F>(mut self, name: String, handler: F) -> Self
    where
        F: Fn(&Request<Vec<u8>>) -> wry::Result<Response<Cow<'static, [u8]>>>
            + 'static
            + Send
            + Sync,
    {
        self.custom_protocol = Some((name, Arc::new(handler)));
        self
    }

    pub fn with_event_loop<F>(mut self, handler: F) -> Self
    where
        F: Fn(&WindowHandler, ParamSetter, &mut baseview::Window) + 'static + Send + Sync,
    {
        self.event_loop_handler = Arc::new(handler);
        self
    }

    pub fn with_developer_mode(mut self, mode: bool) -> Self {
        self.developer_mode = mode;
        self
    }

    pub fn with_keyboard_handler<F>(mut self, handler: F) -> Self
    where
        F: Fn(KeyboardEvent) -> bool + Send + Sync + 'static,
    {
        self.keyboard_handler = Arc::new(handler);
        self
    }

    pub fn with_mouse_handler<F>(mut self, handler: F) -> Self
    where
        F: Fn(MouseEvent) -> EventStatus + Send + Sync + 'static,
    {
        self.mouse_handler = Arc::new(handler);
        self
    }
}

pub struct WindowHandler {
    context: Arc<dyn GuiContext>,
    event_loop_handler: Arc<EventLoopHandler>,
    keyboard_handler: Arc<KeyboardHandler>,
    mouse_handler: Arc<MouseHandler>,
    webview: WebView,
    events_receiver: Receiver<Value>,
    pub width: Arc<AtomicU32>,
    pub height: Arc<AtomicU32>,
}

impl WindowHandler {
    pub fn resize(&self, window: &mut baseview::Window, width: u32, height: u32) {
        self.webview.set_bounds(wry::Rect {
            x: 0,
            y: 0,
            width,
            height,
        });
        self.width.store(width, Ordering::Relaxed);
        self.height.store(height, Ordering::Relaxed);
        self.context.request_resize();
        window.resize(Size {
            width: width as f64,
            height: height as f64,
        });
    }

    pub fn send_json(&self, json: Value) -> Result<(), String> {
        let json_str = json.to_string();
        let json_str_quoted =
            serde_json::to_string(&json_str).expect("Should not fail: the value is always string");
        self.webview
            .evaluate_script(&format!("onPluginMessageInternal({});", json_str_quoted))
            .unwrap();
        return Ok(());
    }

    pub fn next_event(&self) -> Result<Value, crossbeam::channel::TryRecvError> {
        self.events_receiver.try_recv()
    }
}

impl baseview::WindowHandler for WindowHandler {
    fn on_frame(&mut self, window: &mut baseview::Window) {
        let setter = ParamSetter::new(&*self.context);
        (self.event_loop_handler)(&self, setter, window);
    }

    fn on_event(&mut self, _window: &mut baseview::Window, event: Event) -> EventStatus {
        match event {
            Event::Keyboard(event) => {
                if (self.keyboard_handler)(event) {
                    EventStatus::Captured
                } else {
                    EventStatus::Ignored
                }
            }
            Event::Mouse(mouse_event) => (self.mouse_handler)(mouse_event),
            _ => EventStatus::Ignored,
        }
    }
}

struct Instance {
    window_handle: WindowHandle,
}

impl Drop for Instance {
    fn drop(&mut self) {
        self.window_handle.close();
    }
}

unsafe impl Send for Instance {}

impl Editor for WebViewEditor {
    fn spawn(
        &self,
        parent: nih_plug::prelude::ParentWindowHandle,
        context: Arc<dyn GuiContext>,
    ) -> Box<dyn std::any::Any + Send> {
        let options = WindowOpenOptions {
            scale: WindowScalePolicy::SystemScaleFactor,
            size: Size {
                width: self.width.load(Ordering::Relaxed) as f64,
                height: self.height.load(Ordering::Relaxed) as f64,
            },
            title: "Plug-in".to_owned(),
        };

        let width = self.width.clone();
        let height = self.height.clone();
        let developer_mode = self.developer_mode;
        let source = self.source.clone();
        let background_color = self.background_color;
        let custom_protocol = self.custom_protocol.clone();
        let event_loop_handler = self.event_loop_handler.clone();
        let keyboard_handler = self.keyboard_handler.clone();
        let mouse_handler = self.mouse_handler.clone();

        let window_handle = baseview::Window::open_parented(&parent, options, move |window| {
            let (events_sender, events_receiver) = unbounded();

            let mut web_context = WebContext::new(Some(std::env::temp_dir()));

            let mut webview_builder = WebViewBuilder::new_as_child(window)
                .with_bounds(wry::Rect {
                    x: 0,
                    y: 0,
                    width: width.load(Ordering::Relaxed) as u32,
                    height: height.load(Ordering::Relaxed) as u32,
                })
                .with_accept_first_mouse(true)
                .with_devtools(developer_mode)
                .with_web_context(&mut web_context)
                .with_initialization_script(include_str!("script.js"))
                .with_ipc_handler(move |msg: String| {
                    if let Ok(json_value) = serde_json::from_str(&msg) {
                        let _ = events_sender.send(json_value);
                    } else {
                        panic!("Invalid JSON from web view: {}.", msg);
                    }
                })
                .with_background_color(background_color);

            if let Some(custom_protocol) = custom_protocol.as_ref() {
                let handler = custom_protocol.1.clone();
                webview_builder = webview_builder
                    .with_custom_protocol(custom_protocol.0.to_owned(), move |request| {
                        handler(&request).unwrap()
                    });
            }

            let webview = match source.as_ref() {
                HTMLSource::String(html_str) => webview_builder.with_html(*html_str),
                HTMLSource::URL(url) => webview_builder.with_url(*url),
            }
            .unwrap()
            .build();

            WindowHandler {
                context,
                event_loop_handler,
                webview: webview.unwrap_or_else(|e| panic!("Failed to construct webview. {}", e)),
                events_receiver,
                keyboard_handler,
                mouse_handler,
                width,
                height,
            }
        });
        return Box::new(Instance { window_handle });
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

    fn param_value_changed(&self, _id: &str, _normalized_value: f32) {}

    fn param_modulation_changed(&self, _id: &str, _modulation_offset: f32) {}
}
