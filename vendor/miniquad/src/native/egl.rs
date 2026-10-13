#![allow(non_camel_case_types, non_snake_case, dead_code)]

#[cfg(target_os = "linux")]
pub type EGLNativeDisplayType = *mut crate::native::linux_x11::libx11::Display;
#[cfg(target_os = "linux")]
pub type EGLNativePixmapType = crate::native::linux_x11::libx11::Pixmap;
#[cfg(target_os = "linux")]
pub type EGLNativeWindowType = crate::native::linux_x11::libx11::Window;

#[cfg(target_os = "android")]
pub type EGLNativeDisplayType = *mut ();
#[cfg(target_os = "android")]
pub type EGLNativePixmapType = ::core::ffi::c_ulong;
#[cfg(target_os = "android")]
pub type EGLNativeWindowType = ::core::ffi::c_ulong;

pub use core::ptr::null_mut;
use std::fmt::Display;

pub const EGL_SUCCESS: u32 = 12288;

pub const EGL_WINDOW_BIT: u32 = 4;

pub const EGL_ALPHA_SIZE: u32 = 12321;
pub const EGL_BLUE_SIZE: u32 = 12322;
pub const EGL_GREEN_SIZE: u32 = 12323;
pub const EGL_RED_SIZE: u32 = 12324;
pub const EGL_DEPTH_SIZE: u32 = 12325;
pub const EGL_STENCIL_SIZE: u32 = 12326;
pub const EGL_SAMPLES: u32 = 12337;
pub const EGL_NATIVE_VISUAL_ID: u32 = 12334;
pub const EGL_WIDTH: u32 = 12375;
pub const EGL_HEIGHT: u32 = 12374;
pub const EGL_SURFACE_TYPE: u32 = 12339;
pub const EGL_NONE: u32 = 12344;
pub const EGL_CONTEXT_CLIENT_VERSION: u32 = 12440;
pub const EGL_RENDERABLE_TYPE: u32 = 12352;
pub const EGL_OPENGL_ES2_BIT: u32 = 4;
pub const EGL_OPENGL_ES3_BIT: u32 = 64;

pub type NativeDisplayType = EGLNativeDisplayType;
pub type NativePixmapType = EGLNativePixmapType;
pub type NativeWindowType = EGLNativeWindowType;
pub type EGLint = i32;
pub type EGLBoolean = ::core::ffi::c_uint;
pub type EGLDisplay = *mut ::core::ffi::c_void;
pub type EGLConfig = *mut ::core::ffi::c_void;
pub type EGLSurface = *mut ::core::ffi::c_void;
pub type EGLContext = *mut ::core::ffi::c_void;
pub type __eglMustCastToProperFunctionPointerType = ::std::option::Option<unsafe extern "C" fn()>;

crate::declare_module! {
    LibEgl,
    "libEGL.so",
    "libEGL.so.1",
    ...
    ...
    pub fn eglChooseConfig(
        EGLDisplay,
        *const EGLint,
        *mut EGLConfig,
        EGLint,
        *mut EGLint,
    ) -> EGLBoolean,
    pub fn eglCopyBuffers(
        EGLDisplay,
        EGLSurface,
        EGLNativePixmapType,
    ) -> EGLBoolean,
    pub fn eglCreateContext(
        EGLDisplay,
        EGLConfig,
        EGLContext,
        *const EGLint,
    ) -> EGLContext,
    pub fn eglCreatePbufferSurface(
        EGLDisplay,
        EGLConfig,
        *const EGLint,
    ) -> EGLSurface,
    pub fn eglCreatePixmapSurface(
        EGLDisplay,
        EGLConfig,
        EGLNativePixmapType,
        *const EGLint,
    ) -> EGLSurface,
    pub fn eglCreateWindowSurface(
        EGLDisplay,
        EGLConfig,
        EGLNativeWindowType,
        *const EGLint,
    ) -> EGLSurface,
    pub fn eglDestroyContext(EGLDisplay, EGLContext) -> EGLBoolean,
    pub fn eglDestroySurface(EGLDisplay, EGLSurface) -> EGLBoolean,
    pub fn eglGetConfigAttrib(
        EGLDisplay,
        EGLConfig,
        EGLint,
        *mut EGLint,
    ) -> EGLBoolean,
    pub fn eglGetConfigs(
        EGLDisplay,
        *mut EGLConfig,
        EGLint,
        *mut EGLint,
    ) -> EGLBoolean,
    pub fn eglGetCurrentContext() -> EGLContext,
    pub fn eglGetCurrentDisplay() -> EGLDisplay,
    pub fn eglGetCurrentSurface(EGLint) -> EGLSurface,
    pub fn eglGetDisplay(EGLNativeDisplayType) -> EGLDisplay,
    pub fn eglGetError() -> EGLint,
    pub fn eglGetProcAddress(
        *const ::core::ffi::c_char,
    ) -> __eglMustCastToProperFunctionPointerType,
    pub fn eglInitialize(EGLDisplay, *mut EGLint, *mut EGLint) -> EGLBoolean,
    pub fn eglMakeCurrent(
        EGLDisplay,
        EGLSurface,
        EGLSurface,
        EGLContext,
    ) -> EGLBoolean,
    pub fn eglQueryContext(
        EGLDisplay,
        EGLContext,
        EGLint,
        *mut EGLint,
    ) -> EGLBoolean,
    pub fn eglQueryString(EGLDisplay, EGLint) -> *const ::core::ffi::c_char,
    pub fn eglQuerySurface(
        EGLDisplay,
        EGLSurface,
        EGLint,
        *mut EGLint,
    ) -> EGLBoolean,
    pub fn eglSwapBuffers(EGLDisplay, EGLSurface) -> EGLBoolean,
    pub fn eglTerminate(EGLDisplay) -> EGLBoolean,
    pub fn eglWaitGL() -> EGLBoolean,
    pub fn eglWaitNative(EGLint) -> EGLBoolean,
    pub fn eglBindTexImage(EGLDisplay, EGLSurface, EGLint) -> EGLBoolean,
    pub fn eglReleaseTexImage(EGLDisplay, EGLSurface, EGLint) -> EGLBoolean,
    pub fn eglSurfaceAttrib(
        EGLDisplay,
        EGLSurface,
        EGLint,
        EGLint,
    ) -> EGLBoolean,
    pub fn eglSwapInterval(EGLDisplay, EGLint) -> EGLBoolean,
    ...
    ...
}

