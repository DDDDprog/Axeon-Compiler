//! Shared backend utilities for assembler, linker, and data emission.
//!
//! All four backends (x86-64, i686, AArch64, RISC-V 64) share identical logic for:
//! - Emitting assembly data directives (.data, .bss, .rodata, string literals, constants)
//!
//! This module extracts that shared logic, parameterized only by:
//! - The 64-bit data directive (`.quad` vs `.xword` vs `.dword`)
//! - Extra assembler/linker flags

use crate::backend::elf::{EM_386, EM_AARCH64, EM_RISCV, EM_X86_64};
use crate::common::types::IrType;
use crate::ir::reexports::{GlobalInit, IrConst, IrGlobal, IrModule};

pub struct LinkerConfig {
    /// Expected ELF e_machine value for this target (e.g., EM_X86_64=62, EM_RISCV=243).
    /// Used to validate input .o files before linking and produce clear error messages
    /// when stale/wrong-arch objects are accidentally passed to the linker.
    pub expected_elf_machine: u16,
    /// Human-readable architecture name for error messages (e.g., "RISC-V", "x86-64").
    pub arch_name: &'static str,
}

/// Map an ELF e_machine value to a human-readable architecture name.
fn elf_machine_name(em: u16) -> &'static str {
    match em {
        EM_386 => "i386",
        40 => "ARM",
        EM_X86_64 => "x86-64",
        EM_AARCH64 => "aarch64",
        EM_RISCV => "RISC-V",
        _ => "unknown",
    }
}

/// Validate that all .o files in a list match the expected ELF e_machine.
/// Returns Ok(()) if all files match or are not ELF objects (archives, shared libs, etc.).
/// Returns Err with a diagnostic listing the mismatched files.
fn validate_object_architectures(
    files: impl Iterator<Item: AsRef<str>>,
    expected_machine: u16,
    arch_name: &str,
) -> Result<(), String> {
    use std::io::Read;
    let mut mismatched = Vec::new();

    for path_ref in files {
        let path = path_ref.as_ref();
        // Only check .o files (not .a, .so, -l flags, -Wl, flags, etc.)
        if !path.ends_with(".o") {
            continue;
        }
        // Read the ELF header: first 20 bytes contain e_ident (16) + e_type (2) + e_machine (2)
        let mut buf = [0u8; 20];
        let Ok(mut f) = std::fs::File::open(path) else {
            continue;
        };
        let Ok(n) = f.read(&mut buf) else { continue };
        if n < 20 {
            continue;
        }
        // Verify ELF magic
        if &buf[0..4] != b"\x7fELF" {
            continue;
        }
        // e_machine is at offset 18, always 2 bytes.
        // Determine endianness from EI_DATA (byte 5): 1=LE, 2=BE
        let is_le = buf[5] == 1;
        let em = if is_le {
            u16::from_le_bytes([buf[18], buf[19]])
        } else {
            u16::from_be_bytes([buf[18], buf[19]])
        };
        if em != expected_machine {
            mismatched.push((path.to_string(), em));
        }
    }

    if mismatched.is_empty() {
        return Ok(());
    }

    let mut msg = format!(
        "Object file architecture mismatch: target is {} (ELF e_machine={}) but these files are for a different architecture:\n",
        arch_name, expected_machine
    );
    for (path, em) in &mismatched {
        msg.push_str(&format!(
            "  {} ({}; e_machine={})\n",
            path,
            elf_machine_name(*em),
            em
        ));
    }
    msg.push_str("Hint: these look like stale objects from a previous build. Try running 'make clean' before rebuilding.");
    Err(msg)
}

pub fn link_with_args(
    config: &LinkerConfig,
    object_files: &[&str],
    output_path: &str,
    user_args: &[String],
) -> Result<(), String> {
    // Validate that all input .o files match the target architecture.
    validate_object_architectures(
        object_files
            .iter()
            .copied()
            .chain(user_args.iter().map(|s| s.as_str())),
        config.expected_elf_machine,
        config.arch_name,
    )?;

    let is_shared = user_args.iter().any(|a| a == "-shared");
    let is_nostdlib = user_args.iter().any(|a| a == "-nostdlib");
    let is_relocatable = user_args.iter().any(|a| a == "-r");
    let is_static = user_args.iter().any(|a| a == "-static");

    if is_relocatable {
        return Err("Relocatable linking (-r) is not currently supported.".to_string());
    }

    // Look up the architecture config by ELF machine number
    let arch = match config.expected_elf_machine {
        EM_X86_64 => &DIRECT_LD_X86_64,
        EM_AARCH64 => &DIRECT_LD_AARCH64,
        EM_RISCV => &DIRECT_LD_RISCV64,
        EM_386 => &DIRECT_LD_I686,
        _ => {
            return Err(format!(
                "No built-in linker for ELF machine {} ({}).",
                config.expected_elf_machine, config.arch_name
            ));
        }
    };

    link_builtin_native(
        arch,
        object_files,
        output_path,
        user_args,
        is_nostdlib,
        is_static,
        is_shared,
    )
}

// DirectLdArchConfig captures differences between architectures for
// CRT/library discovery and built-in linker invocation.
#[allow(dead_code)]
struct DirectLdArchConfig {
    /// Human-readable architecture name for error messages (e.g., "x86-64", "RISC-V")
    arch_name: &'static str,
    /// ELF e_machine value (e.g., EM_X86_64=62, EM_RISCV=243).
    /// Used to dispatch to the correct backend linker.
    elf_machine: u16,
    /// ld emulation mode (e.g., "elf_x86_64", "elf64lriscv", "elf_i386", "aarch64linux")
    emulation: &'static str,
    /// Dynamic linker path (e.g., "/lib64/ld-linux-x86-64.so.2")
    dynamic_linker: &'static str,
    /// Candidate directories for system CRT objects (crt1.o)
    crt_dir_candidates: &'static [&'static str],
    /// Standard system library directories for -L paths
    system_lib_dirs: &'static [&'static str],
    /// Extra ld flags specific to this architecture (e.g., AArch64 erratum workarounds)
    extra_ld_flags: &'static [&'static str],
    /// Extra flags to skip when converting user args (e.g., "-m32" for i686)
    extra_skip_flags: &'static [&'static str],
    /// If true, crti.o and crtn.o are found in the runtime lib dir rather than the CRT dir.
    crti_from_runtime_dir: bool,
    /// Package hint for CRT not-found error messages
    crt_package_hint: &'static str,
    /// Package hint for runtime lib not-found error messages
    runtime_package_hint: &'static str,
}

const DIRECT_LD_X86_64: DirectLdArchConfig = DirectLdArchConfig {
    arch_name: "x86-64",
    elf_machine: EM_X86_64,
    emulation: "elf_x86_64",
    dynamic_linker: "/lib64/ld-linux-x86-64.so.2",
    crt_dir_candidates: &[
        "/usr/lib/x86_64-linux-gnu",
        "/usr/lib64",
        "/lib/x86_64-linux-gnu",
        "/lib64",
    ],
    system_lib_dirs: &[
        "/lib/x86_64-linux-gnu",
        "/lib/../lib",
        "/usr/lib/x86_64-linux-gnu",
        "/usr/lib/../lib",
    ],
    extra_ld_flags: &[],
    extra_skip_flags: &[],
    crti_from_runtime_dir: false,
    crt_package_hint: "Is the libc development package installed?",
    runtime_package_hint: "Is the compiler-runtime package installed?",
};

const DIRECT_LD_RISCV64: DirectLdArchConfig = DirectLdArchConfig {
    arch_name: "RISC-V 64",
    elf_machine: EM_RISCV,
    emulation: "elf64lriscv",
    dynamic_linker: "/lib/ld-linux-riscv64-lp64d.so.1",
    crt_dir_candidates: &[
        "/usr/riscv64-linux-gnu/lib",
        "/usr/lib/riscv64-linux-gnu",
        "/lib/riscv64-linux-gnu",
    ],
    system_lib_dirs: &["/lib/riscv64-linux-gnu", "/usr/lib/riscv64-linux-gnu"],
    extra_ld_flags: &[],
    extra_skip_flags: &[],
    crti_from_runtime_dir: true,
    crt_package_hint: "Is the riscv64-linux-gnu libc development package installed?",
    runtime_package_hint: "Is the riscv64-linux-gnu compiler-runtime installed?",
};

const DIRECT_LD_I686: DirectLdArchConfig = DirectLdArchConfig {
    arch_name: "i686",
    elf_machine: EM_386,
    emulation: "elf_i386",
    dynamic_linker: "/lib/ld-linux.so.2",
    crt_dir_candidates: &[
        "/usr/lib/i386-linux-gnu",
        "/usr/i686-linux-gnu/lib",
        "/usr/lib32",
        "/lib/i386-linux-gnu",
        "/lib32",
    ],
    system_lib_dirs: &[
        "/lib/i386-linux-gnu",
        "/lib/../lib",
        "/usr/lib/i386-linux-gnu",
        "/usr/lib/../lib",
        "/usr/i686-linux-gnu/lib",
    ],
    extra_ld_flags: &[],
    extra_skip_flags: &["-m32"],
    crti_from_runtime_dir: false,
    crt_package_hint: "Is the libc-dev-i386 package installed?",
    runtime_package_hint: "Is the compiler-runtime package installed?",
};

