use std::ffi::c_void;
use std::sync::Arc;
use std::{ptr, mem};

use cacao::core_foundation::runloop::__CFRunLoopTimer;
use cacao::core_foundation::{runloop, date};
use cacao::foundation::{id, NSString, YES, NO};
use cacao::geometry::Rect;
use cacao::layout::{Layout};
use cacao::objc::{sel, sel_impl, self, msg_send, class};
use cacao::webview::WebViewDelegate;
use raw_window_handle::RawWindowHandle;
use cacao::webview::*;

use super::HTMLSource;

extern "C" fn callback(_timer: runloop::CFRunLoopTimerRef, info: *mut c_void) {
    unsafe {
        let timer = info as *mut Timer;
        ((*timer).func)();
    }
}

// Helper to run a timer on the UI thread.
pub struct Timer {
    timer: Option<*mut __CFRunLoopTimer>,
    func: Box<dyn Fn()>
}

impl Timer {
    pub fn new(interval: f64, func: Box<dyn Fn()>) -> Box<Self> {
        unsafe {
            let mut s = Box::new(Timer { timer: None, func });
            
            s.timer = Some(runloop::CFRunLoopTimerCreate(
                ptr::null(),
                date::CFAbsoluteTimeGetCurrent() + interval,
                interval,
                0,
                0,
                callback,
                &mut runloop::CFRunLoopTimerContext {
                    info: &mut *s as *mut _ as *mut c_void,
                    ..mem::zeroed()
                },
            ));
            
            runloop::CFRunLoopAddTimer(
                runloop::CFRunLoopGetCurrent(),
                s.timer.unwrap(),
                runloop::kCFRunLoopCommonModes,
            );
            
            s
        }
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        unsafe {
            runloop::CFRunLoopTimerInvalidate(self.timer.unwrap());
        }
    }
}

unsafe impl Sync for Timer {}
unsafe impl Send for Timer {}

type Callback = Box<dyn Fn(&str)>;

pub struct WebViewInstance {
    message_callback: Callback
}

impl WebViewDelegate for WebViewInstance {
    fn on_message(&self, _name: &str, body: &str) {
        (self.message_callback)(body);
    }

    const NAME: &'static str = "WebViewDelegate";
}

pub struct NativeWebView {
    native_view: WebView<WebViewInstance>
}

unsafe impl Sync for NativeWebView {}
unsafe impl Send for NativeWebView {}

fn set_config_value(config: &WebViewConfig, key: &str, value: bool) {
    let key = NSString::new(key);
    unsafe {
        let value = if value { YES } else { NO };
        let yes: id = msg_send![class!(NSNumber), numberWithBool: value];
        let preferences: id = msg_send![&*config.objc, preferences];
        let _: () = msg_send![preferences, setValue:yes forKey:key];
    }
}

impl NativeWebView {
    pub fn new(handle: RawWindowHandle, source: Arc<HTMLSource>, initial_size: (u32, u32), developer_mode: bool, message_callback: Callback) -> Self {
        if let RawWindowHandle::AppKit(handle) = handle {
            // setup config
            let mut webview_config = WebViewConfig::default();
            webview_config.handlers.push("main".to_owned());

            if developer_mode {
                webview_config.enable_developer_extras();
            }

            set_config_value(&webview_config, "fullScreenEnabled", true);
            set_config_value(&webview_config, "DOMPasteAllowed", true);
            set_config_value(&webview_config, "javaScriptCanAccessClipboard", true);
            webview_config.add_user_script(include_str!("mac_script.js"), InjectAt::Start, true);

            // construct webview
            let view = WebView::with(webview_config, WebViewInstance { message_callback });
            view.set_translates_autoresizing_mask_into_constraints(true);
            view.set_frame(Rect {
                top: 0.0,
                left: 0.0,
                width: initial_size.0 as f64,
                height: initial_size.1 as f64
            });

            // add webview to parent view (received via raw window handle)
            let root_view = handle.ns_view as id;
            view.objc.get(|obj| {
                unsafe {
                    let _: () = objc::msg_send![root_view, addSubview: obj];
                    let window: id = objc::msg_send![root_view, window];
                    let _: () = objc::msg_send![window, makeFirstResponder: obj];
                };
            });

            // set HTML content
            match *source {
                HTMLSource::String(html) => view.load_html(html),
                HTMLSource::URL(url) => view.load_url(url)
            }

            NativeWebView { native_view: view }
        } else {
            panic!("Invalid window handle.");
        }
    }
    
    pub fn set_size(&mut self, size: (u32, u32)) {
        self.native_view.set_frame(Rect {
            top: 0.0,
            left: 0.0,
            width: size.0 as f64,
            height: size.1 as f64
        });
    }
    
    pub fn evaluate_javascript(&self, js: &str) {
        self.native_view.objc.get(|obj| {
            unsafe {
                let _: () = objc::msg_send![obj, evaluateJavaScript:NSString::new(js) completionHandler:false];
            };
        });
    }
}
