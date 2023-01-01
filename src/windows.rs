use std::ffi::c_void;
use std::sync::Arc;
use std::{ ptr, mem };
use raw_window_handle::RawWindowHandle;
use winapi::shared::windef::HWND;
use winapi::um::winuser::{ SetTimer, WM_TIMER, KillTimer };
use winapi::shared::basetsd::{ UINT_PTR };
use winapi::shared::minwindef::{ UINT, DWORD };
use super::HTMLSource;
use std::io::prelude::*;

pub struct Timer {
    handle: *mut c_void,
    id_event: Option<*mut c_void>,
    func: Box<dyn FnMut()>
}

impl Timer {
    pub fn new(handle: *mut c_void, interval: f64, func: Box<dyn FnMut()>) -> Arc<Self> {
        let mut timer = Arc::new(Self { handle, func, id_event: None });
        timer.id_event = Some(Arc::as_ptr(&timer) as *mut c_void);
        unsafe {
            SetTimer(handle as HWND, timer.id_event.unwrap(), interval as u32, callback);
        }
        timer
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        unsafe { KillTimer(self.handle as HWND, self.id_event.unwrap()); }
    }
}

unsafe impl Sync for Timer {}
unsafe impl Send for Timer {}

extern "C" fn callback(hwnd: HWND, u_msg: UINT, id_event: UINT_PTR, dw_time: DWORD){
    unsafe{
        let timer = id_event as *mut Timer;
        ((*timer).func)();
    }
}
