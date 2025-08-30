#![allow(non_camel_case_types)]
use std::ffi::{c_char, CStr, CString};
use std::path::{Path, PathBuf};
use std::ptr;

mod compiler;
mod download;
mod query;
mod world;

use ecow::EcoString;
use typst::diag::{SourceDiagnostic, StrResult, Warned};
use typst::foundations::{Dict, Value};
use world::SystemWorld;

// This represents the stateful compiler in Rust.
pub struct Compiler(SystemWorld);

#[repr(C)]
pub struct Buffer {
    pub ptr: *mut u8,
    pub len: usize,
}

#[repr(C)]
pub struct Warning {
    pub message: *mut c_char,
}

#[repr(C)]
pub struct CompileResult {
    pub buffers: *mut Buffer,
    pub buffers_len: usize,
    pub warnings: *mut Warning,
    pub warnings_len: usize,
    pub error: *mut c_char,
}

impl Default for CompileResult {
    fn default() -> Self {
        Self {
            buffers: ptr::null_mut(),
            buffers_len: 0,
            warnings: ptr::null_mut(),
            warnings_len: 0,
            error: ptr::null_mut(),
        }
    }
}

#[no_mangle]
pub extern "C" fn create_compiler(
    input: *const c_char,
    font_paths: *const *const c_char,
    font_paths_len: usize,
    sys_inputs: *const c_char,
    ignore_system_fonts: bool,
) -> *mut Compiler {
    let input_str = unsafe { CStr::from_ptr(input).to_str().unwrap_or("") };
    let sys_inputs_str = unsafe { CStr::from_ptr(sys_inputs).to_str().unwrap_or("{}") };

    let font_paths_vec: Vec<PathBuf> = unsafe {
        std::slice::from_raw_parts(font_paths, font_paths_len)
            .iter()
            .map(|&p| PathBuf::from(CStr::from_ptr(p).to_str().unwrap_or("")))
            .collect()
    };

    let inputs: Dict = serde_json::from_str(sys_inputs_str).unwrap_or_default();

    let root = std::path::PathBuf::from(".");
    match SystemWorld::new(root, &font_paths_vec, inputs, input_str, !ignore_system_fonts) {
        Ok(world) => Box::into_raw(Box::new(Compiler(world))),
        Err(_) => ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn free_compiler(compiler: *mut Compiler) {
    if !compiler.is_null() {
        unsafe { let _ = Box::from_raw(compiler); }
    }
}

fn compile_inner(
    world: &mut SystemWorld,
    format: &str,
    ppi: f32,
) -> StrResult<(Vec<Vec<u8>>, Vec<SourceDiagnostic>)> {
    let (document, warnings) = match typst::compile(world) {
        Warned { output, warnings } => {
            let doc = output.map_err(|errors| EcoString::from(format!("{:?}", errors)))?;
            (doc, warnings.to_vec())
        }
    };

    let buffers = compiler::export(&document, format, ppi, &[])?;
    Ok((buffers, warnings))
}

#[no_mangle]
pub extern "C" fn compile(
    compiler: *mut Compiler,
    format: *const c_char,
    ppi: f32,
) -> CompileResult {
    let compiler = unsafe { &mut *compiler };
    let format_str = unsafe { CStr::from_ptr(format).to_str().unwrap_or("pdf") };

    match compile_inner(&mut compiler.0, format_str, ppi) {
        Ok((buffers, warnings)) => {
            let mut c_buffers: Vec<Buffer> = buffers.into_iter().map(|mut b| {
                b.shrink_to_fit();
                let buffer = Buffer { ptr: b.as_mut_ptr(), len: b.len() };
                std::mem::forget(b);
                buffer
            }).collect();

            let mut c_warnings: Vec<Warning> = warnings.into_iter().map(|w| {
                let message = CString::new(w.message.to_string()).unwrap().into_raw();
                Warning { message }
            }).collect();

            c_buffers.shrink_to_fit();
            c_warnings.shrink_to_fit();

            let result = CompileResult {
                buffers: c_buffers.as_mut_ptr(),
                buffers_len: c_buffers.len(),
                warnings: c_warnings.as_mut_ptr(),
                warnings_len: c_warnings.len(),
                error: ptr::null_mut(),
            };

            std::mem::forget(c_buffers);
            std::mem::forget(c_warnings);

            result
        }
        Err(err) => {
            let error_str = CString::new(err.to_string()).unwrap();
            CompileResult { error: error_str.into_raw(), ..Default::default() }
        }
    }
}

#[no_mangle]
pub extern "C" fn free_compile_result(result: CompileResult) {
    unsafe {
        if !result.buffers.is_null() {
            let buffers = Vec::from_raw_parts(result.buffers, result.buffers_len, result.buffers_len);
            for buffer in buffers {
                let _ = Vec::from_raw_parts(buffer.ptr, buffer.len, buffer.len);
            }
        }
        if !result.warnings.is_null() {
            let warnings = Vec::from_raw_parts(result.warnings, result.warnings_len, result.warnings_len);
            for warning in warnings {
                let _ = CString::from_raw(warning.message);
            }
        }
        if !result.error.is_null() {
            let _ = CString::from_raw(result.error);
        }
    }
}

#[no_mangle]
pub extern "C" fn query(
    compiler: *mut Compiler,
    selector: *const c_char,
    field: *const c_char,
    one: bool,
) -> *mut c_char {
    let compiler = unsafe { &mut *compiler };
    let selector_str = unsafe { CStr::from_ptr(selector).to_str().unwrap_or("") };
    let field_str = if field.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(field).to_str().unwrap_or("").to_string() })
    };

    let command = query::QueryCommand {
        selector: selector_str.to_string(),
        field: field_str,
        one,
        format: query::SerializationFormat::Json,
    };

    match query::query(&mut compiler.0, &command) {
        Ok(result) => CString::new(result).unwrap().into_raw(),
        Err(err) => CString::new(format!("Error during query: {}", err)).unwrap().into_raw(),
    }
}

#[no_mangle]
pub extern "C" fn free_string(s: *mut c_char) {
    unsafe {
        if s.is_null() { return }
        let _ = CString::from_raw(s);
    }
}
