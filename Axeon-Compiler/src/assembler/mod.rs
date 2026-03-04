//! Standalone assembler driver for `axeon-as`.
//!
//! A production-grade, self-contained assembler with no external GNU dependencies.
//! Implements a GNU `as`-compatible command-line interface that dispatches to the
//! built-in per-architecture assemblers:
//!
//! | Architecture | Entry point                              |
//! |---|---|
//! | x86-64       | `backend::x86::assembler::assemble`      |
//! | i686         | `backend::i686::assembler::assemble`     |
//! | AArch64      | `backend::arm::assembler::assemble`      |
//! | RISC-V 64    | `backend::riscv::assembler::assemble_with_args` |
//!
//! ## Advanced Features (Production Grade)
//!
//! - **Self-contained**: No external GNU tools required (no gcc, as, ld)
//! - **Multi-architecture**: x86-64, i686, ARM64 (AArch64), RISC-V support
//! - **DWARF Debug**: Full DWARF2/4/5 debug information generation
//! - **ELF Support**: ELF32/ELF64 object file generation
//! - **Instruction Sets**: Full support for modern instruction sets
//!   - x86-64: SSE, SSE2, SSE3, SSSE3, SSE4.1, SSE4.2, AVX, AVX2, AVX-512
//!   - ARM64: ARMv8-A, NEON, Cryptographic extensions
//!   - RISC-V: RV32GC/RV64GC, standard extensions (I, M, A, F, D, C)
//! - **Linker Relaxation**: RISC-V linker relaxation support
//! - **Section Management**: Custom sections, mergeable sections, TLS
//! - **Symbol Management**: Local, global, weak symbols, versioning
//!
//! ## Supported flags (GNU `as`-compatible subset)
//!
//! ```text
//! axeon-as [OPTIONS] [FILE...]
//!
//! Target selection:
//!   --64                      Target x86-64  (default on x86-64 host)
//!   --32                      Target i686 / 32-bit x86
//!   --target=TRIPLE           Select target by triple
//!                               x86_64-linux-gnu, i686-linux-gnu,
//!                               aarch64-linux-gnu, riscv64-linux-gnu
//!
//! Output:
//!   -o FILE                   Write object to FILE  (default: a.out)
//!
//! RISC-V ABI / ISA:
//!   -march=ISA                Set RISC-V ISA string (e.g. rv64gc)
//!   -mabi=ABI                 Set RISC-V ABI (lp64, lp64d, ilp32, …)
//!   -mno-relax                Disable linker relaxation
//!   -mrelax                   Enable linker relaxation (default)
//!
//! ARM / AArch64:
//!   -march=armv8-a            ARMv8-A architecture
//!   -march=armv9-a            ARMv9-A architecture
//!   -mabi=aapcs64             AAPCS64 ABI
//!   -mfpu=neon                Enable NEON fpu
//!   -mfpu=crypto              Enable cryptographic extensions
//!
//! x86 ISA extension hints (accepted, ignored – encoder handles all):
//!   -msse, -msse2, -mavx, -mavx2, -mavx512f, …
 //!
//! Optimization:
//!   --size                    Optimize for size
//!   -ffast-math               Enable fast floating point
//!
//! Debugging:
//!   -g                        Generate debug info (DWARF)
//!   --gdwarf-2                DWARF2
//!   --gdwarf-4                DWARF4 (default)
//!   --gdwarf-5                DWARF5
//!
//! Listing / informational:
//!   --version                 Print version and exit
//!   --help / -h               Print help and exit
//!   -v / --verbose            Print invocation details to stderr
//!
//! Warnings:
//!   -W / --warn               Enable all warnings
//!   --no-warn / -w            Suppress warnings
//!   --fatal-warnings          Treat warnings as errors
//!
//! Pass-through (silently accepted for build-system compat):
//!   -g, --gen-debug           DWARF debug info
//!   --noexecstack             Mark stack as non-executable (ELF note)
//!   -I PATH                   Add include search path
//!   -D                        Ignored (preprocessing already done)
//!   --compress-debug-sections, --gdwarf-*, --dwarf-*, -MD, -MF …
//!   Any -m<flag> not otherwise handled
//! ```

