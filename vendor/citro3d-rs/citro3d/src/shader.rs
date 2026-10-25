//! Functionality for parsing and using PICA200 shaders on the 3DS. This module
//! does not compile shaders, but enables using pre-compiled shaders at runtime.
//!
//! For more details about the PICA200 compiler / shader language, see
//! documentation for <https://github.com/devkitPro/picasso>.

use std::borrow::Cow;
use std::error::Error;
use std::ffi::CString;
use std::mem::MaybeUninit;
use std::rc::Rc;

use crate::uniform;

/// A PICA200 shader program. It may have one or both of:
///
/// * A [vertex](Type::Vertex) shader [`Library`]
/// * A [geometry](Type::Geometry) shader [`Library`]
///
/// The PICA200 does not support user-programmable fragment shaders.
#[doc(alias = "shaderProgram_s")]
#[must_use]
pub struct Program {
    program: ctru_sys::shaderProgram_s,
    _vsh: Entrypoint,
    gsh: Option<Entrypoint>,
}

impl Program {
    /// Create a new shader program from a vertex shader.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * the shader program cannot be initialized
    /// * the input shader is not a vertex shader or is otherwise invalid
    #[doc(alias = "shaderProgramInit")]
    #[doc(alias = "shaderProgramSetVsh")]
    pub fn new(mut vertex_shader: Entrypoint) -> Result<Self, ctru::Error> {
        let mut program = unsafe {
            let mut program = MaybeUninit::uninit();
            let result = ctru_sys::shaderProgramInit(program.as_mut_ptr());
            if result != 0 {
                return Err(ctru::Error::from(result));
            }
            program.assume_init()
        };

        let ret = unsafe { ctru_sys::shaderProgramSetVsh(&mut program, vertex_shader.as_raw()) };

        if ret == 0 {
            Ok(Self {
                program,
                _vsh: vertex_shader,
                gsh: None,
            })
        } else {
            Err(ctru::Error::from(ret))
        }
    }

    /// Set the geometry shader for a given program.
    ///
    /// # Errors
    ///
    /// Returns an error if the input shader is not a geometry shader or is
    /// otherwise invalid.
    #[doc(alias = "shaderProgramSetGsh")]
    pub fn set_geometry_shader(
        &mut self,
        mut geometry_shader: Entrypoint,
        stride: u8,
    ) -> Result<(), ctru::Error> {
        let ret = unsafe {
            ctru_sys::shaderProgramSetGsh(&mut self.program, geometry_shader.as_raw(), stride)
        };

        if ret == 0 {
            self.gsh = Some(geometry_shader);
            Ok(())
        } else {
            Err(ctru::Error::from(ret))
        }
    }

    /// Get the index of a uniform in the vertex shader by name.
    ///
    /// # Errors
    ///
    /// * If the given `name` contains a null byte
    /// * If a uniform with the given `name` could not be found
    #[doc(alias = "shaderInstanceGetUniformLocation")]
    pub fn get_vertex_uniform(&self, name: &str) -> crate::Result<uniform::Index> {
        let vertex_instance = unsafe { (*self.as_raw()).vertexShader };
        assert!(
            !vertex_instance.is_null(),
            "vertex shader should never be null!"
        );

        let name = CString::new(name)?;

        let idx =
            unsafe { ctru_sys::shaderInstanceGetUniformLocation(vertex_instance, name.as_ptr()) };

        if idx < 0 {
            Err(crate::Error::NotFound)
        } else {
            Ok((idx as u8).into())
        }
    }

    /// Get the index of a uniform in the geometry shader by name.
    ///
    /// # Errors
    ///
    /// * If a geometry shader has not been set
    /// * If the given `name` contains a null byte
    /// * If a uniform with the given `name` could not be found
    #[doc(alias = "shaderInstanceGetUniformLocation")]
    pub fn get_geometry_uniform(&self, name: &str) -> crate::Result<uniform::Index> {
        if self.gsh.is_none() {
            return Err(crate::Error::MissingProgram);
        }

        let geometry_instance = unsafe { (*self.as_raw()).geometryShader };

        let name = CString::new(name)?;

        let idx =
            unsafe { ctru_sys::shaderInstanceGetUniformLocation(geometry_instance, name.as_ptr()) };

        if idx < 0 {
            Err(crate::Error::NotFound)
        } else {
            Ok((idx as u8).into())
        }
    }

