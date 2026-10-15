//! Small, scoped SDL/GLES window owner for the separate client. No system service calls.
use std::ffi::{c_char, c_int, c_void, CStr};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
extern "C" {
    fn SDL_Init(flags: u32) -> c_int;
    fn SDL_GL_SetAttribute(attr: c_int, value: c_int) -> c_int;
    fn SDL_CreateWindow(
        title: *const c_char,
        x: c_int,
        y: c_int,
        w: c_int,
        h: c_int,
        flags: u32,
    ) -> *mut c_void;
    fn SDL_GL_CreateContext(window: *mut c_void) -> *mut c_void;
    fn SDL_GL_GetDrawableSize(window: *mut c_void, w: *mut c_int, h: *mut c_int);
    fn SDL_GL_SetSwapInterval(interval: c_int) -> c_int;
    fn SDL_GL_SwapWindow(window: *mut c_void);
    fn SDL_GL_DeleteContext(ctx: *mut c_void);
    fn SDL_DestroyWindow(window: *mut c_void);
    fn SDL_Quit();
    fn SDL_GetError() -> *const c_char;
    fn glViewport(x: c_int, y: c_int, w: c_int, h: c_int);
    fn glReadPixels(
        x: c_int,
        y: c_int,
        w: c_int,
        h: c_int,
        format: u32,
        kind: u32,
        pixels: *mut c_void,
    );
    fn glGetString(name: u32) -> *const u8;
    fn glGetError() -> u32;
    fn glGetIntegerv(name: u32, values: *mut c_int);
    fn glIsEnabled(cap: u32) -> u8;
    fn native_home_poll(event: *mut Event) -> c_int;
    fn native_home_keyboard_status(window: *mut c_void, status: *mut KeyboardStatus);
    fn native_home_text_start(window: *mut c_void) -> c_int;
    fn native_home_text_stop();
    fn native_home_text_flush();
    #[cfg(test)]
    fn native_home_test_text_decode(
        composition: c_int,
        text: *const u8,
        length: usize,
        event: *mut Event,
    ) -> c_int;
    #[cfg(target_arch = "arm")]
    fn native_home_test_text_push(text: *const u8, length: usize) -> c_int;
}
#[derive(Debug, serde::Serialize)]
#[repr(C)]
pub struct Event {
    pub kind: u32,
    pub key: u32,
    pub repeat: u32,
    pub source: u32,
    pub scan: i32,
    pub x: i32,
    pub y: i32,
    pub wheel: i32,
    #[serde(skip)]
    pub text: [u8; 256],
    pub edit_start: i32,
    pub edit_length: i32,
}
const _: () = assert!(std::mem::size_of::<Event>() == 296);
impl Default for Event {
    fn default() -> Self {
        Self {
            kind: 0,
            key: 0,
            repeat: 0,
            source: 0,
            scan: 0,
            x: 0,
            y: 0,
            wheel: 0,
            text: [0; 256],
            edit_start: 0,
            edit_length: 0,
        }
    }
}
impl Event {
    pub fn text(&self) -> Option<&str> {
        if !matches!(self.kind, 10 | 11) {
            return None;
        }
        let end = self.text.iter().position(|b| *b == 0)?;
        std::str::from_utf8(&self.text[..end]).ok()
    }
}
#[derive(Default, Debug, serde::Serialize)]
#[repr(C)]
pub struct KeyboardStatus {
    pub support: u32,
    pub active: u32,
    pub panel_flag: u32,
    pub focused: u32,
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub reserved: u32,
}
const _: () = assert!(std::mem::size_of::<KeyboardStatus>() == 32);
pub fn poll() -> Option<Event> {
    let mut event = Event::default();
    (unsafe { native_home_poll(&mut event) } != 0).then_some(event)
}
fn error() -> String {
    unsafe {
        CStr::from_ptr(SDL_GetError())
            .to_string_lossy()
            .into_owned()
    }
}
/// Read-only diagnostic state from this application's current GL context.
#[derive(Debug, serde::Serialize)]
pub struct GraphicsState {
    pub scissor_enabled: bool,
    pub scissor_box: [i32; 4],
    pub viewport: [i32; 4],
    pub framebuffer: i32,
    pub error: u32,
}
pub struct Window {
    window: *mut c_void,
    context: *mut c_void,
    pub renderer: String,
    pub swap_interval_request_accepted: bool,
    text_owned: std::cell::Cell<bool>,
}
impl Window {
    pub fn new(visible: bool) -> Result<Self, Box<dyn std::error::Error>> {
        let identity = if cfg!(target_arch = "arm") {
            crate::identity::dashboard(plx_base::paths::app_id())
                .ok_or("Unexpected dashboard identity")?
        } else {
            crate::identity::PREVIEW
        };
        Self::create(visible, identity)
    }
    pub fn launch_target() -> Result<Self, Box<dyn std::error::Error>> {
        Self::create(true, crate::identity::TARGET)
    }
    fn create(visible: bool, expected_id: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let device = cfg!(target_arch = "arm");
        if device && (unsafe { libc::geteuid() } == 0 || plx_base::paths::app_id() != expected_id) {
            return Err("Refusing root or unexpected package identity".into());
        }
        if device {
            // SDL_webOS.h's app-local Back opt-in, also used by upstream main.c. Home,
            // power and volume policies are deliberately left to LG.
            std::env::set_var("SDL_WEBOS_ACCESS_POLICY_KEYS_BACK", "true");
        }
        unsafe {
            let limit = libc::rlimit {
                rlim_cur: 0,
                rlim_max: 0,
            };
            libc::setrlimit(libc::RLIMIT_CORE, &limit);
            if SDL_Init(0x20) != 0 {
                return Err(error().into());
            }
            for (attr, value) in [
                (21, if device { 4 } else { 1 }),
                (17, if device { 2 } else { 4 }),
                (18, if device { 0 } else { 1 }),
            ] {
                if SDL_GL_SetAttribute(attr, value) != 0 {
                    SDL_Quit();
                    return Err(error().into());
                }
            }
            let window = SDL_CreateWindow(
                c"Luma Home Preview".as_ptr(),
                0,
                0,
                1920,
                1080,
                0x2 | if visible { 0x4 } else { 0x8 },
            );
            if window.is_null() {
                SDL_Quit();
                return Err(error().into());
            }
            let context = SDL_GL_CreateContext(window);
            if context.is_null() {
                SDL_DestroyWindow(window);
                SDL_Quit();
                return Err(error().into());
            }
            let mut owner = Self {
                window,
                context,
                renderer: String::new(),
                swap_interval_request_accepted: false,
                text_owned: std::cell::Cell::new(false),
            };
            let (mut w, mut h) = (0, 0);
            SDL_GL_GetDrawableSize(window, &mut w, &mut h);
            if (w, h) != (1920, 1080) {
                return Err(format!("Unexpected drawable {w}x{h}").into());
            }
            let ptr = glGetString(0x1f01);
            if ptr.is_null() {
                return Err("GL renderer unavailable".into());
            }
            owner.renderer = CStr::from_ptr(ptr.cast()).to_string_lossy().into_owned();
            glViewport(0, 0, w, h);
            owner.swap_interval_request_accepted = SDL_GL_SetSwapInterval(1) == 0;
            plx_gfx::gfx::init_gl();
            plx_gfx::gfx::init_image();
            plx_gfx::text::init_text();
            Ok(owner)
        }
    }
    pub fn keyboard_status(&self) -> KeyboardStatus {
        let mut state = KeyboardStatus::default();
        unsafe { native_home_keyboard_status(self.window, &mut state) };
        state
    }
    pub fn text_start(&self) -> bool {
        if self.text_owned.get() {
            return true;
        }
        unsafe {
            native_home_text_flush();
        }
        let started = unsafe { native_home_text_start(self.window) } != 0;
        self.text_owned.set(started);
        started
    }
    #[cfg(target_arch = "arm")]
    pub fn diagnostic_text_flush(&self) {
        if plx_base::paths::app_id() == crate::identity::PROOF {
            unsafe {
                native_home_text_flush();
            }
        }
    }
    #[cfg(target_arch = "arm")]
    pub fn diagnostic_text_start_prepared(&self) -> bool {
        if plx_base::paths::app_id() != crate::identity::PROOF {
            return false;
        }
        let started = unsafe { native_home_text_start(self.window) } != 0;
        self.text_owned.set(started);
        started
    }
    pub fn text_stop(&self) {
        if self.text_owned.replace(false) {
            unsafe {
                native_home_text_stop();
            }
        }
    }
    #[cfg(target_arch = "arm")]
    pub fn diagnostic_text(&self, text: &str) -> bool {
        if plx_base::paths::app_id() != crate::identity::PROOF || !self.text_owned.get() {
            return false;
        }
        unsafe { native_home_test_text_push(text.as_ptr(), text.len()) != 0 }
    }
    pub fn swap(&self) {
        unsafe {
            SDL_GL_SwapWindow(self.window);
        }
    }
    pub fn graphics_state(&self) -> GraphicsState {
        let mut state = GraphicsState {
            scissor_enabled: false,
            scissor_box: [0; 4],
            viewport: [0; 4],
            framebuffer: 0,
            error: 0,
        };
        // GLenum/GLint/GLboolean signatures and constants match the actual GLES2 SDK header.
        // This is diagnostic-only, on the window/GL thread; it changes no rendering policy.
        unsafe {
            state.scissor_enabled = glIsEnabled(0x0C11) != 0;
            glGetIntegerv(0x0C10, state.scissor_box.as_mut_ptr());
            glGetIntegerv(0x0BA2, state.viewport.as_mut_ptr());
            glGetIntegerv(0x8CA6, &mut state.framebuffer);
            state.error = glGetError();
        }
        state
    }
    pub fn capture(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let mut rgba = vec![0_u8; 1920 * 1080 * 4];
        unsafe {
            glReadPixels(0, 0, 1920, 1080, 0x1908, 0x1401, rgba.as_mut_ptr().cast());
            let e = glGetError();
            if e != 0 {
                return Err(format!("Framebuffer error {e:#x}").into());
            }
        }
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o640)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(path)?;
        write!(file, "P6\n1920 1080\n255\n")?;
        let mut row = vec![0_u8; 1920 * 3];
        for y in (0..1080).rev() {
            for x in 0..1920 {
                row[x * 3..x * 3 + 3]
                    .copy_from_slice(&rgba[(y * 1920 + x) * 4..(y * 1920 + x) * 4 + 3]);
            }
            file.write_all(&row)?;
        }
        Ok(())
    }
}
impl Drop for Window {
    fn drop(&mut self) {
        self.text_stop();
        unsafe {
            SDL_GL_DeleteContext(self.context);
            SDL_DestroyWindow(self.window);
            SDL_Quit();
        }
    }
}

