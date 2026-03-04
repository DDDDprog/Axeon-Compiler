//! Standalone linker driver for `axeon-ld`.
//!
//! A production-grade, self-contained linker with no external GNU dependencies.
//! Implements a GNU `ld`-compatible command-line interface that dispatches to the
//! built-in per-architecture linkers.
//!
//! ## Advanced Features (Production Grade)
//!
//! - **Self-contained**: No external GNU tools required (no gcc, ld, ar)
//! - **Multi-architecture**: x86-64, i686, ARM64 (AArch64), RISC-V support
//! - **ELF Support**: Full ELF32/ELF64 executable and shared library generation
//! - **Dynamic Linking**: Full dynamic linking support with PLT/GOT
//! - **Static Linking**: Static archive (.a) handling
//! - **Symbol Resolution**: Automatic library search and symbol resolution
//! - **Link-time Optimization**: Section garbage collection (--gc-sections)
//! - **Position Independent**: PIE (Position Independent Executable) support
//! - **Debug Info**: DWARF debug info preservation and stripping
//! - **Symbol Stripping**: Full and debug-only symbol stripping
//! - **Entry Point**: Custom entry point specification
//! - **Section Management**: Custom sections, mergeable sections, TLS
//! - **Relocations**: Full relocation processing for all supported architectures
//! - **Symbol Versioning**: Version scripts and symbol aliases
//! - **Run-time Relocations**: RELA/REL relocations support

use crate::backend::Target;

/// Result of `LinkerDriver::run()`.
pub type LinkResult = Result<(), String>;

/// Parsed linker configuration derived from command-line arguments.
struct LinkerConfig {
    /// Target architecture.
    target: Target,
    /// Output executable path.
    output: String,
    /// Input object/archive files.
    inputs: Vec<String>,
    /// Flags to be passed to the underlying linker.
    user_args: Vec<String>,
    /// Whether to link statically.
    is_static: bool,
    /// Whether to produce a shared library.
    is_shared: bool,
    /// Whether to omit standard library linking.
    is_nostdlib: bool,
    /// Print verbose invocation info.
    verbose: bool,
}

impl LinkerConfig {
    fn new() -> Self {
        Self {
            target: host_target(),
            output: "a.out".to_string(),
            inputs: Vec::new(),
            user_args: Vec::new(),
            is_static: false,
            is_shared: false,
            is_nostdlib: false,
            verbose: false,
        }
    }
}

/// Determine the host target.
fn host_target() -> Target {
    #[cfg(target_arch = "x86_64")]
    {
        Target::X86_64
    }
    #[cfg(target_arch = "aarch64")]
    {
        Target::Aarch64
    }
    #[cfg(target_arch = "riscv64")]
    {
        Target::Riscv64
    }
    #[cfg(not(any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "riscv64"
    )))]
    {
        Target::X86_64
    }
}

/// Map a machine triple to a `Target`.
fn target_from_triple(triple: &str) -> Option<Target> {
    let t = triple.to_lowercase();
    if t.starts_with("x86_64") || t.starts_with("amd64") {
        Some(Target::X86_64)
    } else if t.starts_with("i686") || t.starts_with("i386") || t.starts_with("x86") {
        Some(Target::I686)
    } else if t.starts_with("aarch64") || t.starts_with("arm64") {
        Some(Target::Aarch64)
    } else if t.starts_with("riscv64") {
        Some(Target::Riscv64)
    } else {
        None
    }
}

fn print_version() {
    println!("GNU ld (Axeon built-in) 2.43");
    println!("Copyright (C) 2025 Axeon Project");
    println!("This program is free software; you may redistribute it under the terms of");
    println!("the GNU General Public License version 3 or (at your option) a later version.");
    println!("This program has absolutely no warranty.");
    println!("Supported emulations: elf_x86_64, elf_i386, elf64_lriscv, aarch64linux");
}

