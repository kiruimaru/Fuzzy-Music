fn main() {
    let target = std::env::args().last().unwrap();
    if target.contains("hang") {
        std::thread::sleep(std::time::Duration::from_secs(10));
    }
    if target.contains("fail") {
        eprintln!("token=SECRET");
        std::process::exit(1);
    }
    if target.starts_with("scsearch") {
        if target.contains("empty") {
            println!("{{\"entries\":[]}}");
        } else {
            println!(
                "{{\"entries\":[{{\"id\":\"1\",\"title\":\"Example\",\"uploader\":\"Artist\",\"webpage_url\":\"https://soundcloud.com/a/b\"}}]}}"
            );
        }
    } else if target.starts_with("https://soundcloud.com") {
        let stream = if target.ends_with("/unplayable") {
            "fail"
        } else {
            "stream"
        };
        println!(
            "{{\"id\":\"1\",\"title\":\"Example\",\"webpage_url\":\"https://soundcloud.com/a/b\",\"url\":\"https://audio.example/{stream}\"}}"
        );
    } else {
        let endpoint = std::env::args().find_map(|arg| arg.strip_prefix("--input-ipc-server=").map(str::to_owned)).unwrap();
        serve(&endpoint);
    }
}

#[cfg(unix)]
fn serve(endpoint: &str) {
    use std::io::{BufRead, BufReader, Write};
    let listener = std::os::unix::net::UnixListener::bind(endpoint).unwrap();
    let (mut stream, _) = listener.accept().unwrap();
    let reader = stream.try_clone().unwrap();
    stream.write_all(initial().as_bytes()).unwrap();
    for line in BufReader::new(reader).lines() {
        let Ok(line) = line else { break };
        if let Some(reply) = reply(&line) { if stream.write_all(reply.as_bytes()).is_err() { break; } }
    }
}

fn initial() -> &'static str {
    "{\"event\":\"file-loaded\"}\n{\"event\":\"property-change\",\"name\":\"duration\",\"data\":180}\n{\"event\":\"property-change\",\"name\":\"seekable\",\"data\":true}\n"
}

fn reply(line: &str) -> Option<String> {
    let compact = line.replace(' ', "");
    let (name, value) = if compact.contains("\"pause\"") && compact.contains("set_property") {
        ("pause", if compact.contains("true") { "true".into() } else { "false".into() })
    } else if compact.contains("\"volume\"") && compact.contains("set_property") {
        ("volume", compact.split("\"volume\",").nth(1)?.split(']').next()?.to_string())
    } else if compact.contains("\"seek\"") {
        ("time-pos", compact.split("\"seek\",").nth(1)?.split(',').next()?.to_string())
    } else { return None };
    Some(format!("{{\"event\":\"property-change\",\"name\":\"{name}\",\"data\":{value}}}\n"))
}

#[cfg(windows)]
fn serve(endpoint: &str) {
    use std::ffi::c_void;
    type Handle = *mut c_void;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateNamedPipeW(name: *const u16, mode: u32, pipe_mode: u32, instances: u32, out_size: u32, in_size: u32, timeout: u32, security: *const c_void) -> Handle;
        fn ConnectNamedPipe(pipe: Handle, overlapped: *mut c_void) -> i32;
        fn ReadFile(handle: Handle, buffer: *mut u8, size: u32, read: *mut u32, overlapped: *mut c_void) -> i32;
        fn WriteFile(handle: Handle, buffer: *const u8, size: u32, written: *mut u32, overlapped: *mut c_void) -> i32;
        fn CloseHandle(handle: Handle) -> i32;
    }
    let name: Vec<u16> = endpoint.encode_utf16().chain(Some(0)).collect();
    unsafe {
        let pipe = CreateNamedPipeW(name.as_ptr(), 3, 0, 1, 65536, 65536, 0, std::ptr::null());
        assert_ne!(pipe as isize, -1);
        ConnectNamedPipe(pipe, std::ptr::null_mut());
        let send = |text: &str| {
            let mut written = 0;
            WriteFile(pipe, text.as_ptr(), text.len() as u32, &mut written, std::ptr::null_mut())
        };
        send(initial());
        let mut pending = String::new();
        loop {
            let mut buffer = [0u8; 4096];
            let mut read = 0;
            if ReadFile(pipe, buffer.as_mut_ptr(), buffer.len() as u32, &mut read, std::ptr::null_mut()) == 0 || read == 0 { break; }
            pending.push_str(&String::from_utf8_lossy(&buffer[..read as usize]));
            while let Some(end) = pending.find('\n') {
                let line: String = pending.drain(..=end).collect();
                if let Some(response) = reply(&line) { send(&response); }
            }
        }
        CloseHandle(pipe);
    }
}
