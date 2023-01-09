use std::ffi::c_void;
use winapi::shared::windef::{HWND};
use winapi::um::winuser::{ SetTimer, KillTimer };
use winapi::shared::basetsd::{ UINT_PTR };
use winapi::shared::minwindef::{ UINT, DWORD };

pub struct Timer {
    handle: *mut c_void,
    func: Box<dyn FnMut()>
}

impl Timer {
    pub fn new(handle: *mut c_void, interval: f64, func: Box<dyn FnMut()>) -> Box<Self> {
        let mut timer =  Box::new(Self { handle, func});

        unsafe {
            SetTimer(handle as HWND, &mut *timer as *mut _ as UINT_PTR, interval as u32, Some(callback));
        }

        timer
       
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        unsafe { 
            KillTimer(self.handle as HWND, self as *mut _ as UINT_PTR); 
        }
    }
}

unsafe impl Sync for Timer {}
unsafe impl Send for Timer {}

extern "system" fn callback(_hwnd: HWND, _u_msg: UINT, id_event: UINT_PTR, _dw_time: DWORD){
    unsafe{
        let timer = id_event as *mut Timer;
        ((*timer).func)();
    }
}