fn print_help(prog: &str) {
    println!("Usage: {prog} [OPTIONS] [FILE...]");
    println!();
    println!("Axeon built-in linker — Production Grade");
    println!();
    println!("General options:");
    println!("  -o FILE               Write output to FILE (default: a.out)");
    println!("  -lLIB                 Search for library LIB");
    println!("  -LDIR                 Add DIR to library search path");
    println!("  -EL                  Link little-endian (default)");
    println!("  -EB                  Link big-endian");
    println!();
    println!("Linking style:");
    println!("  -shared               Create a shared library");
    println!("  -static               Link statically");
    println!("  -nostdlib             Only use libraries provided on the command line");
    println!("  -relocatable          Produce relocatable object file (-r)");
    println!("  -Pie                  Produce position independent executable");
    println!("  -no-pie               Disable position independent executable");
    println!();
    println!("Optimization:");
    println!("  --gc-sections         Remove unused sections");
    println!("  --strip-all           Strip all symbols");
    println!("  --strip-debug         Strip debug symbols only");
    println!("  --as-needed           Only link when needed (default)");
    println!("  --no-as-needed        Always link even when not needed");
    println!("  --whole-archive       Include all objects from archive");
    println!("  --no-whole-archive    Disable whole archive (default)");
    println!();
    println!("Entry point:");
    println!("  -e ADDRESS            Set entry point address");
    println!("  -e SYMBOL            Set entry point symbol (default: _start)");
    println!();
    println!("Symbol management:");
    println!("  -u SYMBOL             Force symbol to be undefined");
    println!("  --defsym SYM=EXP      Define symbol");
    println!("  --wrap SYMBOL         Wrap symbol references");
    println!();
    println!("Output format:");
    println!("  --target=TRIPLE       Set output file format");
    println!("  -oformat=binary       Raw binary output");
    println!("  -oformat=elf64-x86-64 ELF64 x86_64 output");
    println!();
    println!("Debugging:");
    println!("  -g                    Preserve debug info");
    println!("  --debug               Enable debug info");
    println!();
    println!("Other options:");
    println!("  -v, --verbose         Verbose mode");
    println!("  --version             Print version and exit");
    println!("  --help, -h            Print this help and exit");
}

/// Parse command-line arguments.
fn parse_args(args: &[String]) -> Result<Option<LinkerConfig>, String> {
    let prog = args.first().map(|s| s.as_str()).unwrap_or("axeon-ld");
    let mut cfg = LinkerConfig::new();

    let mut i = 1;
    while i < args.len() {
        let arg = args[i].as_str();
        match arg {
            "--version" => {
                print_version();
                return Ok(None);
            }
            "--help" | "-h" => {
                print_help(prog);
                return Ok(None);
            }
            "-v" | "--verbose" => {
                cfg.verbose = true;
                cfg.user_args.push(arg.to_string());
            }
            "-o" => {
                i += 1;
                if i < args.len() {
                    cfg.output = args[i].clone();
                } else {
                    return Err("axeon-ld: -o requires an argument".to_string());
                }
            }
            "-static" => {
                cfg.is_static = true;
                cfg.user_args.push(arg.to_string());
            }
            "-shared" => {
                cfg.is_shared = true;
                cfg.user_args.push(arg.to_string());
            }
            "-nostdlib" => {
                cfg.is_nostdlib = true;
                cfg.user_args.push(arg.to_string());
            }
            arg if arg.starts_with("--target=") => {
                let triple = &arg["--target=".len()..];
                match target_from_triple(triple) {
                    Some(t) => cfg.target = t,
                    None => return Err(format!("axeon-ld: unknown target triple '{triple}'")),
                }
            }
            arg if arg.starts_with("-l")
                || arg.starts_with("-L")
                || arg.starts_with("-Wl,")
                || arg.starts_with("--gc-sections")
                || arg.starts_with("--defsym") =>
            {
                cfg.user_args.push(arg.to_string());
            }
            arg if !arg.starts_with('-') => {
                cfg.inputs.push(arg.to_string());
            }
            unknown => {
                // Forward unknown flags to the underlying linker
                cfg.user_args.push(unknown.to_string());
            }
        }
        i += 1;
    }

    Ok(Some(cfg))
}

/// Main entry point for the standalone linker.
pub fn linker_main_inner() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let cfg = match parse_args(&args)? {
        None => return Ok(()),
        Some(c) => c,
    };

    if cfg.inputs.is_empty() && !cfg.user_args.iter().any(|a| a.contains("-l")) {
        return Err("axeon-ld: no input files".to_string());
    }

    let input_refs: Vec<&str> = cfg.inputs.iter().map(|s| s.as_str()).collect();

    if cfg.verbose {
        eprintln!(
            "axeon-ld: target={}, output={}",
            cfg.target.triple(),
            cfg.output
        );
    }

    cfg.target
        .link_with_args(&input_refs, &cfg.output, &cfg.user_args)
}

pub fn linker_main() {
    const STACK_SIZE: usize = 16 * 1024 * 1024;
    let builder = std::thread::Builder::new()
        .name("axeon-ld".to_string())
        .stack_size(STACK_SIZE);

    let handle = builder
        .spawn(linker_main_inner)
        .expect("axeon-ld: failed to spawn linker thread");

    match handle.join() {
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
            eprintln!("axeon-ld: error: {e}");
            std::process::exit(1);
        }
        Err(_) => {
            eprintln!("axeon-ld: internal error (thread panicked)");
            std::process::exit(1);
        }
    }
}