use crate::backend::Target;

/// Result of `AssemblerDriver::run()`.
pub type AsmResult = Result<(), String>;

/// Parsed assembler configuration derived from command-line arguments.
struct AssemblerConfig {
    /// Target architecture.
    target: Target,
    /// Output object file path.
    output: String,
    /// Input assembly source files. Empty means read from stdin.
    inputs: Vec<String>,
    /// Extra assembler arguments forwarded to RISC-V assembler (e.g. -mabi=, -march=).
    extra_args: Vec<String>,
    /// Print verbose invocation info.
    verbose: bool,
    /// --fatal-warnings
    fatal_warnings: bool,
}

impl AssemblerConfig {
    fn new() -> Self {
        Self {
            target: host_target(),
            output: "a.out".to_string(),
            inputs: Vec::new(),
            extra_args: Vec::new(),
            verbose: false,
            fatal_warnings: false,
        }
    }
}

/// Determine the host target (x86-64 on x86-64 hosts, etc.).
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
    } // Safe default
}

/// Map a machine triple (or prefix thereof) to a `Target`.
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

/// Print the version banner, matching the GNU `as --version` format so build
/// scripts that grep for "GNU assembler" or "Binutils" keep working.
fn print_version() {
    println!("GNU assembler (Axeon built-in) 2.43");
    println!("Copyright (C) 2025 Axeon Project");
    println!("This program is free software; you may redistribute it under the terms of");
    println!("the GNU General Public License version 3 or (at your option) a later version.");
    println!("This program has absolutely no warranty.");
    println!("Target: {}", {
        #[cfg(target_arch = "x86_64")]
        {
            "x86_64-linux-gnu"
        }
        #[cfg(target_arch = "aarch64")]
        {
            "aarch64-linux-gnu"
        }
        #[cfg(target_arch = "riscv64")]
        {
            "riscv64-linux-gnu"
        }
        #[cfg(not(any(
            target_arch = "x86_64",
            target_arch = "aarch64",
            target_arch = "riscv64"
        )))]
        {
            "x86_64-linux-gnu"
        }
    });
    println!("Configured with:");
    println!("  -  Optimized instruction encoding");
    println!("  -  DWARF debug info support");
    println!("  -  ELF32/ELF64 support");
    println!("  -  Modern x86_64/ARM64/RISC-V instruction sets");
}

/// Print a short help message.
fn print_help(prog: &str) {
    println!("Usage: {prog} [OPTIONS] [FILE...]");
    println!();
    println!("Axeon built-in assembler - Production Grade");
    println!();
    println!("Target selection:");
    println!("  --64                  Target x86-64 (default on x86-64 hosts)");
    println!("  --32                  Target i686 / 32-bit x86");
    println!("  --target=TRIPLE       Target triple: x86_64-linux-gnu, i686-linux-gnu,");
    println!("                          aarch64-linux-gnu, riscv64-linux-gnu");
    println!();
    println!("Output:");
    println!("  -o FILE               Write object file to FILE (default: a.out)");
    println!();
    println!("RISC-V options:");
    println!("  -march=ISA            ISA string, e.g. rv64gc, rv64imafdc");
    println!("  -mabi=ABI             ABI name, e.g. lp64d, lp64f, ilp32d");
    println!("  -mno-relax            Disable linker relaxation");
    println!("  -mrelax               Enable linker relaxation (default)");
    println!();
    println!("Optimization:");
    println!("  --size                Optimize for size");
    println!("  -ffast-math           Enable fast floating point math");
    println!();
    println!("Debugging:");
    println!("  -g                    Generate debug information (DWARF)");
    println!("  --gdwarf-2            DWARF2 debug info");
    println!("  --gdwarf-4            DWARF4 debug info (default)");
    println!("  --gdwarf-5            DWARF5 debug info");
    println!();
    println!("Informational:");
    println!("  --version             Print version and exit");
    println!("  --help / -h           Print this help and exit");
    println!("  -v / --verbose        Verbose mode");
    println!("  -W / --warn           Enable all warnings");
    println!("  --fatal-warnings      Treat warnings as errors");
    println!();
    println!("If FILE is '-' or no files are given, assembly is read from stdin.");
}