const DIRECT_LD_AARCH64: DirectLdArchConfig = DirectLdArchConfig {
    arch_name: "AArch64",
    elf_machine: EM_AARCH64,
    emulation: "aarch64linux",
    dynamic_linker: "/lib/ld-linux-aarch64.so.1",
    crt_dir_candidates: &[
        "/usr/aarch64-linux-gnu/lib",
        "/usr/lib/aarch64-linux-gnu",
        "/usr/lib64",
        "/lib/aarch64-linux-gnu",
        "/lib64",
    ],
    system_lib_dirs: &[
        "/lib/aarch64-linux-gnu",
        "/lib/../lib",
        "/usr/lib/aarch64-linux-gnu",
        "/usr/lib/../lib",
        "/usr/aarch64-linux-gnu/lib",
    ],
    extra_ld_flags: &["-EL", "-X", "--fix-cortex-a53-843419"],
    extra_skip_flags: &[],
    crti_from_runtime_dir: false,
    crt_package_hint: "Is the libc-dev-arm64 package installed?",
    runtime_package_hint: "Is the aarch64-linux-gnu compiler-runtime installed?",
};

/// Discover the compiler runtime library directory (containing crtbegin.o / libgcc.a).
/// Returns the path (e.g., "/usr/lib/axeon/lib/x86_64-linux-gnu").
fn find_runtime_lib_dir(arch: &DirectLdArchConfig) -> Option<String> {
    // 1. Check bundled lib (relative to executable)
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let arch_suffix = match arch.elf_machine {
                EM_X86_64 => "x86_64-linux-gnu",
                EM_386 => "i386-linux-gnu",
                EM_AARCH64 => "aarch64-linux-gnu",
                EM_RISCV => "riscv64-linux-gnu",
                _ => "",
            };
            if !arch_suffix.is_empty() {
                let bundled = exe_dir.join("lib").join(arch_suffix);
                if bundled.join("crtbegin.o").exists() {
                    return Some(bundled.to_string_lossy().to_string());
                }
                // Check project root style
                let bundled_project = std::path::Path::new("lib").join(arch_suffix);
                if bundled_project.join("crtbegin.o").exists() {
                    return Some(bundled_project.to_string_lossy().to_string());
                }
            }
        }
    }

    // 2. We no longer probe system GCC directories.
    // Axeon is independent.
    None
}

/// Discover the system CRT directory containing crt1.o.
/// Returns the path (e.g., "/usr/lib/x86_64-linux-gnu").
fn find_crt_dir(arch: &DirectLdArchConfig) -> Option<String> {
    // 1. Check bundled CRT dir (relative to executable)
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let arch_suffix = match arch.elf_machine {
                EM_X86_64 => "x86_64-linux-gnu",
                EM_386 => "i386-linux-gnu",
                EM_AARCH64 => "aarch64-linux-gnu",
                EM_RISCV => "riscv64-linux-gnu",
                _ => "",
            };
            if !arch_suffix.is_empty() {
                let bundled = exe_dir.join("lib").join(arch_suffix);
                if bundled.join("crt1.o").exists() {
                    return Some(bundled.to_string_lossy().to_string());
                }
                // Check project root style
                let bundled_project = std::path::Path::new("lib").join(arch_suffix);
                if bundled_project.join("crt1.o").exists() {
                    return Some(bundled_project.to_string_lossy().to_string());
                }
            }
        }
    }

    // 2. Fall back to system candidates
    for dir in arch.crt_dir_candidates {
        let crt1 = format!("{}/crt1.o", dir);
        if std::path::Path::new(&crt1).exists() {
            return Some(dir.to_string());
        }
    }
    None
}

/// Resolve CRT objects and library paths for a built-in linker using DirectLdArchConfig.
///
/// This shared helper is used by all four built-in linker wrappers
/// (x86-64, i686, AArch64, RISC-V) to avoid duplicating CRT/library
/// discovery logic. Returns:
/// - `crt_before`: CRT objects to link before user objects
/// - `crt_after`: CRT objects to link after user objects
/// - `lib_paths`: Combined library search paths (user -L first, then system paths)
/// - `needed_libs`: Default libraries to link
///
/// ## GCC-free linking strategy
///
/// musl libc does **not** use `crtbegin.o` / `crtend.o` (those are GCC
/// compiler-support objects). When musl's CRT directory is found we use it
/// directly and skip the GCC lib dir entirely, resulting in zero GCC
/// dependency at link time.
///
/// Discovery order:
///   1. musl CRT dirs   — no crtbegin/crtend, no libgcc needed
///   2. glibc CRT dir   — needs crtbegin/crtend from GCC lib dir
///
/// The musl CRT search covers the common Debian/Ubuntu/Fedora/Alpine
/// install paths for `musl-dev` / `musl-libc`.
struct BuiltinLinkSetup {
    crt_before: Vec<String>,
    crt_after: Vec<String>,
    lib_paths: Vec<String>,
    needed_libs: Vec<String>,
    dynamic_linker_override: Option<String>,
}

/// Known musl CRT directories, ordered by preference.
/// These directories contain `crt1.o`, `crti.o`, `crtn.o` **without**
/// needing `crtbegin.o` / `crtend.o` from GCC.
const MUSL_CRT_DIRS_X86_64: &[&str] = &[
    "/usr/lib/x86_64-linux-musl", // Debian/Ubuntu musl-dev
    "/usr/lib/musl/lib",          // Alpine / some Fedora layouts
    "/usr/musl/lib",              // musl-cross-make installs
    "/lib/x86_64-linux-musl",
];

const MUSL_CRT_DIRS_I686: &[&str] = &[
    "/usr/lib/i686-linux-musl",
    "/usr/lib/musl/lib",
    "/usr/musl/lib",
    "/lib/i686-linux-musl",
];

const MUSL_CRT_DIRS_AARCH64: &[&str] = &[
    "/usr/lib/aarch64-linux-musl",
    "/usr/aarch64-linux-musl/lib",
    "/usr/lib/musl/lib",
];

const MUSL_CRT_DIRS_RISCV64: &[&str] = &[
    "/usr/lib/riscv64-linux-musl",
    "/usr/riscv64-linux-musl/lib",
    "/usr/lib/musl/lib",
];

/// Return the musl CRT directory for `elf_machine`, or `None` if musl is not
/// installed. The returned directory is guaranteed to contain `crt1.o`.
fn find_musl_crt_dir(elf_machine: u16) -> Option<String> {
    let candidates: &[&str] = match elf_machine {
        EM_X86_64 => MUSL_CRT_DIRS_X86_64,
        EM_386 => MUSL_CRT_DIRS_I686,
        EM_AARCH64 => MUSL_CRT_DIRS_AARCH64,
        EM_RISCV => MUSL_CRT_DIRS_RISCV64,
        _ => return None,
    };
    for dir in candidates {
        if std::path::Path::new(&format!("{}/crt1.o", dir)).exists() {
            return Some(dir.to_string());
        }
    }
    None
}

/// Return the musl dynamic-linker path for `elf_machine`, or `None`.
fn find_musl_dynamic_linker(elf_machine: u16) -> Option<&'static str> {
    // Standard install locations for the musl dynamic linker on Linux.
    let candidates: &[&str] = match elf_machine {
        EM_X86_64 => &["/usr/lib/ld-musl-x86_64.so.1", "/lib/ld-musl-x86_64.so.1"],
        EM_386 => &["/usr/lib/ld-musl-i386.so.1", "/lib/ld-musl-i386.so.1"],
        EM_AARCH64 => &["/usr/lib/ld-musl-aarch64.so.1", "/lib/ld-musl-aarch64.so.1"],
        EM_RISCV => &["/usr/lib/ld-musl-riscv64.so.1", "/lib/ld-musl-riscv64.so.1"],
        _ => return None,
    };
    for path in candidates {
        if std::path::Path::new(path).exists() {
            return Some(path);
        }
    }
    None
}

