use anyhow::Result;
use quickjs_wasm_rs::{JSContextRef, JSValue, JSValueRef};
use std::io::Write;

#[link(wasm_import_module = "wasi_experimental_http")]
extern "C" {
    fn req(
        url_ptr: u32,
        url_len: u32,
        method_ptr: u32,
        method_len: u32,
        req_headers_ptr: u32,
        req_headers_len: u32,
        req_body_ptr: u32,
        req_body_len: u32,
        status_code_ptr: u32,
        res_handle_ptr: u32,
    ) -> u32;
}

/// set quickjs globals
pub fn set_quickjs_globals(context: &JSContextRef) -> anyhow::Result<()> {
    let console_log_callback = context.wrap_callback(console_log_to(std::io::stdout()))?;
    let console_error_callback = context.wrap_callback(console_log_to(std::io::stderr()))?;
    let fletch_callback = context.wrap_callback(fetch_callback())?;

    let console_object = context.object_value()?;
    console_object.set_property("log", console_log_callback)?;
    console_object.set_property("error", console_error_callback)?;

    let global = context.global_object()?;
    global.set_property("console", console_object)?;
    global.set_property("fletch", fletch_callback)?;

    Ok(())
}

/// console_log_to is used to allow the javascript functions console.log and console.error to
/// log to the stdout and stderr respectively.
fn console_log_to<T>(
    mut stream: T,
) -> impl FnMut(&JSContextRef, JSValueRef, &[JSValueRef]) -> Result<JSValue>
where
    T: Write + 'static,
{
    move |_ctx: &JSContextRef, _this: JSValueRef, args: &[JSValueRef]| {
        // Write full string to in-memory destination before writing to stream since each write call to the stream
        // will invoke a hostcall.
        let mut log_line = String::new();
        for (i, arg) in args.iter().enumerate() {
            if i != 0 {
                log_line.push(' ');
            }
            let line = arg.to_string();
            log_line.push_str(&line);
        }

        writeln!(stream, "{log_line}")?;

        Ok(JSValue::Undefined)
    }
}

fn fetch_callback() -> impl FnMut(&JSContextRef, JSValueRef, &[JSValueRef]) -> Result<JSValue> {
    move |_ctx: &JSContextRef, _this: JSValueRef, args: &[JSValueRef]| {
        // Check if there are at least four arguments (the URL, method, body, and headers)
        if args.len() < 4 {
            return Err(anyhow::anyhow!("fetch requires at least four arguments"));
        }

        // Convert the arguments to strings
        let url = args[0].to_string();
        let method = args[1].to_string();
        let body = args[2].to_string();
        let headers = args[3].to_string();

        // Convert the strings to bytes and get the pointers and lengths
        let url_bytes = url.as_bytes();
        let url_ptr = url_bytes.as_ptr() as u32;
        let url_len = url_bytes.len() as u32;

        let method_bytes = method.as_bytes();
        let method_ptr = method_bytes.as_ptr() as u32;
        let method_len = method_bytes.len() as u32;

        let body_bytes = body.as_bytes();
        let body_ptr = body_bytes.as_ptr() as u32;
        let body_len = body_bytes.len() as u32;

        let headers_bytes = headers.as_bytes();
        let headers_ptr = headers_bytes.as_ptr() as u32;
        let headers_len = headers_bytes.len() as u32;

        // Create uninitialized memory for the status code and response handle
        let mut status_code_ptr = std::mem::MaybeUninit::<u16>::uninit();
        let mut res_handle_ptr = std::mem::MaybeUninit::<u32>::uninit();

        // Call the req function

        let res = unsafe {
            req(
                url_ptr,
                url_len,
                method_ptr,
                method_len,
                headers_ptr,
                headers_len,
                body_ptr,
                body_len,
                status_code_ptr.as_mut_ptr() as u32,
                res_handle_ptr.as_mut_ptr() as u32,
            )
        };

        if res != 0 {
            return Err(anyhow::anyhow!("fetch failed"));
        }
        let status_code = unsafe { status_code_ptr.assume_init() };
        let res_handle = unsafe { res_handle_ptr.assume_init() };

        // Return the result as a JSValue
        Ok(JSValue::from(format!(
            "fetch result: status code {}, response handle {}",
            status_code, res_handle
        )))
    }
}