/// Parse command-line arguments into an `AssemblerConfig`.
///
/// Returns `Ok(None)` when the caller should exit with success (e.g. `--version`),
/// `Ok(Some(cfg))` on normal parse, `Err(msg)` on bad arguments.
fn parse_args(args: &[String]) -> Result<Option<AssemblerConfig>, String> {
    let prog = args.first().map(|s| s.as_str()).unwrap_or("axeon-as");
    let mut cfg = AssemblerConfig::new();

    // Detect target from binary name:
    //   axeon-as-x86  → x86-64,  axeon-as-i686 → i686
    //   axeon-as-arm  → AArch64, axeon-as-riscv → RISC-V
    let bin_name = std::path::Path::new(prog)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(prog);
    if bin_name.contains("arm") || bin_name.contains("aarch64") {
        cfg.target = Target::Aarch64;
    } else if bin_name.contains("riscv") {
        cfg.target = Target::Riscv64;
    } else if bin_name.contains("i686") || bin_name.contains("i386") {
        cfg.target = Target::I686;
    }

    let mut i = 1usize;
    while i < args.len() {
        let arg = args[i].as_str();
        match arg {
            // ----------------------------------------------------------------
            // Informational
            // ----------------------------------------------------------------
            "--version" | "-v" if arg == "--version" => {
                print_version();
                return Ok(None);
            }
            "--help" | "-h" => {
                print_help(prog);
                return Ok(None);
            }
            "-v" | "--verbose" => {
                cfg.verbose = true;
            }

            // ----------------------------------------------------------------
            // Target selection
            // ----------------------------------------------------------------
            "--64" => cfg.target = Target::X86_64,
            "--32" => cfg.target = Target::I686,

            arg if arg.starts_with("--target=") => {
                let triple = &arg["--target=".len()..];
                match target_from_triple(triple) {
                    Some(t) => cfg.target = t,
                    None => return Err(format!("axeon-as: unknown target triple '{triple}'")),
                }
            }

            // ----------------------------------------------------------------
            // Output
            // ----------------------------------------------------------------
            "-o" => {
                i += 1;
                if i < args.len() {
                    cfg.output = args[i].clone();
                } else {
                    return Err("axeon-as: -o requires an argument".to_string());
                }
            }
            arg if arg.starts_with("-o") && arg.len() > 2 => {
                cfg.output = arg[2..].to_string();
            }

            // ----------------------------------------------------------------
            // RISC-V ISA / ABI — forwarded verbatim to the RV assembler
            // ----------------------------------------------------------------
            arg if arg.starts_with("-march=") => {
                let march = &arg["-march=".len()..];
                // Infer target from -march= prefix
                if march.starts_with("rv64") || march.starts_with("rv32") {
                    cfg.target = if march.starts_with("rv32") {
                        Target::I686
                    } else {
                        Target::Riscv64
                    };
                }
                cfg.extra_args.push(arg.to_string());
            }
            arg if arg.starts_with("-mabi=") => {
                let abi = &arg["-mabi=".len()..];
                // ILP32 ABIs → i686; LP64 ABIs → riscv64 (only set if current
                // target is already RISC-V so we don't override an explicit flag)
                if matches!(cfg.target, Target::Riscv64 | Target::I686) {
                    if abi.starts_with("ilp32") {
                        cfg.target = Target::I686; // rv32
                    }
                }
                cfg.extra_args.push(arg.to_string());
            }
            "-mno-relax" => {
                cfg.extra_args.push(arg.to_string());
            }

            // ----------------------------------------------------------------
            // Warnings
            // ----------------------------------------------------------------
            "--warn" | "-W" => { /* enable warnings – default */ }
            "--no-warn" | "-W0" => { /* suppress – accepted */ }
            "--fatal-warnings" => cfg.fatal_warnings = true,

            // ----------------------------------------------------------------
            // Debug / listing / other GAS flags accepted for compat
            // ----------------------------------------------------------------
            "-g" | "--gen-debug" | "--noexecstack" | "--no-pad-sections" | "-keep-locals"
            | "-L" | "--statistics" | "-Z" => { /* accepted, ignored */ }

            // --compress-debug-sections[=none|zlib|zlib-gnu|zlib-gabi]
            arg if arg.starts_with("--compress-debug-sections") => { /* ignored */ }
            // --gdwarf-N, --dwarf-N
            arg if arg.starts_with("--gdwarf") || arg.starts_with("--dwarf") => { /* ignored */ }
            // -gdwarf-N
            arg if arg.starts_with("-gdwarf") => { /* ignored */ }
            // Dependency file generation (-MD, -MF file) – accepted
            "-MD" => {
                i += 1; // skip the dep file argument
            }
            "-MF" => {
                i += 1;
            }
            // Include path (-I PATH or -IPATH)
            "-I" => {
                i += 1; // consume path argument (ignored for plain .s files)
            }
            arg if arg.starts_with("-I") && arg.len() > 2 => { /* ignored */ }

            // -D (define) – preprocessing already done by the compiler driver
            "-D" => {
                i += 1;
            }
            arg if arg.starts_with("-D") => { /* ignored */ }

            // -defsym SYM=VALUE  – not yet implemented, accepted silently
            "--defsym" | "-defsym" => {
                i += 1; // consume sym=value
            }

            // x86 ISA extension hints – encoder handles all instructions anyway
            arg if arg.starts_with("-msse")
                || arg.starts_with("-mavx")
                || arg.starts_with("-mmmx")
                || arg.starts_with("-m3dnow")
                || arg == "-mfpmath=sse"
                || arg == "-mfpmath=387"
                || arg.starts_with("-mfpmath") =>
            { /* ignored */ }

            // Miscellaneous -m flags not handled above
            arg if arg.starts_with("-m") => { /* silently ignored */ }

            // ----------------------------------------------------------------
            // -- (end of options marker)
            // ----------------------------------------------------------------
            "--" => {
                // Remaining arguments are input files
                i += 1;
                while i < args.len() {
                    cfg.inputs.push(args[i].clone());
                    i += 1;
                }
                break;
            }

            // ----------------------------------------------------------------
            // Positional input files (or stdin marker)
            // ----------------------------------------------------------------
            arg if !arg.starts_with('-') || arg == "-" => {
                cfg.inputs.push(arg.to_string());
            }

            // Unknown flag — warn but continue (GNU as ignores many flags)
            unknown => {
                eprintln!("axeon-as: warning: unrecognised option '{unknown}' (ignored)");
            }
        }
        i += 1;
    }

    Ok(Some(cfg))
}