#[cfg(test)]
mod text_abi_tests {
    use super::*;
    #[test]
    fn typed_c_decoder_keeps_utf8_and_composition_distinct_without_host_event_injection() {
        for text in ["hello", "Écran", "電視", "е́"] {
            let mut e = Event::default();
            assert_eq!(
                unsafe { native_home_test_text_decode(0, text.as_ptr(), text.len(), &mut e) },
                1
            );
            assert_eq!(e.kind, 10);
            assert_eq!(e.text(), Some(text));
            let logged = serde_json::to_string(&e).unwrap();
            assert!(!logged.contains(text));
            assert_eq!(
                unsafe { native_home_test_text_decode(1, text.as_ptr(), text.len(), &mut e) },
                1
            );
            assert_eq!(e.kind, 11);
            assert_eq!((e.edit_start, e.edit_length), (1, 2));
            assert_eq!(e.text(), Some(text));
        }
        let text = [b'x'; 32];
        let mut e = Event::default();
        assert_eq!(
            unsafe { native_home_test_text_decode(0, text.as_ptr(), text.len(), &mut e) },
            0
        );
        e.kind = 10;
        e.text = [0xff; 256];
        assert!(e.text().is_none());
        e.text[2] = 0;
        assert!(e.text().is_none());
    }
}