#[derive(Debug)]
pub enum EglError {
    NoDisplay,
    InitializeFailed,
    CreateContextFailed,
}

impl Display for EglError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoDisplay => write!(f, "No display"),
            Self::InitializeFailed => write!(f, "Failed to initialize context"),
            Self::CreateContextFailed => write!(f, "Faild to create context"),
        }
    }
}

impl std::error::Error for EglError {}

/// OpenSGolf: the Android EGL setup. upstream's [`create_egl_context`] asks for one exact config (with the default
/// `EGL_RENDERABLE_TYPE`, OpenGL ES 1, and `EGL_SAMPLES` at least `sample_count`), asserts that one exists and makes an
/// OpenGL ES 2 context. A GPU driver without such a config (or one that gives exactly ES 2 for a version 2 request, as
/// ANGLE does) closed the app at start. This tries an ES 3 context first, then ES 2, each with the upstream attributes,
/// then without multisampling, then with any depth buffer, and says what it tried when nothing works.
/// Returns the context, its config, the display and the ES version made.
#[cfg(target_os = "android")]
pub unsafe fn create_egl_context_android(
    egl: &mut LibEgl,
    alpha: bool,
    sample_count: i32,
) -> Result<(EGLContext, EGLConfig, EGLDisplay, i32), String> {
    let display = (egl.eglGetDisplay)(null_mut());
    if display.is_null() {
        return Err(format!("eglGetDisplay: no display (EGL error {:#x})", (egl.eglGetError)()));
    }
    if (egl.eglInitialize)(display, null_mut(), null_mut()) == 0 {
        return Err(format!("eglInitialize failed (EGL error {:#x})", (egl.eglGetError)()));
    }
    let alpha_size = if alpha { 8 } else { 0 };
    let mut tried = Vec::new();
    for (version, bit) in [(3, EGL_OPENGL_ES3_BIT), (2, EGL_OPENGL_ES2_BIT)] {
        for (samples, depth) in [(sample_count.max(0) as u32, 16), (0, 16), (0, 0)] {
            #[rustfmt::skip]
            let attributes = [
                EGL_SURFACE_TYPE, EGL_WINDOW_BIT,
                EGL_RENDERABLE_TYPE, bit,
                EGL_RED_SIZE, 8,
                EGL_GREEN_SIZE, 8,
                EGL_BLUE_SIZE, 8,
                EGL_ALPHA_SIZE, alpha_size,
                EGL_DEPTH_SIZE, depth,
                EGL_STENCIL_SIZE, 0,
                EGL_SAMPLES, samples,
                EGL_NONE,
            ];
            let mut configs: Vec<EGLConfig> = vec![null_mut(); 64];
            let mut count: EGLint = 0;
            if (egl.eglChooseConfig)(display, attributes.as_ptr() as _, configs.as_mut_ptr(), 64, &mut count) == 0
                || count <= 0
            {
                tried.push(format!("ES {version}, {samples} samples, depth {depth}: no config"));
                continue;
            }
            let configs = &configs[..(count as usize).min(64)];
            // the first 8-bit RGB config with the depth asked for (the NDK sample does not trust the sort order)
            let attr = |c: EGLConfig, a: u32| {
                let mut v: EGLint = 0;
                if (egl.eglGetConfigAttrib)(display, c, a as _, &mut v) == 1 {
                    v
                } else {
                    -1
                }
            };
            let exact = configs.iter().copied().find(|&c| {
                attr(c, EGL_RED_SIZE) == 8
                    && attr(c, EGL_GREEN_SIZE) == 8
                    && attr(c, EGL_BLUE_SIZE) == 8
                    && (alpha_size == 0 || attr(c, EGL_ALPHA_SIZE) == alpha_size as i32)
                    && (depth == 0 || attr(c, EGL_DEPTH_SIZE) == depth as i32)
            });
            let config = exact.unwrap_or(configs[0]);
            let context_attributes = [EGL_CONTEXT_CLIENT_VERSION, version, EGL_NONE];
            let context = (egl.eglCreateContext)(display, config, null_mut(), context_attributes.as_ptr() as _);
            if !context.is_null() {
                return Ok((context, config, display, version as i32));
            }
            tried.push(format!(
                "ES {version}, {samples} samples, depth {depth}: eglCreateContext failed (EGL error {:#x})",
                (egl.eglGetError)()
            ));
        }
    }
    Err(format!("no OpenGL ES context: {}", tried.join("; ")))
}