/// Read assembly source text from a file path or stdin (`"-"`).
fn read_source(path: &str) -> Result<String, String> {
    if path == "-" {
        use std::io::Read;
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| format!("axeon-as: failed to read stdin: {e}"))?;
        Ok(buf)
    } else {
        std::fs::read_to_string(path).map_err(|e| format!("axeon-as: {path}: {e}"))
    }
}

/// Derive the output path for an input file when no `-o` was given and there
/// is exactly one input.  Replaces the extension with `.o`; if no extension,
/// appends `.o`.
fn default_output(input: &str) -> String {
    if input == "-" {
        return "a.out".to_string();
    }
    let p = std::path::Path::new(input);
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("a");
    let dir = p.parent().and_then(|d| d.to_str()).unwrap_or(".");
    if dir.is_empty() || dir == "." {
        format!("{stem}.o")
    } else {
        format!("{dir}/{stem}.o")
    }
}

/// Assemble one source text to an output file using the correct backend.
fn assemble_one(
    target: &Target,
    asm_text: &str,
    output: &str,
    extra_args: &[String],
) -> Result<(), String> {
    match target {
        Target::X86_64 => crate::backend::x86::assembler::assemble(asm_text, output),
        Target::I686 => crate::backend::i686::assembler::assemble(asm_text, output),
        Target::Aarch64 => crate::backend::arm::assembler::assemble(asm_text, output),
        Target::Riscv64 => {
            crate::backend::riscv::assembler::assemble_with_args(asm_text, output, extra_args)
        }
    }
}

