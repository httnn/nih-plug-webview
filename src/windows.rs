use std::ffi::c_void;
use std::sync::Arc;
use std::{ptr, mem};

use raw_window_handle::RawWindowHandle;

use super::HTMLSource;

pub struct Timer {}

impl Timer {
    pub fn new(interval: f64, func: Box<dyn Fn()>) -> Box<Self> {
        todo!()
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        todo!()
    }
}

unsafe impl Sync for Timer {}
unsafe impl Send for Timer {}

type Callback = Box<dyn Fn(&str)>;

impl NativeWebView {
    pub fn new(handle: RawWindowHandle, source: Arc<HTMLSource>, size: (u32, u32), callback: Callback) -> Self {
        todo!()
    }
    
    pub fn set_size(&mut self, size: (u32, u32)) {
        todo!()
    }
    
    pub fn evaluate_javascript(&self, js: &str) {
        todo!()
    }
}