fn resolve_builtin_link_setup(
    arch: &DirectLdArchConfig,
    user_args: &[String],
    is_nostdlib: bool,
    is_static: bool,
) -> BuiltinLinkSetup {
    // ------------------------------------------------------------------
    // Detect runtime/CRT directories.
    // ------------------------------------------------------------------
    let (runtime_lib_dir, system_crt_dir) = (find_runtime_lib_dir(arch), find_crt_dir(arch));

    let musl_crt_dir = if runtime_lib_dir.is_none() || system_crt_dir.is_none() {
        find_musl_crt_dir(arch.elf_machine)
    } else {
        None
    };

    // Effective CRT directory: musl takes priority, then system glibc.
    let crt_dir: Option<String> = musl_crt_dir.clone().or_else(|| system_crt_dir.clone());

    // ------------------------------------------------------------------
    // System library search paths
    // ------------------------------------------------------------------
    let mut system_lib_paths: Vec<String> = Vec::new();

    // musl lib dir first (when musl is available)
    if let Some(ref mdir) = musl_crt_dir {
        system_lib_paths.push(mdir.clone());
    }
    // Runtime lib dir (only when musl is absent)
    if let Some(ref runtime) = runtime_lib_dir {
        system_lib_paths.push(runtime.clone());
    }
    // System CRT dir (only when musl is absent)
    if let Some(ref crt) = system_crt_dir {
        if musl_crt_dir.is_none() {
            system_lib_paths.push(crt.clone());
        }
    }
    // Architecture system lib dirs
    for dir in arch.system_lib_dirs {
        if std::path::Path::new(dir).exists() {
            system_lib_paths.push(dir.to_string());
        }
    }

    // Bundled libraries (relative to executable)
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let arch_suffix = match arch.elf_machine {
                EM_X86_64 => "x86_64-linux-gnu",
                EM_386 => "i386-linux-gnu",
                EM_AARCH64 => "aarch64-linux-gnu",
                EM_RISCV => "riscv64-linux-gnu",
                _ => "",
            };
            if !arch_suffix.is_empty() {
                let bundled_lib = exe_dir.join("lib").join(arch_suffix);
                if bundled_lib.exists() {
                    system_lib_paths.push(bundled_lib.to_string_lossy().to_string());
                }
                // Also check project root style
                let bundled_lib_project = std::path::Path::new("lib").join(arch_suffix);
                if bundled_lib_project.exists()
                    && !system_lib_paths
                        .contains(&bundled_lib_project.to_string_lossy().to_string())
                {
                    system_lib_paths.push(bundled_lib_project.to_string_lossy().to_string());
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // User-provided -L paths
    // ------------------------------------------------------------------
    let mut user_lib_paths: Vec<String> = Vec::new();
    let mut i = 0;
    while i < user_args.len() {
        let arg = &user_args[i];
        if let Some(path) = arg.strip_prefix("-L") {
            if path.is_empty() {
                if i + 1 < user_args.len() {
                    i += 1;
                    user_lib_paths.push(user_args[i].clone());
                }
            } else {
                user_lib_paths.push(path.to_string());
            }
        } else if let Some(wl_arg) = arg.strip_prefix("-Wl,") {
            for part in wl_arg.split(',') {
                if let Some(lpath) = part.strip_prefix("-L") {
                    user_lib_paths.push(lpath.to_string());
                }
            }
        }
        i += 1;
    }

    // ------------------------------------------------------------------
    // CRT objects
    // ------------------------------------------------------------------
    let mut crt_before: Vec<String> = Vec::new();
    let mut crt_after: Vec<String> = Vec::new();

    if !is_nostdlib {
        if let Some(ref crt) = crt_dir {
            // crt1.o — musl uses Scrt1.o for PIE, crt1.o for non-PIE
            // (we always use non-PIE / -no-pie style, so crt1.o is correct)
            let crt1 = format!("{}/crt1.o", crt);
            if std::path::Path::new(&crt1).exists() {
                crt_before.push(crt1);
            }
            // crti.o
            let crti = format!("{}/crti.o", crt);
            if std::path::Path::new(&crti).exists() {
                crt_before.push(crti);
            }
        }

        if musl_crt_dir.is_some() {
            // musl: NO crtbegin / crtend — musl handles its own startup fully.
            // crtn.o comes from the same musl dir.
            if let Some(ref crt) = crt_dir {
                let crtn = format!("{}/crtn.o", crt);
                if std::path::Path::new(&crtn).exists() {
                    crt_after.push(crtn);
                }
            }
        } else {
            // System path: needs runtime objects (crtbegin/crtend).
            if arch.crti_from_runtime_dir {
                // For some architectures, crti/crtn come from the runtime dir.
                if let Some(ref runtime) = runtime_lib_dir {
                    let crti = format!("{}/crti.o", runtime);
                    if std::path::Path::new(&crti).exists() {
                        // Insert before crt1.o position
                    }
                    // crtbegin
                    if is_static {
                        let crtbegin_t = format!("{}/crtbeginT.o", runtime);
                        if std::path::Path::new(&crtbegin_t).exists() {
                            crt_before.push(crtbegin_t);
                        } else {
                            crt_before.push(format!("{}/crtbegin.o", runtime));
                        }
                    } else {
                        crt_before.push(format!("{}/crtbegin.o", runtime));
                    }
                    crt_after.push(format!("{}/crtend.o", runtime));
                    crt_after.push(format!("{}/crtn.o", runtime));
                }
            } else {
                // Standard system: crti/crtn from system dir, crtbegin/crtend from runtime dir.
                if let Some(ref runtime) = runtime_lib_dir {
                    if is_static {
                        let crtbegin_t = format!("{}/crtbeginT.o", runtime);
                        if std::path::Path::new(&crtbegin_t).exists() {
                            crt_before.push(crtbegin_t);
                        } else {
                            crt_before.push(format!("{}/crtbegin.o", runtime));
                        }
                    } else {
                        crt_before.push(format!("{}/crtbegin.o", runtime));
                    }
                    crt_after.push(format!("{}/crtend.o", runtime));
                }
                if let Some(ref system) = system_crt_dir {
                    crt_after.push(format!("{}/crtn.o", system));
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // Default libraries
    // ------------------------------------------------------------------
    let needed_libs: Vec<String> = if !is_nostdlib {
        if musl_crt_dir.is_some() {
            vec!["c".to_string()]
        } else {
            // "gcc" is still the name of the runtime library file on most systems,
            // but we treat it as an anonymous compiler runtime.
            vec!["gcc".to_string(), "c".to_string(), "m".to_string()]
        }
    } else {
        vec![]
    };

    // ------------------------------------------------------------------
    // Combined lib paths: user first, then system
    // ------------------------------------------------------------------
    let mut lib_paths: Vec<String> = user_lib_paths;
    lib_paths.extend(system_lib_paths);

    let dynamic_linker_override = if musl_crt_dir.is_some() {
        find_musl_dynamic_linker(arch.elf_machine).map(|s| s.to_string())
    } else {
        None
    };

    BuiltinLinkSetup {
        crt_before,
        crt_after,
        lib_paths,
        needed_libs,
        dynamic_linker_override,
    }
}

/// Add architecture-specific extra runtime libraries.
fn add_arch_extra_libs(setup: &BuiltinLinkSetup, elf_machine: u16, is_static: bool) -> Vec<String> {
    let mut libs = setup.needed_libs.clone();
    if elf_machine == EM_X86_64 {
        return libs;
    }
    if let Some(pos) = libs.iter().position(|l| l == "gcc") {
        let extra = if elf_machine == EM_386 || is_static {
            "gcc_eh"
        } else {
            "gcc_s"
        };
        libs.insert(pos + 1, extra.to_string());
    }
    libs
}

/// Convert a `BuiltinLinkSetup` into borrowed slices for passing to backend linkers.
///
/// Avoids repeating the same 4-line `.iter().map(|s| s.as_str()).collect()` pattern.
struct LinkSetupRefs<'a> {
    lib_paths: Vec<&'a str>,
    needed_libs: Vec<&'a str>,
    crt_before: Vec<&'a str>,
    crt_after: Vec<&'a str>,
}

impl BuiltinLinkSetup {
    fn as_refs(&self) -> LinkSetupRefs<'_> {
        LinkSetupRefs {
            lib_paths: self.lib_paths.iter().map(|s| s.as_str()).collect(),
            needed_libs: self.needed_libs.iter().map(|s| s.as_str()).collect(),
            crt_before: self.crt_before.iter().map(|s| s.as_str()).collect(),
            crt_after: self.crt_after.iter().map(|s| s.as_str()).collect(),
        }
    }
}

/// Link using the built-in native ELF linker for any supported architecture.
///
/// This is the fully native path: no external ld binary is needed. The linker
/// reads ELF .o files and .a archives, resolves symbols against system shared
/// libraries (libc.so.6), handles relocations, and produces a dynamically-linked
/// ELF executable. Dispatches to the correct per-architecture backend based on
/// the `arch.elf_machine` value.
///
/// For shared library output (-shared), delegates to the per-arch `link_shared`
/// entry point with library paths only (no CRT objects).
fn link_builtin_native(
    arch: &DirectLdArchConfig,
    object_files: &[&str],
    output_path: &str,
    user_args: &[String],
    is_nostdlib: bool,
    is_static: bool,
    is_shared: bool,
) -> Result<(), String> {
    use crate::backend::{arm, i686, riscv, x86};

    if is_shared {
        // Shared libraries: no CRT objects, lib paths only
        let setup = resolve_builtin_link_setup(arch, user_args, true, false);
        let refs = setup.as_refs();
        return match arch.elf_machine {
            EM_X86_64 => {
                // x86-64 shared linker also takes implicit libs (gcc for runtime helpers)
                let implicit_libs: Vec<&str> = if is_nostdlib { vec![] } else { vec!["gcc"] };
                x86::linker::link_shared(
                    object_files,
                    output_path,
                    user_args,
                    &refs.lib_paths,
                    &implicit_libs,
                    setup.dynamic_linker_override.as_deref(),
                )
            }
            EM_AARCH64 => arm::linker::link_shared(
                object_files,
                output_path,
                user_args,
                &refs.lib_paths,
                setup.dynamic_linker_override.as_deref(),
            ),
            EM_RISCV => riscv::linker::link_shared(
                object_files,
                output_path,
                user_args,
                &refs.lib_paths,
                setup.dynamic_linker_override.as_deref(),
            ),
            EM_386 => i686::linker::link_shared(
                object_files,
                output_path,
                user_args,
                &refs.lib_paths,
                setup.dynamic_linker_override.as_deref(),
            ),
            _ => Err(format!(
                "No shared library linker for {} (elf_machine={})",
                arch.arch_name, arch.elf_machine
            )),
        };
    }

    let mut setup = resolve_builtin_link_setup(arch, user_args, is_nostdlib, is_static);
    add_arch_extra_libs(&mut setup, arch.elf_machine, is_static);
    let refs = setup.as_refs();

    let dynlinker: &str = setup
        .dynamic_linker_override
        .as_deref()
        .unwrap_or(arch.dynamic_linker);

    match arch.elf_machine {
        EM_X86_64 => x86::linker::link_builtin(
            object_files,
            output_path,
            user_args,
            &refs.lib_paths,
            &refs.needed_libs,
            &refs.crt_before,
            &refs.crt_after,
            dynlinker,
        ),
        EM_386 => i686::linker::link_builtin(
            object_files,
            output_path,
            user_args,
            &refs.lib_paths,
            &refs.needed_libs,
            &refs.crt_before,
            &refs.crt_after,
            dynlinker,
        ),
        EM_AARCH64 => arm::linker::link_builtin(
            object_files,
            output_path,
            user_args,
            &refs.lib_paths,
            &refs.needed_libs,
            &refs.crt_before,
            &refs.crt_after,
            dynlinker,
        ),
        EM_RISCV => riscv::linker::link_builtin(
            object_files,
            output_path,
            user_args,
            &refs.lib_paths,
            &refs.needed_libs,
            &refs.crt_before,
            &refs.crt_after,
            dynlinker,
        ),
        _ => Err(format!(
            "No built-in linker for {} (elf_machine={})",
            arch.arch_name, arch.elf_machine
        )),
    }
}

/// Assembly output buffer with helpers for emitting text.
///
/// Besides the generic `emit` and `emit_fmt` methods, this provides specialized
/// fast-path emitters for common patterns that avoid `core::fmt` overhead.
/// The fast integer writer (`write_i64`) uses direct digit extraction instead
/// of going through `Display`/`write_fmt` machinery.
pub struct AsmOutput {
    pub buf: String,
}

/// Write an i64 directly into a String buffer using manual digit extraction.
/// This is ~3-4x faster than `write!(buf, "{}", val)` for the common case
/// because it avoids the `core::fmt` vtable dispatch and `pad_integral` overhead.
#[inline]
fn write_i64_fast(buf: &mut String, val: i64) {
    if val == 0 {
        buf.push('0');
        return;
    }
    let mut tmp = [0u8; 20]; // i64 max is 19 digits + sign
    let negative = val < 0;
    // Work with absolute value using wrapping to handle i64::MIN correctly
    let mut v = if negative {
        (val as u64).wrapping_neg()
    } else {
        val as u64
    };
    let mut pos = 20;
    while v > 0 {
        pos -= 1;
        tmp[pos] = b'0' + (v % 10) as u8;
        v /= 10;
    }
    if negative {
        pos -= 1;
        tmp[pos] = b'-';
    }
    // All bytes are ASCII digits and optionally '-', which is always valid UTF-8.
    let s = std::str::from_utf8(&tmp[pos..20]).expect("integer formatting produced non-UTF8");
    buf.push_str(s);
}

/// Write a u64 directly into a String buffer.
#[inline]
fn write_u64_fast(buf: &mut String, val: u64) {
    if val == 0 {
        buf.push('0');
        return;
    }
    let mut tmp = [0u8; 20]; // u64 max is 20 digits
    let mut v = val;
    let mut pos = 20;
    while v > 0 {
        pos -= 1;
        tmp[pos] = b'0' + (v % 10) as u8;
        v /= 10;
    }
    let s = std::str::from_utf8(&tmp[pos..20]).expect("integer formatting produced non-UTF8");
    buf.push_str(s);
}

impl AsmOutput {
    pub fn new() -> Self {
        // Pre-allocate 256KB to avoid repeated reallocations during codegen.
        Self {
            buf: String::with_capacity(256 * 1024),
        }
    }

    /// Emit a line of assembly.
    #[inline]
    pub fn emit(&mut self, s: &str) {
        self.buf.push_str(s);
        self.buf.push('\n');
    }

    /// Emit formatted assembly directly into the buffer (no temporary String).
    #[inline]
    pub fn emit_fmt(&mut self, args: std::fmt::Arguments<'_>) {
        std::fmt::Write::write_fmt(&mut self.buf, args).unwrap();
        self.buf.push('\n');
    }

    // ── Fast-path emitters ──────────────────────────────────────────────
    //
    // These avoid the overhead of `format_args!` + `core::fmt::write` for
    // the most common codegen patterns. Each one directly pushes bytes into
    // the buffer using `push_str` and our fast integer writer.

    /// Emit: `    {mnemonic} ${imm}, %{reg}`
    /// Used for movq/movl/movabsq with immediate to register.
    #[inline]
    pub fn emit_instr_imm_reg(&mut self, mnemonic: &str, imm: i64, reg: &str) {
        self.buf.push_str(mnemonic);
        self.buf.push_str(" $");
        write_i64_fast(&mut self.buf, imm);
        self.buf.push_str(", %");
        self.buf.push_str(reg);
        self.buf.push('\n');
    }

    /// Emit: `    {mnemonic} %{src}, %{dst}`
    /// Used for movq/movl/xorq register-to-register.
    #[inline]
    pub fn emit_instr_reg_reg(&mut self, mnemonic: &str, src: &str, dst: &str) {
        self.buf.push_str(mnemonic);
        self.buf.push_str(" %");
        self.buf.push_str(src);
        self.buf.push_str(", %");
        self.buf.push_str(dst);
        self.buf.push('\n');
    }

    /// Emit: `    {mnemonic} {offset}(%rbp), %{reg}`
    /// Used for loads from stack slots.
    #[inline]
    pub fn emit_instr_rbp_reg(&mut self, mnemonic: &str, offset: i64, reg: &str) {
        self.buf.push_str(mnemonic);
        self.buf.push(' ');
        write_i64_fast(&mut self.buf, offset);
        self.buf.push_str("(%rbp), %");
        self.buf.push_str(reg);
        self.buf.push('\n');
    }

    /// Emit: `    {mnemonic} %{reg}, {offset}(%rbp)`
    /// Used for stores to stack slots.
    #[inline]
    pub fn emit_instr_reg_rbp(&mut self, mnemonic: &str, reg: &str, offset: i64) {
        self.buf.push_str(mnemonic);
        self.buf.push_str(" %");
        self.buf.push_str(reg);
        self.buf.push_str(", ");
        write_i64_fast(&mut self.buf, offset);
        self.buf.push_str("(%rbp)");
        self.buf.push('\n');
    }

    /// Emit a block label line: `.LBB{id}:`
    #[inline]
    pub fn emit_block_label(&mut self, block_id: u32) {
        self.buf.push_str(".LBB");
        write_u64_fast(&mut self.buf, block_id as u64);
        self.buf.push(':');
        self.buf.push('\n');
    }

    /// Emit: `    jmp .LBB{block_id}`
    #[inline]
    pub fn emit_jmp_block(&mut self, block_id: u32) {
        self.buf.push_str("    jmp .LBB");
        write_u64_fast(&mut self.buf, block_id as u64);
        self.buf.push('\n');
    }

    /// Emit: `    {jcc} .LBB{block_id}` (conditional jump to block label)
    #[inline]
    pub fn emit_jcc_block(&mut self, jcc: &str, block_id: u32) {
        self.buf.push_str(jcc);
        self.buf.push_str(" .LBB");
        write_u64_fast(&mut self.buf, block_id as u64);
        self.buf.push('\n');
    }

    /// Emit: `    {mnemonic} {reg}`  (single-register instruction like push/pop)
    #[inline]
    pub fn emit_instr_reg(&mut self, mnemonic: &str, reg: &str) {
        self.buf.push_str(mnemonic);
        self.buf.push_str(" %");
        self.buf.push_str(reg);
        self.buf.push('\n');
    }

    /// Emit: `    {mnemonic} ${imm}`  (single-immediate instruction like push)
    #[inline]
    pub fn emit_instr_imm(&mut self, mnemonic: &str, imm: i64) {
        self.buf.push_str(mnemonic);
        self.buf.push_str(" $");
        write_i64_fast(&mut self.buf, imm);
        self.buf.push('\n');
    }

    /// Write an i64 into the buffer without newline. Useful for building
    /// custom format patterns that include integers.
    #[inline]
    pub fn write_i64(&mut self, val: i64) {
        write_i64_fast(&mut self.buf, val);
    }

    /// Write a u64 into the buffer without newline.
    #[inline]
    pub fn write_u64(&mut self, val: u64) {
        write_u64_fast(&mut self.buf, val);
    }

    /// Emit: `    {mnemonic} {offset}(%rbp)` (single rbp-offset operand, e.g. fldt/fstpt)
    #[inline]
    pub fn emit_instr_rbp(&mut self, mnemonic: &str, offset: i64) {
        self.buf.push_str(mnemonic);
        self.buf.push(' ');
        write_i64_fast(&mut self.buf, offset);
        self.buf.push_str("(%rbp)");
        self.buf.push('\n');
    }

    /// Emit a named label definition: `{label}:`
    #[inline]
    pub fn emit_named_label(&mut self, label: &str) {
        self.buf.push_str(label);
        self.buf.push(':');
        self.buf.push('\n');
    }

    /// Emit: `    jmp {label}` (jump to named label)
    #[inline]
    pub fn emit_jmp_label(&mut self, label: &str) {
        self.buf.push_str("    jmp ");
        self.buf.push_str(label);
        self.buf.push('\n');
    }

    /// Emit: `    {jcc} {label}` (conditional jump to named label)
    #[inline]
    pub fn emit_jcc_label(&mut self, jcc: &str, label: &str) {
        self.buf.push_str(jcc);
        self.buf.push(' ');
        self.buf.push_str(label);
        self.buf.push('\n');
    }

    /// Emit: `    call {target}` (direct call to named function/label)
    #[inline]
    pub fn emit_call(&mut self, target: &str) {
        self.buf.push_str("    call ");
        self.buf.push_str(target);
        self.buf.push('\n');
    }

    /// Emit: `    {mnemonic} {offset}(%{base}), %{reg}` (memory to register with arbitrary base)
    #[inline]
    pub fn emit_instr_mem_reg(&mut self, mnemonic: &str, offset: i64, base: &str, reg: &str) {
        self.buf.push_str(mnemonic);
        self.buf.push(' ');
        if offset != 0 {
            write_i64_fast(&mut self.buf, offset);
        }
        self.buf.push_str("(%");
        self.buf.push_str(base);
        self.buf.push_str("), %");
        self.buf.push_str(reg);
        self.buf.push('\n');
    }

    /// Emit: `    {mnemonic} %{reg}, {offset}(%{base})` (register to memory with arbitrary base)
    #[inline]
    pub fn emit_instr_reg_mem(&mut self, mnemonic: &str, reg: &str, offset: i64, base: &str) {
        self.buf.push_str(mnemonic);
        self.buf.push_str(" %");
        self.buf.push_str(reg);
        self.buf.push_str(", ");
        if offset != 0 {
            write_i64_fast(&mut self.buf, offset);
        }
        self.buf.push_str("(%");
        self.buf.push_str(base);
        self.buf.push(')');
        self.buf.push('\n');
    }

    /// Emit: `    {mnemonic} ${imm}, {offset}(%{base})` (immediate to memory with arbitrary base)
    #[inline]
    pub fn emit_instr_imm_mem(&mut self, mnemonic: &str, imm: i64, offset: i64, base: &str) {
        self.buf.push_str(mnemonic);
        self.buf.push_str(" $");
        write_i64_fast(&mut self.buf, imm);
        self.buf.push_str(", ");
        if offset != 0 {
            write_i64_fast(&mut self.buf, offset);
        }
        self.buf.push_str("(%");
        self.buf.push_str(base);
        self.buf.push(')');
        self.buf.push('\n');
    }

    /// Emit: `    {mnemonic} {symbol}(%{base}), %{reg}` (symbol-relative addressing)
    /// Used for RIP-relative loads like `leaq table_label(%rip), %rcx`.
    #[inline]
    pub fn emit_instr_sym_base_reg(&mut self, mnemonic: &str, symbol: &str, base: &str, reg: &str) {
        self.buf.push_str(mnemonic);
        self.buf.push(' ');
        self.buf.push_str(symbol);
        self.buf.push_str("(%");
        self.buf.push_str(base);
        self.buf.push_str("), %");
        self.buf.push_str(reg);
        self.buf.push('\n');
    }

    /// Emit: `    {mnemonic} ${symbol}, %{reg}` (symbol as immediate)
    /// Used for absolute symbol addressing like `movq $name, %rax`.
    #[inline]
    pub fn emit_instr_sym_imm_reg(&mut self, mnemonic: &str, symbol: &str, reg: &str) {
        self.buf.push_str(mnemonic);
        self.buf.push_str(" $");
        self.buf.push_str(symbol);
        self.buf.push_str(", %");
        self.buf.push_str(reg);
        self.buf.push('\n');
    }

    /// Push a string slice without newline.
    #[inline]
    pub fn write_str(&mut self, s: &str) {
        self.buf.push_str(s);
    }

    /// Push a newline to end the current line.
    #[inline]
    pub fn newline(&mut self) {
        self.buf.push('\n');
    }
}

/// Emit formatted assembly directly into the output buffer, avoiding temporary
/// String allocations from `format!()`. Usage: `emit!(state, "    mov {}, {}", src, dst)`
#[macro_export]
macro_rules! emit {
    ($state:expr, $($arg:tt)*) => {
        $state.out.emit_fmt(format_args!($($arg)*))
    };
}

/// The only arch-specific difference in data emission: the name of the 64-bit pointer directive.
/// x86 uses `.quad`, AArch64 uses `.xword`, RISC-V uses `.dword`.
#[derive(Clone, Copy)]
pub enum PtrDirective {
    Quad,  // x86-64
    Long,  // i686 (32-bit)
    Xword, // AArch64
    Dword, // RISC-V 64
}

impl PtrDirective {
    pub fn as_str(self) -> &'static str {
        match self {
            PtrDirective::Quad => ".quad",
            PtrDirective::Long => ".long",
            PtrDirective::Xword => ".xword",
            PtrDirective::Dword => ".dword",
        }
    }

    /// Returns true if this is an x86 target directive (x86-64 or i686).
    /// Used to select x87 80-bit extended precision format for long double constants.
    pub fn is_x86(self) -> bool {
        matches!(self, PtrDirective::Quad | PtrDirective::Long)
    }

    /// Returns true if this is a 32-bit pointer directive.
    pub fn is_32bit(self) -> bool {
        matches!(self, PtrDirective::Long)
    }

    /// Returns true if this is the RISC-V target directive.
    /// RISC-V stores full IEEE binary128 long doubles in memory (allocas and globals).
    pub fn is_riscv(self) -> bool {
        matches!(self, PtrDirective::Dword)
    }

    /// Returns true if this is the AArch64 target directive.
    /// AArch64 stores full IEEE binary128 long doubles in memory (allocas and globals).
    pub fn is_arm(self) -> bool {
        matches!(self, PtrDirective::Xword)
    }

    /// Convert a byte alignment value to the correct `.align` argument for this target.
    /// On x86-64, `.align N` means N bytes. On ARM and RISC-V, `.align N` means 2^N bytes,
    /// so we must emit log2(N) instead.
    pub fn align_arg(self, bytes: usize) -> usize {
        debug_assert!(
            bytes == 0 || bytes.is_power_of_two(),
            "alignment must be power of 2"
        );
        match self {
            PtrDirective::Quad | PtrDirective::Long => bytes,
            PtrDirective::Xword | PtrDirective::Dword => {
                if bytes <= 1 {
                    0
                } else {
                    bytes.trailing_zeros() as usize
                }
            }
        }
    }
}

/// Emit all data sections (rodata for string literals, .data and .bss for globals).
pub fn emit_data_sections(out: &mut AsmOutput, module: &IrModule, ptr_dir: PtrDirective) {
    // String literals in .rodata
    if !module.string_literals.is_empty()
        || !module.wide_string_literals.is_empty()
        || !module.char16_string_literals.is_empty()
    {
        out.emit(".section .rodata");
        for (label, value) in &module.string_literals {
            out.emit_fmt(format_args!("{}:", label));
            emit_string_bytes(out, value);
        }
        // Wide string literals (L"..."): each char is a 4-byte wchar_t value
        for (label, chars) in &module.wide_string_literals {
            out.emit_fmt(format_args!(".align {}", ptr_dir.align_arg(4)));
            out.emit_fmt(format_args!("{}:", label));
            for &ch in chars {
                out.emit_fmt(format_args!("  .long {}", ch));
            }
        }
        // char16_t string literals (u"..."): each char is a 2-byte char16_t value
        for (label, chars) in &module.char16_string_literals {
            out.emit_fmt(format_args!(".align {}", ptr_dir.align_arg(2)));
            out.emit_fmt(format_args!("{}:", label));
            for &ch in chars {
                out.emit_fmt(format_args!("  .short {}", ch));
            }
        }
        out.emit("");
    }

    // Global variables
    emit_globals(out, &module.globals, ptr_dir);
}

/// Compute effective alignment for a global, promoting to 16 when size >= 16.
/// This matches GCC/Clang behavior on x86-64 and aarch64, enabling aligned SSE/NEON access.
/// Globals placed in custom sections are excluded from promotion because they may
/// form contiguous arrays (e.g. the kernel's __param or .init.setup sections) where
/// the linker expects elements at their natural stride with no extra padding.
/// Additionally, when the user explicitly specified an alignment via __attribute__((aligned(N)))
/// or _Alignas, we respect their choice and don't auto-promote. GCC behaves the same way:
/// explicit aligned(8) on a 24-byte struct gives 8-byte alignment, not 16.
fn effective_align(g: &IrGlobal) -> usize {
    if g.section.is_some() || g.has_explicit_align {
        return g.align;
    }
    if g.size >= 16 && g.align < 16 {
        16
    } else {
        g.align
    }
}

/// Emit a zero-initialized global variable (used in .bss, .tbss, and custom section zero-init).
fn emit_zero_global(out: &mut AsmOutput, g: &IrGlobal, obj_type: &str, ptr_dir: PtrDirective) {
    emit_symbol_directives(out, g);
    out.emit_fmt(format_args!(
        ".align {}",
        ptr_dir.align_arg(effective_align(g))
    ));
    out.emit_fmt(format_args!(".type {}, {}", g.name, obj_type));
    out.emit_fmt(format_args!(".size {}, {}", g.name, g.size));
    out.emit_fmt(format_args!("{}:", g.name));
    out.emit_fmt(format_args!("    .zero {}", g.size));
}

/// Target section classification for a global variable.
///
/// Each global is classified exactly once into one of these categories,
/// which determines which assembly section it belongs to.
#[derive(PartialEq, Eq)]
enum GlobalSection {
    /// Extern (undefined) symbol -- only needs visibility directive, no storage.
    Extern,
    /// Has `__attribute__((section(...)))` -- emitted in its custom section.
    Custom,
    /// Const-qualified, non-TLS, initialized, non-zero-size -> `.rodata`.
    Rodata,
    /// Thread-local, initialized, non-zero-size -> `.tdata`.
    Tdata,
    /// Non-const, non-TLS, initialized, non-zero-size -> `.data`.
    Data,
    /// Zero-initialized, `is_common` flag set -> `.comm` directive.
    Common,
    /// Thread-local, zero-initialized (or zero-size) -> `.tbss`.
    Tbss,
    /// Non-TLS, zero-initialized (or zero-size with init) -> `.bss`.
    Bss,
}

/// Classify a global variable into the section it should be emitted to.
///
/// The classification priority matches GCC behavior:
/// 1. Extern symbols get no storage (just visibility directives).
/// 2. Custom section overrides all other placement.
/// 3. TLS globals go to .tdata (initialized) or .tbss (zero-init).
/// 4. Const globals go to .rodata.
/// 5. Non-zero initialized non-const globals go to .data.
/// 6. Zero-initialized common globals go to .comm.
/// 7. Zero-initialized non-common globals go to .bss.
fn classify_global(g: &IrGlobal) -> GlobalSection {
    if g.is_extern {
        return GlobalSection::Extern;
    }
    if g.section.is_some() {
        return GlobalSection::Custom;
    }
    let is_zero = matches!(g.init, GlobalInit::Zero);
    let has_nonzero_init = !is_zero && g.size > 0;
    if g.is_thread_local {
        return if has_nonzero_init {
            GlobalSection::Tdata
        } else {
            GlobalSection::Tbss
        };
    }
    if has_nonzero_init {
        return if g.is_const {
            GlobalSection::Rodata
        } else {
            GlobalSection::Data
        };
    }
    // Zero-initialized (or zero-size with init)
    if g.is_common && is_zero {
        return GlobalSection::Common;
    }
    GlobalSection::Bss
}

/// Emit global variable definitions, grouped by target section.
///
/// Classifies each global once via `classify_global`, then emits all globals
/// for each section in a fixed order: extern visibility, custom sections,
/// .rodata, .tdata, .data, .comm, .tbss, .bss.
fn emit_globals(out: &mut AsmOutput, globals: &[IrGlobal], ptr_dir: PtrDirective) {
    // Phase 1: classify every global into its target section.
    let classified: Vec<GlobalSection> = globals.iter().map(classify_global).collect();

    // Phase 2: emit each section group in order.

    // Extern visibility directives (needed for PIC code so the assembler/linker knows
    // these symbols are resolved within the link unit).
    for (g, sect) in globals.iter().zip(&classified) {
        if matches!(sect, GlobalSection::Extern) {
            emit_visibility_directive(out, &g.name, &g.visibility);
            // For extern TLS variables, emit .type @tls_object so the assembler
            // creates a TLS-typed undefined symbol. Without this, the linker
            // reports "TLS definition mismatches non-TLS reference" when the
            // defining TU has the symbol in .tdata but this TU's reference
            // lacks TLS type information (defaults to STT_NOTYPE).
            if g.is_thread_local {
                out.emit_fmt(format_args!(".type {}, @tls_object", g.name));
            }
        }
    }

    // Custom section globals: each gets its own .section directive since they
    // may target different sections.
    for (g, sect) in globals.iter().zip(&classified) {
        if !matches!(sect, GlobalSection::Custom) {
            continue;
        }
        let section_name = g.section.as_ref().expect("custom section must have a name");
        // Use "a" (read-only) for const-qualified globals or rodata sections,
        // "aw" (writable) otherwise. GCC uses the const qualification of the
        // variable to determine section flags, not just the section name.
        // This matters for kernel sections like .modinfo which contain const data.
        let flags = if g.is_const || section_name.contains("rodata") {
            "a"
        } else {
            "aw"
        };
        // Sections starting with ".bss" are NOBITS (no file space, BSS semantics)
        let section_type = if section_name.starts_with(".bss") {
            "@nobits"
        } else {
            "@progbits"
        };
        out.emit_fmt(format_args!(
            ".section {},\"{}\",{}",
            section_name, flags, section_type
        ));
        if matches!(g.init, GlobalInit::Zero) || g.size == 0 {
            emit_zero_global(out, g, "@object", ptr_dir);
        } else {
            emit_global_def(out, g, ptr_dir);
        }
        out.emit("");
    }

    // .rodata: const-qualified initialized globals (matches GCC -fno-PIE behavior;
    // the linker handles relocations in .rodata fine, and kernel linker scripts
    // don't recognize .data.rel.ro).
    emit_section_group(
        out,
        globals,
        &classified,
        &GlobalSection::Rodata,
        ".section .rodata",
        false,
        ptr_dir,
    );

    // .tdata: thread-local initialized globals
    emit_section_group(
        out,
        globals,
        &classified,
        &GlobalSection::Tdata,
        ".section .tdata,\"awT\",@progbits",
        false,
        ptr_dir,
    );

    // .data: non-const initialized globals
    emit_section_group(
        out,
        globals,
        &classified,
        &GlobalSection::Data,
        ".section .data",
        false,
        ptr_dir,
    );

    // .comm: zero-initialized common globals (weak linkage, linker merges duplicates).
    // .comm alignment is always in bytes on all platforms, unlike .align.
    for (g, sect) in globals.iter().zip(&classified) {
        if matches!(sect, GlobalSection::Common) {
            out.emit_fmt(format_args!(
                ".comm {},{},{}",
                g.name,
                g.size,
                effective_align(g)
            ));
        }
    }

    // .tbss: thread-local zero-initialized globals
    emit_section_group(
        out,
        globals,
        &classified,
        &GlobalSection::Tbss,
        ".section .tbss,\"awT\",@nobits",
        true,
        ptr_dir,
    );

    // .bss: non-TLS zero-initialized globals (includes zero-size globals with
    // empty initializers like `Type arr[0] = {}` to avoid address overlap).
    emit_section_group(
        out,
        globals,
        &classified,
        &GlobalSection::Bss,
        ".section .bss",
        true,
        ptr_dir,
    );
}

/// Emit all globals matching `target` section, with a section header on first match.
/// If `is_zero` is true, emits as zero-initialized; otherwise as initialized data.
fn emit_section_group(
    out: &mut AsmOutput,
    globals: &[IrGlobal],
    classified: &[GlobalSection],
    target: &GlobalSection,
    section_header: &str,
    is_zero: bool,
    ptr_dir: PtrDirective,
) {
    let mut emitted_header = false;
    for (g, sect) in globals.iter().zip(classified) {
        if sect != target {
            continue;
        }
        if !emitted_header {
            out.emit(section_header);
            emitted_header = true;
        }
        if is_zero {
            let obj_type = if g.is_thread_local {
                "@tls_object"
            } else {
                "@object"
            };
            emit_zero_global(out, g, obj_type, ptr_dir);
        } else {
            emit_global_def(out, g, ptr_dir);
        }
    }
    if emitted_header {
        out.emit("");
    }
}

/// Emit a visibility directive (.hidden, .protected, .internal) for a symbol if applicable.
fn emit_visibility_directive(out: &mut AsmOutput, name: &str, visibility: &Option<String>) {
    if let Some(ref vis) = visibility {
        match vis.as_str() {
            "hidden" => out.emit_fmt(format_args!(".hidden {}", name)),
            "protected" => out.emit_fmt(format_args!(".protected {}", name)),
            "internal" => out.emit_fmt(format_args!(".internal {}", name)),
            _ => {} // "default" or unknown: no directive needed
        }
    }
}

/// Emit linkage directives (.globl or .weak) for a non-static symbol.
fn emit_linkage_directive(out: &mut AsmOutput, name: &str, is_static: bool, is_weak: bool) {
    if !is_static {
        if is_weak {
            out.emit_fmt(format_args!(".weak {}", name));
        } else {
            out.emit_fmt(format_args!(".globl {}", name));
        }
    }
}

/// Emit both linkage (.globl/.weak) and visibility (.hidden/.protected/.internal) directives.
fn emit_symbol_directives(out: &mut AsmOutput, g: &IrGlobal) {
    emit_linkage_directive(out, &g.name, g.is_static, g.is_weak);
    emit_visibility_directive(out, &g.name, &g.visibility);
}

/// Emit a single global variable definition.
fn emit_global_def(out: &mut AsmOutput, g: &IrGlobal, ptr_dir: PtrDirective) {
    emit_symbol_directives(out, g);
    out.emit_fmt(format_args!(
        ".align {}",
        ptr_dir.align_arg(effective_align(g))
    ));
    let obj_type = if g.is_thread_local {
        "@tls_object"
    } else {
        "@object"
    };
    out.emit_fmt(format_args!(".type {}, {}", g.name, obj_type));
    out.emit_fmt(format_args!(".size {}, {}", g.name, g.size));
    out.emit_fmt(format_args!("{}:", g.name));

    emit_init_data(out, &g.init, g.ty, g.size, ptr_dir);
}

/// Emit the data for a single GlobalInit element.
///
/// Handles all init variants: scalars, arrays, strings, global addresses, label diffs,
/// and compound initializers (which recurse into this function for each element).
/// `fallback_ty` is the declared element type of the enclosing global/array, used to
/// widen narrow constants (e.g., IrConst::I32(0) in a pointer array emits .quad 0).
/// `total_size` is the declared size of the enclosing global for padding calculations.
fn emit_init_data(
    out: &mut AsmOutput,
    init: &GlobalInit,
    fallback_ty: IrType,
    total_size: usize,
    ptr_dir: PtrDirective,
) {
    match init {
        GlobalInit::Zero => {
            out.emit_fmt(format_args!("    .zero {}", total_size));
        }
        GlobalInit::Scalar(c) => {
            emit_const_data(out, c, fallback_ty, ptr_dir);
        }
        GlobalInit::Array(values) => {
            // Coalesce consecutive zero-valued elements into .zero directives
            // to avoid emitting millions of individual `.byte 0` lines for
            // large partially-initialized arrays like `char x[500000]={'a'}`.
            let mut i = 0;
            while i < values.len() {
                let val = &values[i];
                let const_ty = const_natural_type(val, fallback_ty);
                // Only widen integer constants to fallback_ty (e.g., I32(0) in a pointer
                // array should emit .quad 0). Float constants (F32, F64, LongDouble) must
                // keep their natural size -- complex arrays store F32 pairs where each zero
                // imaginary slot is exactly 4 bytes, not pointer-sized.
                let elem_ty = if fallback_ty.size() > const_ty.size() && const_ty.is_integer() {
                    fallback_ty
                } else {
                    const_ty
                };

                if val.is_zero() {
                    // Count consecutive zero elements and emit as a single .zero
                    let elem_size = elem_ty.size();
                    let mut zero_count = 1usize;
                    while i + zero_count < values.len() && values[i + zero_count].is_zero() {
                        zero_count += 1;
                    }
                    let zero_bytes = zero_count * elem_size;
                    if zero_bytes > 0 {
                        out.emit_fmt(format_args!("    .zero {}", zero_bytes));
                    }
                    i += zero_count;
                } else {
                    emit_const_data(out, val, elem_ty, ptr_dir);
                    i += 1;
                }
            }
        }
        GlobalInit::String(s) => {
            let string_chars = s.chars().count();
            let string_bytes_with_nul = string_chars + 1;
            if string_bytes_with_nul <= total_size {
                // NUL terminator fits: use .asciz (emits string + NUL)
                out.emit_fmt(format_args!("    .asciz \"{}\"", escape_string(s)));
                if total_size > string_bytes_with_nul {
                    out.emit_fmt(format_args!(
                        "    .zero {}",
                        total_size - string_bytes_with_nul
                    ));
                }
            } else {
                // NUL terminator doesn't fit (C11 6.7.9 p14): truncate to array size.
                // Use .ascii (no implicit NUL) with the string truncated to total_size chars.
                let truncated: String = s.chars().take(total_size).collect();
                out.emit_fmt(format_args!("    .ascii \"{}\"", escape_string(&truncated)));
            }
        }
        GlobalInit::WideString(chars) => {
            emit_wide_string(out, chars);
            let wide_bytes = (chars.len() + 1) * 4;
            if total_size > wide_bytes {
                out.emit_fmt(format_args!("    .zero {}", total_size - wide_bytes));
            }
        }
        GlobalInit::Char16String(chars) => {
            emit_char16_string(out, chars);
            let char16_bytes = (chars.len() + 1) * 2;
            if total_size > char16_bytes {
                out.emit_fmt(format_args!("    .zero {}", total_size - char16_bytes));
            }
        }
        GlobalInit::GlobalAddr(label) => {
            out.emit_fmt(format_args!("    {} {}", ptr_dir.as_str(), label));
        }
        GlobalInit::GlobalAddrOffset(label, offset) => {
            if *offset >= 0 {
                out.emit_fmt(format_args!(
                    "    {} {}+{}",
                    ptr_dir.as_str(),
                    label,
                    offset
                ));
            } else {
                out.emit_fmt(format_args!("    {} {}{}", ptr_dir.as_str(), label, offset));
            }
        }
        GlobalInit::GlobalLabelDiff(lab1, lab2, byte_size) => {
            emit_label_diff(out, lab1, lab2, *byte_size);
        }
        GlobalInit::Compound(elements) => {
            for elem in elements {
                // Compound elements are self-typed: each element knows its own size.
                // For Scalar elements, use the constant's natural type (falling back
                // to the enclosing global's type for I64/wider constants).
                emit_compound_element(out, elem, fallback_ty, ptr_dir);
            }
        }
    }
}

/// Emit a single element within a Compound initializer.
///
/// Most variants delegate to the shared emit_init_data. Scalar elements use the
/// constant's natural type rather than the enclosing global's type, since compound
/// elements may have heterogeneous types (e.g., struct with int and pointer fields).
fn emit_compound_element(
    out: &mut AsmOutput,
    elem: &GlobalInit,
    fallback_ty: IrType,
    ptr_dir: PtrDirective,
) {
    match elem {
        GlobalInit::Scalar(c) => {
            // In compound initializers, each element may have a different type.
            // Use the constant's own type, falling back to fallback_ty for I64 and wider.
            let elem_ty = const_natural_type(c, fallback_ty);
            emit_const_data(out, c, elem_ty, ptr_dir);
        }
        GlobalInit::Zero => {
            // Zero element in compound: emit a single pointer-sized zero
            out.emit_fmt(format_args!("    {} 0", ptr_dir.as_str()));
        }
        GlobalInit::Compound(elements) => {
            // Nested compound: recurse into each element
            for inner in elements {
                emit_compound_element(out, inner, fallback_ty, ptr_dir);
            }
        }
        // All other variants (GlobalAddr, GlobalAddrOffset, WideString, etc.)
        // delegate to the shared handler with zero total_size (no padding).
        other => emit_init_data(out, other, fallback_ty, 0, ptr_dir),
    }
}

/// Get the natural IR type of a constant, falling back to `default_ty` for
/// types that don't have a narrower representation (I64, I128, etc.).
fn const_natural_type(c: &IrConst, default_ty: IrType) -> IrType {
    match c {
        IrConst::I8(_) => IrType::I8,
        IrConst::I16(_) => IrType::I16,
        IrConst::I32(_) => IrType::I32,
        IrConst::F32(_) => IrType::F32,
        IrConst::F64(_) => IrType::F64,
        IrConst::LongDouble(..) => IrType::F128,
        _ => default_ty,
    }
}

/// Emit a wide string (wchar_t) as .long directives with null terminator.
fn emit_wide_string(out: &mut AsmOutput, chars: &[u32]) {
    for &ch in chars {
        out.emit_fmt(format_args!("    .long {}", ch));
    }
    out.emit("    .long 0"); // null terminator
}

/// Emit a char16_t string as .short directives with null terminator.
fn emit_char16_string(out: &mut AsmOutput, chars: &[u16]) {
    for &ch in chars {
        out.emit_fmt(format_args!("    .short {}", ch));
    }
    out.emit("    .short 0"); // null terminator
}

/// Emit a label difference as a sized assembly directive (`.long lab1-lab2`, etc.).
fn emit_label_diff(out: &mut AsmOutput, lab1: &str, lab2: &str, byte_size: usize) {
    let dir = match byte_size {
        1 => ".byte",
        2 => ".short",
        4 => ".long",
        _ => ".quad",
    };
    out.emit_fmt(format_args!("    {} {}-{}", dir, lab1, lab2));
}

/// Emit a 64-bit value as two `.long` directives in little-endian order.
/// Used on i686 (32-bit) targets where 64-bit values must be split.
#[inline]
fn emit_u64_as_long_pair(out: &mut AsmOutput, bits: u64) {
    out.emit_fmt(format_args!("    .long {}", bits as u32));
    out.emit_fmt(format_args!("    .long {}", (bits >> 32) as u32));
}

pub fn emit_const_data(out: &mut AsmOutput, c: &IrConst, ty: IrType, ptr_dir: PtrDirective) {
    match c {
        // Integer constants: all share the same widening/narrowing logic.
        // The value is sign-extended to i64, then emitted at the target type's width.
        IrConst::I8(v) => emit_int_data(out, *v as i64, ty, ptr_dir),
        IrConst::I16(v) => emit_int_data(out, *v as i64, ty, ptr_dir),
        IrConst::I32(v) => emit_int_data(out, *v as i64, ty, ptr_dir),
        IrConst::I64(v) => emit_int_data(out, *v, ty, ptr_dir),
        IrConst::F32(v) => {
            out.emit_fmt(format_args!("    .long {}", v.to_bits()));
        }
        IrConst::F64(v) => {
            let bits = v.to_bits();
            if ptr_dir.is_32bit() {
                emit_u64_as_long_pair(out, bits);
            } else {
                out.emit_fmt(format_args!("    {} {}", ptr_dir.as_str(), bits));
            }
        }
        IrConst::LongDouble(f64_val, f128_bytes) => {
            if ptr_dir.is_x86() {
                // x86: convert f128 bytes to x87 80-bit extended precision for emission.
                // x87 80-bit format = 10 bytes: 8 bytes (significand+exp low) + 2 bytes (exp high+sign)
                let x87 = crate::common::long_double::f128_bytes_to_x87_bytes(f128_bytes);
                let lo = u64::from_le_bytes(x87[0..8].try_into().unwrap());
                let hi = u64::from_le_bytes([x87[8], x87[9], 0, 0, 0, 0, 0, 0]);
                if ptr_dir.is_32bit() {
                    emit_u64_as_long_pair(out, lo);
                    // x87 80-bit: third .long holds the upper 2 bytes
                    out.emit_fmt(format_args!("    .long {}", hi as u32));
                } else {
                    out.emit_fmt(format_args!("    {} {}", ptr_dir.as_str(), lo as i64));
                    out.emit_fmt(format_args!("    {} {}", ptr_dir.as_str(), hi as i64));
                }
            } else if ptr_dir.is_riscv() || ptr_dir.is_arm() {
                // RISC-V and ARM64: f128 bytes are already in IEEE 754 binary128 format.
                let lo = u64::from_le_bytes(f128_bytes[0..8].try_into().unwrap());
                let hi = u64::from_le_bytes(f128_bytes[8..16].try_into().unwrap());
                out.emit_fmt(format_args!("    {} {}", ptr_dir.as_str(), lo as i64));
                out.emit_fmt(format_args!("    {} {}", ptr_dir.as_str(), hi as i64));
            } else {
                // Fallback: store f64 approximation (should not normally be reached).
                let f64_bits = f64_val.to_bits();
                out.emit_fmt(format_args!("    {} {}", ptr_dir.as_str(), f64_bits as i64));
                out.emit_fmt(format_args!("    {} 0", ptr_dir.as_str()));
            }
        }
        IrConst::I128(v) => {
            let lo = *v as u64;
            let hi = (*v >> 64) as u64;
            if ptr_dir.is_32bit() {
                emit_u64_as_long_pair(out, lo);
                emit_u64_as_long_pair(out, hi);
            } else {
                // 64-bit targets: emit as two 64-bit values (little-endian: low quad first)
                out.emit_fmt(format_args!("    {} {}", ptr_dir.as_str(), lo as i64));
                out.emit_fmt(format_args!("    {} {}", ptr_dir.as_str(), hi as i64));
            }
        }
        IrConst::Zero => {
            let size = ty.size();
            out.emit_fmt(format_args!(
                "    .zero {}",
                if size == 0 { 4 } else { size }
            ));
        }
    }
}

/// Emit an integer constant at the width specified by `ty`.
/// Truncates or sign-extends `val` (an i64) as needed to match the target width.
fn emit_int_data(out: &mut AsmOutput, val: i64, ty: IrType, ptr_dir: PtrDirective) {
    match ty {
        IrType::I8 | IrType::U8 => out.emit_fmt(format_args!("    .byte {}", val as u8)),
        IrType::I16 | IrType::U16 => out.emit_fmt(format_args!("    .short {}", val as u16)),
        IrType::I32 | IrType::U32 => out.emit_fmt(format_args!("    .long {}", val as u32)),
        // On i686 (32-bit), pointers are 4 bytes -- emit a single .long, not two.
        IrType::Ptr if ptr_dir.is_32bit() => {
            out.emit_fmt(format_args!("    .long {}", val as u32));
        }
        _ => {
            if ptr_dir.is_32bit() {
                emit_u64_as_long_pair(out, val as u64);
            } else {
                out.emit_fmt(format_args!("    {} {}", ptr_dir.as_str(), val));
            }
        }
    }
}

/// Emit string literal as .byte directives with null terminator.
/// Each char in the string is treated as a raw byte value (0-255),
/// not as a UTF-8 encoded character. This is correct for C narrow
/// string literals where \xNN escapes produce single bytes.
///
/// Writes directly into the output buffer without any intermediate
/// heap allocations (no per-byte String, no Vec, no join). Uses
/// a pre-computed lookup table to convert bytes to decimal strings
/// without fmt::Write overhead.
pub fn emit_string_bytes(out: &mut AsmOutput, s: &str) {
    // Chunk output into lines of at most 32 bytes each to avoid
    // extremely long lines that can cause parser performance issues.
    let mut count = 0;
    for c in s.chars() {
        if count % 32 == 0 {
            if count > 0 {
                out.buf.push('\n');
            }
            out.buf.push_str("    .byte ");
        } else {
            out.buf.push_str(", ");
        }
        push_u8_decimal(&mut out.buf, c as u8);
        count += 1;
    }
    // Null terminator
    if count % 32 == 0 {
        if count > 0 {
            out.buf.push('\n');
        }
        out.buf.push_str("    .byte 0\n");
    } else {
        out.buf.push_str(", 0\n");
    }
}

/// Append a u8 value as a decimal string directly into the buffer.
/// Avoids fmt::Write overhead by using direct digit extraction.
#[inline]
fn push_u8_decimal(buf: &mut String, v: u8) {
    if v >= 100 {
        buf.push((b'0' + v / 100) as char);
        buf.push((b'0' + (v / 10) % 10) as char);
        buf.push((b'0' + v % 10) as char);
    } else if v >= 10 {
        buf.push((b'0' + v / 10) as char);
        buf.push((b'0' + v % 10) as char);
    } else {
        buf.push((b'0' + v) as char);
    }
}

/// Escape a string for use in assembly .asciz directives.
pub fn escape_string(s: &str) -> String {
    let mut result = String::new();
    for c in s.chars() {
        match c {
            '\\' => result.push_str("\\\\"),
            '"' => result.push_str("\\\""),
            '\n' => result.push_str("\\n"),
            '\t' => result.push_str("\\t"),
            '\r' => result.push_str("\\r"),
            '\0' => result.push_str("\\000"),
            c if c.is_ascii_graphic() || c == ' ' => result.push(c),
            c => {
                // Emit the raw byte value (char as u8), not UTF-8 encoding
                use std::fmt::Write;
                let _ = write!(result, "\\{:03o}", c as u8);
            }
        }
    }
    result
}
