//! External tool invocation: assembler, linker, and dependency files.
//!
//! By default, the compiler uses built-in assembler and linker implementations.

use super::Driver;
use crate::backend::Target;

impl Driver {
    /// Assemble a .s or .S file to an object file.
    ///
    /// Axeon uses its built-in assembler with built-in C preprocessor for .S files.
    pub(super) fn assemble_source_file(
        &self,
        input_file: &str,
        output_path: &str,
    ) -> Result<(), String> {
        // Handle -Wa,--version: print GNU-compatible version string
        if self.assembler_extra_args.iter().any(|a| a == "--version") {
            println!("GNU assembler (Claude's C Compiler built-in) 2.42");
            return Ok(());
        }
        self.assemble_source_file_builtin(input_file, output_path)
    }

    /// Assemble a .s or .S source file using the built-in assembler.
    ///
    /// For .S files (assembly with C preprocessor directives), runs our built-in
    /// C preprocessor first to expand macros, includes, and conditionals, then
    /// passes the result to the target's builtin assembler.
    ///
    /// For .s files (pure assembly), reads the file directly and passes it
    /// to the builtin assembler.
    fn assemble_source_file_builtin(
        &self,
        input_file: &str,
        output_path: &str,
    ) -> Result<(), String> {
        let needs_cpp = input_file.ends_with(".S")
            || self.explicit_language.as_deref() == Some("assembler-with-cpp");
        let asm_text = if needs_cpp {
            // .S files (or -x assembler-with-cpp) need C preprocessing before assembly
            let source = Self::read_source(input_file)?;
            let mut preprocessor = crate::frontend::preprocessor::Preprocessor::new();
            self.configure_preprocessor(&mut preprocessor);
            // GCC defines __ASSEMBLER__ when preprocessing assembly source files (.S).
            // This is needed for headers like <cet.h> which gate assembly-specific
            // macro definitions (e.g. _CET_ENDBR) behind #ifdef __ASSEMBLER__.
            preprocessor.define_macro("__ASSEMBLER__", "1");
            // The built-in preprocessor doesn't ship with GCC's cet.h header.
            // When __CET__ is defined, ffitarget.h does `#include <cet.h>` which
            // would fail. We prevent this by pre-defining _CET_H_INCLUDED (the
            // include guard) so cet.h is skipped if encountered, then manually
            // define _CET_ENDBR and _CET_NOTRACK with the correct values.
            preprocessor.define_macro("_CET_H_INCLUDED", "1");
            if self.target == crate::backend::Target::X86_64 {
                preprocessor.define_macro("_CET_ENDBR", "endbr64");
            } else {
                preprocessor.define_macro("_CET_ENDBR", "endbr32");
            }
            preprocessor.define_macro("_CET_NOTRACK", "notrack");
            // In assembly mode, '$' is the AT&T immediate prefix, not part of
            // identifiers. Without this, `$FOO` is tokenized as one identifier
            // and the macro `FOO` is never expanded.
            preprocessor.set_asm_mode(true);
            preprocessor.set_filename(input_file);
            self.process_force_includes(&mut preprocessor)
                .map_err(|e| format!("Preprocessing {} failed: {}", input_file, e))?;
            preprocessor.preprocess(&source)
        } else {
            // .s files are pure assembly - read directly
            Self::read_source(input_file)?
        };

        // Debug: dump preprocessed assembly to /tmp/asm_debug_<basename>.s
        if std::env::var("AXEON_ASM_DEBUG").is_ok() {
            let basename = std::path::Path::new(input_file)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");
            let _ = std::fs::write(format!("/tmp/asm_debug_{}.s", basename), &asm_text);
        }

        let extra = self.build_asm_extra_args();
        self.target
            .assemble_with_extra(&asm_text, output_path, &extra)
    }

