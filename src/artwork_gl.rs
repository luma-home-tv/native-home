//! Main-thread GL backend for the separately owned live artwork store.
//! Error observations are retained instead of silently clearing renderer errors.
use crate::artwork_textures::TextureBackend;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default, Clone)]
pub struct Diagnostics(Rc<RefCell<Metrics>>);
#[derive(Default, serde::Serialize)]
struct Metrics {
    uploads: u64,
    deletes: u64,
    upload_failures: u64,
    deletion_failures: u64,
    observed_gl_errors: u64,
    last_gl_error: u32,
    #[cfg(test)]
    next: u32,
}
impl Diagnostics {
    pub fn receipt(&self) -> serde_json::Value {
        serde_json::to_value(&*self.0.borrow()).expect("GL metrics serialize")
    }
    #[cfg(not(test))]
    fn error(&self, code: u32) {
        if code != 0 {
            let mut m = self.0.borrow_mut();
            m.observed_gl_errors += 1;
            m.last_gl_error = code;
        }
    }
}
pub struct Backend {
    diagnostics: Diagnostics,
}
impl Backend {
    pub fn new(diagnostics: Diagnostics) -> Self {
        Self { diagnostics }
    }
}
#[cfg(not(test))]
extern "C" {
    fn glGetError() -> u32;
    fn glIsTexture(texture: u32) -> u8;
}
impl TextureBackend for Backend {
    type Handle = u32;
    type Error = u32;
    fn upload(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<u32, u32> {
        #[cfg(test)]
        {
            let _ = (width, height, rgba);
            let mut m = self.diagnostics.0.borrow_mut();
            m.next += 1;
            m.uploads += 1;
            Ok(m.next)
        }
        #[cfg(not(test))]
        {
            if self.diagnostics.0.borrow().deletion_failures != 0 {
                self.diagnostics.0.borrow_mut().upload_failures += 1;
                return Err(u32::MAX - 1);
            }
            let before = unsafe { glGetError() };
            if before != 0 {
                self.diagnostics.error(before);
                self.diagnostics.0.borrow_mut().upload_failures += 1;
                return Err(before);
            }
            let id = plx_gfx::gfx::upload_rgba(0, width as i32, height as i32, rgba.as_ptr());
            let error = unsafe { glGetError() };
            if id == 0 || error != 0 {
                self.diagnostics.error(error);
                self.diagnostics.0.borrow_mut().upload_failures += 1;
                if id != 0 {
                    self.delete(id);
                }
                return Err(if error == 0 { u32::MAX } else { error });
            }
            self.diagnostics.0.borrow_mut().uploads += 1;
            Ok(id)
        }
    }
    fn delete(&mut self, handle: u32) {
        if handle == 0 {
            return;
        }
        #[cfg(not(test))]
        {
            let before = unsafe { glGetError() };
            self.diagnostics.error(before);
            plx_gfx::gfx::delete_tex(handle);
            let error = unsafe { glGetError() };
            self.diagnostics.error(error);
            if error != 0 || unsafe { glIsTexture(handle) } != 0 {
                self.diagnostics.0.borrow_mut().deletion_failures += 1;
            }
        }
        self.diagnostics.0.borrow_mut().deletes += 1;
    }
}
