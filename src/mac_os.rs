use std::ffi::c_void;
use std::{ptr, mem};

use cacao::core_foundation::runloop::__CFRunLoopTimer;
use cacao::core_foundation::{runloop, date};

extern "C" fn callback(_timer: runloop::CFRunLoopTimerRef, info: *mut c_void) {
    unsafe {
        let timer = info as *mut Timer;
        ((*timer).func)();
    }
}

// Helper to run a timer on the UI thread.
pub struct Timer {
    timer: Option<*mut __CFRunLoopTimer>,
    func: Box<dyn FnMut()>
}

impl Timer {
    pub fn new(interval: f64, func: Box<dyn FnMut()>) -> Box<Self> {
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
