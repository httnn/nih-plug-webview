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
const WIN_FRAME_TIMER: usize = 4242;
pub struct Timer {
    handle: HWND,
    idevent: usize,
    func: Box<dyn FnMut()>
}
impl Timer {
    pub fn new(handle: HWND, interval: f64, func: Box<dyn Fn()>) -> Box<Self> {
        unsafe {
            SetTimer(handle, 1234, 100, Some(callback));
        }
        Box::new(Self { handle: handle, idevent: 1234,  func:Box::new(|| {})})
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        unsafe { KillTimer(self.handle, self.idevent); }
    }
}

unsafe impl Sync for Timer {}
unsafe impl Send for Timer {}

extern "system" fn callback(hwnd: HWND, uMsg: UINT, idEvent: UINT_PTR, dwTime: DWORD){
    pub static mut _clb: *mut UINT = 0 as *const UINT as *mut UINT;
    // let mut file = std::fs::OpenOptions::new()
    //     .write(true)
    //     .append(true)
    //     .open("D:/VST/file.txt")
    //     .unwrap();
    //     if let Err(e) = writeln!(file, "{:?}", timer) {
    //         eprintln!("Couldn't write to file: {}", e);
    //     }

    

    unsafe{
        let timer = _clb as *mut Timer;
        ((*timer).func)();
    }
}

type Callback = Box<dyn Fn(&str)>;