    /// Build extra assembler arguments for RISC-V ABI/arch overrides.
    ///
    /// When -mabi= or -march= are specified on the CLI, these override the
    /// defaults hardcoded in the assembler config. This is critical for the
    /// Linux kernel which uses -mabi=lp64 (soft-float) instead of the default
    /// lp64d (double-float), and -march=rv64imac... instead of rv64gc.
    /// The assembler uses these flags to set ELF e_flags (float ABI, RVC, etc.).
    pub(super) fn build_asm_extra_args(&self) -> Vec<String> {
        let mut args = Vec::new();
        // Only pass RISC-V flags to the RISC-V assembler. Passing -mabi/-march
        // to x86/ARM gcc would cause warnings or errors.
        if self.target == Target::Riscv64 {
            if let Some(ref abi) = self.riscv_abi {
                args.push(format!("-mabi={}", abi));
            }
            if let Some(ref march) = self.riscv_march {
                args.push(format!("-march={}", march));
            }
            if self.riscv_no_relax {
                args.push("-mno-relax".to_string());
            }
            // The RISC-V GNU assembler defaults to PIC mode, which causes
            // `la` pseudo-instructions to expand with R_RISCV_GOT_HI20 (GOT
            // indirection) instead of R_RISCV_PCREL_HI20 (direct PC-relative).
            // The Linux kernel does not have a GOT and expects PCREL relocations,
            // so we must explicitly pass -fno-pic when PIC is not requested.
            if !self.pic {
                args.push("-fno-pic".to_string());
            }
        }
        // Pass through any -Wa, flags from the command line. These are needed
        // when compiling C code that contains inline asm requiring specific
        // assembler settings (e.g., -Wa,-misa-spec=2.2 for RISC-V to enable
        // implicit zicsr in the old ISA spec, required by Linux kernel vDSO).
        for flag in &self.assembler_extra_args {
            args.push(format!("-Wa,{}", flag));
        }
        args
    }

    /// Build linker args from collected flags, preserving command-line ordering.
    ///
    /// Order-independent flags (-shared, -static, -nostdlib, -L paths) go first.
    /// Then linker_ordered_items provides the original CLI ordering of positional
    /// object/archive files, -l flags, and -Wl, pass-through flags. This ordering
    /// is critical for flags like -Wl,--whole-archive which must appear before
    /// the archive they affect.
    pub(super) fn build_linker_args(&self) -> Vec<String> {
        let mut args = Vec::new();
        if self.relocatable {
            // Relocatable link: merge .o files into a single .o without final linking.
            // -nostdlib prevents CRT startup files, -r tells ld to produce a .o.
            args.push("-nostdlib".to_string());
            args.push("-r".to_string());
        }
        if self.shared_lib {
            args.push("-shared".to_string());
        }
        if self.static_link {
            args.push("-static".to_string());
        }
        if self.nostdlib {
            args.push("-nostdlib".to_string());
        }
        for path in &self.linker_paths {
            args.push(format!("-L{}", path));
        }
        // Emit objects, -l flags, and -Wl, flags in their original command-line order.
        args.extend_from_slice(&self.linker_ordered_items);
        args
    }

    /// Write a Make-compatible dependency file for the given input/output.
    /// Format: "output: input\n"
    /// This is a minimal dependency file that tells make the object depends
    /// on its source file. A full implementation would also list included headers.
    pub(super) fn write_dep_file(&self, input_file: &str, output_file: &str) {
        if let Some(ref dep_path) = self.dep_file {
            let dep_path = if dep_path.is_empty() {
                // Derive from output: replace extension with .d
                let p = std::path::Path::new(output_file);
                p.with_extension("d").to_string_lossy().into_owned()
            } else {
                dep_path.clone()
            };
            let input_name = if input_file == "-" {
                "<stdin>"
            } else {
                input_file
            };
            let content = format!("{}: {}\n", output_file, input_name);
            let _ = std::fs::write(&dep_path, content);
        }
    }

    /// Compile a C++ source file using an external C++ compiler (g++ or clang++).
    ///
    /// Since the internal compiler does not have a C++ frontend, we delegate
    /// C++ compilation to a host compiler to generate assembly, then use our
    /// own backend for the rest of the pipeline.
    pub(super) fn compile_cpp_to_assembly(
        &self,
        input_file: &str,
        output_path: &str,
    ) -> Result<String, String> {
        let mut cmd = std::process::Command::new("g++");

        // Forward raw args, skipping -o <path>, -c, -S flags
        let mut skip_next = false;
        for arg in &self.raw_args {
            if skip_next {
                skip_next = false;
                continue;
            }
            match arg.as_str() {
                "-o" => {
                    skip_next = true;
                    continue;
                }
                "-c" | "-S" => continue,
                _ => {}
            }
            cmd.arg(arg);
        }

        cmd.arg("-S");
        cmd.arg("-o").arg(output_path);
        cmd.arg(input_file);

        let result = cmd
            .output()
            .map_err(|e| format!("Failed to run g++ for C++: {}", e))?;
        if !result.status.success() {
            let stderr = String::from_utf8_lossy(&result.stderr);
            return Err(format!(
                "C++ compilation of {} failed: {}",
                input_file, stderr
            ));
        }

        let asm = std::fs::read_to_string(output_path)
            .map_err(|e| format!("Cannot read C++ assembly output {}: {}", output_path, e))?;
        Ok(asm)
    }
}
