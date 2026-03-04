#![recursion_limit = "512"]

pub mod assembler;
pub mod backend;
pub(crate) mod common;
pub mod driver;
pub(crate) mod frontend;
pub(crate) mod ir;
pub mod linker;
pub(crate) mod passes;

/// Shared entry point for all compiler binaries. Spawns the real work on a
/// thread with a large stack so deeply recursive C files don't overflow.
pub fn compiler_main() {
    const STACK_SIZE: usize = 64 * 1024 * 1024; // 64 MB
    let builder = std::thread::Builder::new().stack_size(STACK_SIZE);
    let handler = builder
        .spawn(|| {
            let args: Vec<String> = std::env::args().collect();
            let mut driver = driver::Driver::new();
            if driver.parse_cli_args(&args)? {
                return Ok(());
            }
            if !driver.has_input_files() {
                return Err("no input files".to_string());
            }
            driver.run()
        })
        .expect("failed to spawn main thread");

    match handler.join() {
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
            eprintln!("axeon: error: {}", e);
            std::process::exit(1);
        }
        Err(e) => {
            if let Some(s) = e.downcast_ref::<&str>() {
                eprintln!("internal error: {}", s);
            } else if let Some(s) = e.downcast_ref::<String>() {
                eprintln!("internal error: {}", s);
            } else {
                eprintln!("internal error (thread panicked)");
            }
            std::process::exit(1);
        }
    }
}