    pub(crate) fn as_raw(&self) -> *const ctru_sys::shaderProgram_s {
        &self.program
    }
}

impl Drop for Program {
    #[doc(alias = "shaderProgramFree")]
    fn drop(&mut self) {
        unsafe {
            let _ = ctru_sys::shaderProgramFree(self.as_raw().cast_mut());
        }
    }
}

/// The type of a shader.
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum Type {
    /// A vertex shader.
    Vertex = ctru_sys::GPU_VERTEX_SHADER,
    /// A geometry shader.
    Geometry = ctru_sys::GPU_GEOMETRY_SHADER,
}

impl From<Type> for u8 {
    fn from(value: Type) -> Self {
        value as u8
    }
}

/// A PICA200 Shader Library (commonly called DVLB). This can be comprised of
/// one or more [`Entrypoint`]s, but most commonly has one vertex shader and an
/// optional geometry shader.
///
/// This is the result of parsing a shader binary (`.shbin`), and the resulting
/// [`Entrypoint`]s can be used as part of a [`Program`].
#[doc(alias = "DVLB_s")]
pub struct Library {
    dvlb: *mut ctru_sys::DVLB_s,
    _bytes: Cow<'static, [u8]>,
}

impl Library {
    /// Parse a new shader library from input bytes.
    ///
    /// # Errors
    ///
    /// An error is returned if the input data does not have an alignment of 4
    /// (cannot be safely converted to `&[u32]`).
    #[doc(alias = "DVLB_ParseFile")]
    pub fn from_bytes<B: Into<Cow<'static, [u8]>>>(bytes: B) -> Result<Self, Box<dyn Error>> {
        let bytes = bytes.into();

        let aligned: &[u32] = bytemuck::try_cast_slice(&bytes)?;
        let dvlb = unsafe {
            ctru_sys::DVLB_ParseFile(
                // SAFETY: we're trusting the parse implementation doesn't mutate
                // the contents of the data. From a quick read it looks like that's
                // correct and it should just take a const arg in the API.
                aligned.as_ptr().cast_mut(),
                aligned.len().try_into()?,
            )
        };

        Ok(Self {
            dvlb,
            _bytes: bytes,
        })
    }

    /// Get the number of [`Entrypoint`]s in this shader library.
    #[must_use]
    #[doc(alias = "numDVLE")]
    pub fn len(&self) -> usize {
        unsafe { (*self.dvlb).numDVLE as usize }
    }

    /// Whether the library has any [`Entrypoint`]s or not.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get the [`Entrypoint`] at the given index, if present.
    #[must_use]
    pub fn get(self, index: usize) -> Option<Entrypoint> {
        if index < self.len() {
            Some(Entrypoint {
                ptr: unsafe { (*self.dvlb).DVLE.add(index) },
                _library: MaybeRc::Owned(self),
            })
        } else {
            None
        }
    }

    #[must_use]
    /// Get the [`Entrypoint`] at the given index, if present.
    ///
    /// Like [`Library::get`], except takes `Rc<Self>` instead of `Self`, to allow the same library
    /// to have multiple entrypoints
    pub fn get_shared(self: Rc<Self>, index: usize) -> Option<Entrypoint> {
        if index < self.len() {
            Some(Entrypoint {
                ptr: unsafe { (*self.dvlb).DVLE.add(index) },
                _library: MaybeRc::Shared(self),
            })
        } else {
            None
        }
    }

    fn as_raw(&mut self) -> *mut ctru_sys::DVLB_s {
        self.dvlb
    }
}

impl Drop for Library {
    #[doc(alias = "DVLB_Free")]
    fn drop(&mut self) {
        unsafe {
            ctru_sys::DVLB_Free(self.as_raw());
        }
    }
}

#[allow(dead_code)]
enum MaybeRc<T> {
    Owned(T),
    Shared(Rc<T>),
}

/// A shader library entrypoint (also called DVLE). This represents either a
/// vertex or a geometry shader.
pub struct Entrypoint {
    ptr: *mut ctru_sys::DVLE_s,
    _library: MaybeRc<Library>,
}

impl Entrypoint {
    fn as_raw(&mut self) -> *mut ctru_sys::DVLE_s {
        self.ptr
    }
}
