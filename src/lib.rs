#[cfg(target_os = "macos")]
mod mac_os;

#[cfg(target_os = "windows")]
mod windows;

use std::sync::{Arc, Mutex, atomic::{AtomicU32, Ordering}};
use nih_plug::prelude::{Editor, GuiContext, ParamSetter};
use serde_json::Value;
use self::mac_os::{NativeWebView, Timer};

struct Instance {
    _timer: Box<Timer>,
    context: Arc<Mutex<Context>>
}

impl Drop for Instance {
    fn drop(&mut self) {
        if let Ok(mut context) = self.context.lock() {
            context.native_view = None;
        }
    }
}

pub struct Context {
    native_view: Option<NativeWebView>,
    pub gui_context: Option<Arc<dyn GuiContext>>,
    messages: Vec<Value>,
    pub width: Arc<AtomicU32>,
    pub height: Arc<AtomicU32>,
}

impl Context {
    pub fn resize(&mut self, size: (u32, u32)) {
        match (self.native_view.as_mut(), self.gui_context.as_ref()) {
            (Some(native_view), Some(gui_ctx)) => {
                native_view.set_size(size);
                self.width.store(size.0, Ordering::Relaxed);
                self.height.store(size.1, Ordering::Relaxed);
                gui_ctx.request_resize();
            },
            _ => {}
        }
    }
    
    pub fn send_json(&mut self, json: Value) -> Result<(), Option<Value>> {
        if let Some(view) = &self.native_view {
            if let Ok(json_str) = serde_json::to_string(&json) {
                view.evaluate_javascript(&format!("onPluginMessageInternal(`{}`);", json_str));
                return Ok(());
            } else {
                return Err(Some(json));
            }
        }
        Err(None)
    }
    
    pub fn consume_json(&mut self) -> Vec<Value> {
        // TODO: there has to be a better way
        let msgs = self.messages.clone();
        self.messages.clear();
        msgs
    }
}

pub struct WebViewEditor {
    source: Arc<HTMLSource>,
    context: Arc<Mutex<Context>>,
    width: Arc<AtomicU32>,
    height: Arc<AtomicU32>,
    cb: Arc<dyn Fn(&mut Context, ParamSetter) + 'static + Send + Sync>
}

pub enum HTMLSource {
    String(&'static str),
    URL(&'static str)
}

impl WebViewEditor {
    pub fn new<F>(source: HTMLSource, size: (u32, u32), cb: F) -> Self
    where F: Fn(&mut Context, ParamSetter) + 'static + Send + Sync {
        let width = Arc::new(AtomicU32::new(size.0));
        let height = Arc::new(AtomicU32::new(size.1));
        Self {
            source: Arc::new(source),
            context: Arc::new(Mutex::new(Context {
                native_view: None,
                gui_context: None,
                messages: vec!(),
                width: width.clone(),
                height: height.clone()
            })),
            width,
            height,
            cb: Arc::new(cb)
        }
    }
}

impl Editor for WebViewEditor {
    fn spawn(
        &self,
        parent: nih_plug::prelude::ParentWindowHandle,
        gui_context: Arc<dyn GuiContext>,
    ) -> Box<dyn std::any::Any + Send> {
        // setup native web view        
        {
            let mut context = self.context.lock().unwrap();
            context.gui_context = Some(gui_context.clone());
            let inner_context = self.context.clone();
            let size = (self.width.load(Ordering::Relaxed), self.height.load(Ordering::Relaxed));
            context.native_view = Some(NativeWebView::new(parent.handle, self.source.clone(), size, Box::new(move |msg| {
                if let Ok(mut context) = inner_context.lock() {
                    if let Ok(json_value) = serde_json::from_str(msg) {
                        context.messages.push(json_value);
                    }
                } else {
                    panic!("Invalid JSON from web view: {}.", msg);
                }
            })));
        }
        
        // setup timer callback
        let context = self.context.clone();
        let cb = self.cb.clone();
        let gui_ctx = gui_context.clone();
        let timer_callback = move || {
            if let Ok(mut s) = context.lock() {
                let setter = ParamSetter::new(&*gui_ctx);
                cb(&mut s, setter);
            }
        };
        
        Box::new(Instance {
            context: self.context.clone(),
            _timer: Timer::new(1.0 / 60.0, Box::new(timer_callback))
        })
    }
    
    fn size(&self) -> (u32, u32) { 
        (self.width.load(Ordering::Relaxed), self.height.load(Ordering::Relaxed))
    }
    
    fn set_scale_factor(&self, _factor: f32) -> bool {
        // TODO: implement for Windows and Linux
        return false;
    }
    
    fn param_values_changed(&self) {
        // TODO: decide if this should be implemented.
        // might not be that useful if there's no info about which parameter changed?
    }
}
