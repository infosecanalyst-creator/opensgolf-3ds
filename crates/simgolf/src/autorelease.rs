//! macOS: an Objective-C autorelease pool around each frame (the port's own; nothing elsewhere).
//!
//! miniquad runs its own event loop on macOS (it asks NSApp for each event and draws in between) without an autorelease
//! pool, so every object Cocoa hands back autoreleased in that loop (each mouse and key event, the objects made while one is
//! dispatched, the presenting of each frame) went into a pool that is never emptied: the game's memory grew for as long as
//! it ran, by every mouse move, and it slowed down the longer it was played until it was started again. Each frame now
//! empties the pool of the frame before it and opens a new one, as an application on Cocoa's own event loop has.

use std::cell::Cell;
use std::ffi::c_void;

#[link(name = "objc")]
extern "C" {
    fn objc_autoreleasePoolPush() -> *mut c_void;
    fn objc_autoreleasePoolPop(pool: *mut c_void);
    fn objc_getClass(name: *const std::ffi::c_char) -> *mut c_void;
    fn sel_registerName(name: *const std::ffi::c_char) -> *mut c_void;
    fn objc_msgSend();
}

thread_local! {
    static POOL: Cell<*mut c_void> = const { Cell::new(std::ptr::null_mut()) };
}

/// Whether Cocoa's run loop is running on this thread now: a frame drawn from inside it (while a window is resized or
/// moved, AppKit draws the view itself) sits inside AppKit's own pools, which only AppKit may empty.
fn in_run_loop() -> bool {
    // SAFETY: plain Objective-C messages to NSRunLoop on the main thread; objc_msgSend is called through the signature of
    // the two methods (an object in, an object out, no arguments)
    unsafe {
        let send: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
            std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        let class = objc_getClass(c"NSRunLoop".as_ptr());
        if class.is_null() {
            return true;
        }
        let run_loop = send(class, sel_registerName(c"currentRunLoop".as_ptr()));
        if run_loop.is_null() {
            return true;
        }
        !send(run_loop, sel_registerName(c"currentMode".as_ptr())).is_null()
    }
}

/// At the start of each frame (main.rs update): empties the pool the last frame opened and opens the next one. Only from
/// miniquad's own loop, where no pool of Cocoa's is open above it.
pub fn next_frame() {
    if in_run_loop() {
        return;
    }
    POOL.with(|p| {
        let old = p.replace(std::ptr::null_mut());
        // SAFETY: `old` was pushed by this function on this thread, at this same level of miniquad's loop; every pool
        // opened since then has been closed again by whoever opened it (AppKit's are balanced within each call)
        unsafe {
            if !old.is_null() {
                objc_autoreleasePoolPop(old);
            }
            p.set(objc_autoreleasePoolPush());
        }
    });
}
