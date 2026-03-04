//! Unified Source Handler - ZeoC language processor
//!
//! This module handles ZeoC source files and transforms modern syntax to C
//! for the Axeon compiler pipeline.
//!
//! ZeoC is a modern C-like language that compiles to C then to machine code.

/// Process ZeoC source code
pub fn process_file(source: &str, filename: &str) -> String {
    // All files are processed as ZeoC
    // For .c files, we still transform (allows C with ZeoC features)
    crate::frontend::zeoc::transform_zeoc(source, filename)
}

/// Check if a file should be processed as ZeoC based on extension
pub fn is_zeoc_file(path: &str) -> bool {
    path.ends_with(".zc")
}
