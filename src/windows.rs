use std::ffi::c_void;
use std::sync::Arc;
use std::{ ptr, mem };
use raw_window_handle::RawWindowHandle;
use winapi::shared::windef::{HWND, HWND__};
use winapi::um::winuser::{ SetTimer, WM_TIMER, KillTimer };
use winapi::shared::basetsd::{ UINT_PTR };
use winapi::shared::minwindef::{ UINT, DWORD };
use super::HTMLSource;
use std::io::prelude::*;

pub struct Timer {
    handle: *mut c_void,
    func: Box<dyn FnMut()>
}

impl Timer {
    pub fn new(handle: *mut c_void, interval: f64, func: Box<dyn FnMut()>) -> Box<Self> {
        let mut timer =  Box::new(Self { handle, func});

        unsafe {
            let test = SetTimer(handle as HWND, &mut *timer as *mut _ as UINT_PTR, interval as u32, Some(callback));
            // std::fs::write("D:/VST/debug/new.txt", test.to_string()).expect("Unable to write file");
        }

        timer
       
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        unsafe { 
            // let ptr_num_transmute = unsafe { std::mem::transmute::<*mut c_void, usize>(self.id_event.unwrap()) };

            // KillTimer(self.handle as HWND, ptr_num_transmute); 
        }
    }
}

unsafe impl Sync for Timer {}
unsafe impl Send for Timer {}

extern "system" fn callback(hwnd: HWND, u_msg: UINT, id_event: UINT_PTR, dw_time: DWORD){
    unsafe{
        let timer = id_event as *mut Timer;
        ((*timer).func)();
        // std::fs::write("D:/VST/debug/callback.txt", id_event.to_string()).expect("Unable to write file");
    }
}