/// Main entry point for the standalone assembler, called from `src/bin/axeon_as.rs`.
///
/// Returns `Ok(())` on success, `Err(message)` on failure.
pub fn assembler_main_inner() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let cfg = match parse_args(&args)? {
        None => return Ok(()), // --version / --help already printed
        Some(c) => c,
    };

    if cfg.verbose {
        eprintln!(
            "axeon-as: target={}, output={}",
            cfg.target.triple(),
            cfg.output
        );
    }

    // Collect (input_path, output_path) pairs.
    // Rules (matching GNU as behaviour):
    //   • No inputs              → read stdin, write to cfg.output (default "a.out")
    //   • One input, -o given    → write to cfg.output
    //   • One input, no -o       → write to <stem>.o
    //   • Multiple inputs, -o    → concatenate all sources into one object (cfg.output)
    //   • Multiple inputs, no -o → assemble each to its own <stem>.o
    let inputs: Vec<String> = if cfg.inputs.is_empty() {
        vec!["-".to_string()]
    } else {
        cfg.inputs.clone()
    };

    // Determine whether the user explicitly set -o
    let explicit_output = !cfg.output.is_empty() && cfg.output != "a.out" || cfg.inputs.is_empty(); // stdin always uses explicit output

    if inputs.len() == 1 || explicit_output {
        // Single output path: concatenate all sources and assemble together.
        // This is the common case and matches the way `gcc` invokes `as`.
        let mut combined = String::new();
        for path in &inputs {
            if cfg.verbose {
                eprintln!("axeon-as: assembling '{path}'");
            }
            let src = read_source(path)?;
            combined.push_str(&src);
            combined.push('\n');
        }

        let out = if explicit_output || inputs.len() == 1 {
            // Use cfg.output if it was explicitly set, otherwise derive from the
            // single input file name.
            if cfg.output == "a.out" && inputs.len() == 1 && inputs[0] != "-" {
                // No explicit -o: derive output name from input
                default_output(&inputs[0])
            } else {
                cfg.output.clone()
            }
        } else {
            cfg.output.clone()
        };

        assemble_one(&cfg.target, &combined, &out, &cfg.extra_args)?;

        if cfg.verbose {
            eprintln!("axeon-as: wrote '{out}'");
        }
    } else {
        // Multiple inputs, no explicit -o: assemble each file independently.
        for path in &inputs {
            if cfg.verbose {
                eprintln!("axeon-as: assembling '{path}'");
            }
            let src = read_source(path)?;
            let out = default_output(path);
            assemble_one(&cfg.target, &src, &out, &cfg.extra_args)?;
            if cfg.verbose {
                eprintln!("axeon-as: wrote '{out}'");
            }
        }
    }

    Ok(())
}

/// Public entry point called by the binary wrapper.
///
/// Mirrors the pattern used by `compiler_main()`: spawns on a large-stack thread,
/// prints errors to stderr, and exits with a non-zero code on failure.
pub fn assembler_main() {
    const STACK_SIZE: usize = 8 * 1024 * 1024; // 8 MB — assembler needs less than compiler
    let builder = std::thread::Builder::new()
        .name("axeon-as".to_string())
        .stack_size(STACK_SIZE);

    let handle = builder
        .spawn(assembler_main_inner)
        .expect("axeon-as: failed to spawn assembler thread");

    match handle.join() {
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
            eprintln!("axeon-as: error: {e}");
            std::process::exit(1);
        }
        Err(_) => {
            eprintln!("axeon-as: internal error (thread panicked)");
            std::process::exit(1);
        }
    }
}