pub struct Egl {}

pub unsafe fn create_egl_context(
    egl: &mut LibEgl,
    display: *mut std::ffi::c_void,
    alpha: bool,
    sample_count: i32,
) -> Result<(EGLContext, EGLConfig, EGLDisplay), EglError> {
    let display = (egl.eglGetDisplay)(display as _);
    if display.is_null() {
        // == EGL_NO_DISPLAY
        return Err(EglError::NoDisplay);
    }

    if (egl.eglInitialize)(display, null_mut(), null_mut()) == 0 {
        return Err(EglError::InitializeFailed);
    }

    let alpha_size = if alpha { 8 } else { 0 };
    #[rustfmt::skip]
    let cfg_attributes = [
        EGL_SURFACE_TYPE, EGL_WINDOW_BIT,
        EGL_RED_SIZE, 8,
        EGL_GREEN_SIZE, 8,
        EGL_BLUE_SIZE, 8,
        EGL_ALPHA_SIZE, alpha_size,
        EGL_DEPTH_SIZE, 16,
        EGL_STENCIL_SIZE, 0,
        EGL_SAMPLES, sample_count as u32,
        EGL_NONE,
    ];
    let mut available_cfgs: Vec<EGLConfig> = vec![null_mut(); 32];
    let mut cfg_count = 0;

    (egl.eglChooseConfig)(
        display,
        cfg_attributes.as_ptr() as _,
        available_cfgs.as_ptr() as _,
        32,
        &mut cfg_count as *mut _ as *mut _,
    );
    assert!(cfg_count > 0);
    assert!(cfg_count <= 32);

    // find config with 8-bit rgb buffer if available, ndk sample does not trust egl spec
    let mut config: EGLConfig = null_mut();
    let mut exact_cfg_found = false;
    for c in &mut available_cfgs[0..cfg_count] {
        let mut r: i32 = 0;
        let mut g: i32 = 0;
        let mut b: i32 = 0;
        let mut a: i32 = 0;
        let mut d: i32 = 0;
        if (egl.eglGetConfigAttrib)(display, *c, EGL_RED_SIZE as _, &mut r) == 1
            && (egl.eglGetConfigAttrib)(display, *c, EGL_GREEN_SIZE as _, &mut g) == 1
            && (egl.eglGetConfigAttrib)(display, *c, EGL_BLUE_SIZE as _, &mut b) == 1
            && (egl.eglGetConfigAttrib)(display, *c, EGL_ALPHA_SIZE as _, &mut a) == 1
            && (egl.eglGetConfigAttrib)(display, *c, EGL_DEPTH_SIZE as _, &mut d) == 1
            && r == 8
            && g == 8
            && b == 8
            && (alpha_size == 0 || a == alpha_size as _)
            && d == 16
        {
            exact_cfg_found = true;
            config = *c;
            break;
        }
    }
    if !exact_cfg_found {
        config = available_cfgs[0];
    }
    let ctx_attributes = [EGL_CONTEXT_CLIENT_VERSION, 2, EGL_NONE];
    let context = (egl.eglCreateContext)(
        display,
        config,
        /* EGL_NO_CONTEXT */ null_mut(),
        ctx_attributes.as_ptr() as _,
    );
    if context.is_null() {
        return Err(EglError::CreateContextFailed);
    }

    Ok((context, config, display))
}
