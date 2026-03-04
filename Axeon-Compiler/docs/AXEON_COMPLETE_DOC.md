# Axeon Compiler - Complete Documentation

## Table of Contents
1. [Overview](#overview)
2. [Features](#features)
3. [Building & Installation](#building--installation)
4. [Usage](#usage)
5. [Language Support](#language-support)
6. [ZeoC Language](#zeoc-language)
7. [Architecture](#architecture)
8. [Compilation Pipeline](#compilation-pipeline)
9. [Target Architectures](#target-architectures)
10. [Command-Line Options](#command-line-options)
11. [Memory Safety](#memory-safety)
12. [Examples](#examples)

---

## Overview

**Axeon** (stylized as AXEON PRO) is a production-grade modern C compiler written in Rust. It targets multiple architectures including x86-64, i686, ARM64 (AArch64), and RISC-V. 

The compiler is **self-contained** - it does not require external GNU tools (gcc, as, ld) to produce executable binaries. It includes its own:
- C Preprocessor
- Lexer and Parser
- Semantic Analyzer
- Optimizer
- Code Generator
- Assembler
- Linker

### Version History
- **Version 2.43** - Current production version with ZeoC support
- **Version 0.3.0-dev** - Development version

---

## Features

### Core Features
- Production Grade: Fully self-contained compilation
- Multi-architecture: x86-64, i686, ARM64, RISC-V
- Full Pipeline: Preprocess → Lex → Parse → Sema → Lower → Optimize → Codegen
- ELF Generation: Produces Linux ELF executables and shared libraries
- Built-in Linker: Static and dynamic linking support
- Built-in Assembler: Integrated assembler for all architectures

### ZeoC Features (New!)
- Modern Syntax: fn, let, struct, import
- Function Declarations: fn name() -> type
- Variables: let x: int = 5
- Structs: struct Point { x: int, y: int }
- Import System: import std.io, import std.memory
- Unsafe Blocks: Raw pointer operations with unsafe { }
- Option Types: Option<T>, Some(x), None
- Memory Safety Macros: Bounds checking, null checks
- Dynamic Arrays: [int], [char] types

---

## Building & Installation

### Prerequisites
- Rust 1.70+
- GCC or Clang (for system libraries)
- Linux (build and test environment)

### Build Commands
```bash
# Debug build
cargo build

# Release build (recommended for production)
cargo build --release

# Run tests
cargo test
```

### Installation
```bash
# Copy binary to PATH
sudo cp target/release/axeon /usr/local/bin/
```

---

## Usage

### Basic Compilation (ZeoC)
```bash
# Compile ZeoC file to executable
axeon source.zc -o output

# Compile to assembly
axeon source.zc -S -o output.s

# Compile to object file
axeon source.zc -c -o output.o
```

### Basic Compilation (C)
```bash
# Compile C file to executable
axeon source.c -o output

# Compile to assembly
axeon source.c -S -o output.s

# Compile to object file
axeon source.c -c -o output.o
```

### Cross-Compilation
```bash
# x86-64 (default)
axeon source.zc -o output

# ARM64
aarch64-linux-gnu-axeon source.zc -o output

# i686
i686-linux-gnu-axeon source.zc -o output

# RISC-V
riscv64-linux-gnu-axeon source.zc -o output
```

---

## Language Support

### Standard C
The compiler supports C language features including:
- All basic types: int, char, float, double, void
- Type qualifiers: const, volatile, restrict
- Storage classes: static, extern, inline, typedef
- Control structures: if, while, for, switch, goto
- Functions and function pointers
- Structs, unions, bitfields
- Enumerations
- Arrays and pointers
- Preprocessor directives

### C Standards
- C89/C90
- C99
- C11
- C17

---

## ZeoC Language

ZeoC is a modern syntax extension for the Axeon compiler that provides more expressive syntax while compiling to C.

### Function Declarations

```zeoc
// Basic function
fn main() -> int {
    return 0
}

// With parameters
fn add(a: int, b: int) -> int {
    return a + b
}

// No return value
fn greet() -> void {
    print("Hello!")
}
```

### Variable Declarations

```zeoc
// With type annotation
let x: int = 5
let name: string = "Axeon"

// Without initialization
let counter: int

// Type inference (assumes int)
let value = 42
```

### Struct Definitions

```zeoc
struct Point {
    x: int
    y: int
}

struct Person {
    name: string
    age: int
}
```

### Import Statements

```zeoc
import std.io      // #include <stdio.h>
import std.string // #include <string.h>
import std.memory // #include <stdlib.h>
import std.math   // #include <math.h>
```

### Unsafe Blocks

```zeoc
fn main() -> int {
    let x: int = 5
    let p: *int = &x
    
    unsafe {
        let raw = p as *int
    }
    
    return 0
}
```

### Option Types

```zeoc
let value: Option<int> = Some(42)
let empty: Option<int> = None
```

### Dynamic Arrays

```zeoc
let arr: [int] = [1, 2, 3]
```

### Print Statements

```zeoc
print(x)           // prints integer
print("Hello")     // prints string
println(x)         // with newline
```

---

## Architecture

```
src/
├── frontend/           # Language frontend
│   ├── lexer/       # Lexical analysis
│   ├── parser/      # Syntax parsing (AST)
│   ├── preprocessor/# C preprocessor
│   ├── sema/        # Semantic analysis
│   └── zeoc/        # ZeoC transformation
│
├── ir/              # Intermediate representation
│   ├── lowering/   # AST → IR lowering
│   └── mem2reg/    # Mem2reg optimization
│
├── backend/         # Code generation
│   ├── x86/        # x86-64/i686 backend
│   ├── arm/        # ARM/ARM64 backend
│   ├── riscv/      # RISC-V backend
│   ├── elf/        # ELF file format
│   ├── assembler/  # Assembly generation
│   └── linker/     # Linking
│
└── driver/         # Compilation driver
    ├── cli.rs      # Command-line interface
    ├── pipeline.rs # Compilation pipeline
    └── file_types.rs # File type detection
```

---

## Compilation Pipeline

```
Source File (.c / .zc)
        │
        ▼
┌───────────────────┐
│  ZeoC Transform   │ (if .zc file)
│  (zeoc.rs)       │
└───────────────────┘
        │
        ▼
┌───────────────────┐
│  Preprocessor     │
│  (preprocessor/) │
└───────────────────┘
        │
        ▼
┌───────────────────┐
│  Lexer           │
│  (lexer/)        │
└───────────────────┘
        │
        ▼
┌───────────────────┐
│  Parser          │
│  (parser/)       │
└───────────────────┘
        │
        ▼
┌───────────────────┐
│  Semantic Analysis│
│  (sema/)         │
└───────────────────┘
        │
        ▼
┌───────────────────┐
│  IR Lowering     │
│  (ir/lowering/) │
└───────────────────┘
        │
        ▼
┌───────────────────┐
│  Optimization    │
│  (passes/)       │
└───────────────────┘
        │
        ▼
┌───────────────────┐
│  Code Generation │
│  (backend/)      │
└───────────────────┘
        │
        ▼
┌───────────────────┐
│  Assembler       │
│  (assembler/)    │
└───────────────────┘
        │
        ▼
┌───────────────────┐
│  Linker          │
│  (linker/)       │
└───────────────────┘
        │
        ▼
   ELF Executable
```

---

## Target Architectures

| Architecture | Triple | Register Size | Pointer Size |
|--------------|--------|---------------|---------------|
| x86-64 | x86_64-linux-gnu | 64-bit | 8 bytes |
| i686 | i686-linux-gnu | 32-bit | 4 bytes |
| ARM64 | aarch64-linux-gnu | 64-bit | 8 bytes |
| RISC-V | riscv64-linux-gnu | 64-bit | 8 bytes |

---

## Command-Line Options

### Basic Options
| Option | Description |
|--------|-------------|
| -o <file> | Place output into <file> |
| -c | Compile to object file (don't link) |
| -S | Compile to assembly (don't assemble) |
| -E | Preprocess only (show transformed output) |
| -v | Verbose output |

### Optimization
| Option | Description |
|--------|-------------|
| -O0 | No optimization |
| -O1 | Basic optimization |
| -O2 | Standard optimization |
| -O3 | Aggressive optimization |

### Debugging
| Option | Description |
|--------|-------------|
| -g | Generate debug information |
| -Wall | Enable all warnings |
| -Werror | Treat warnings as errors |

### Target
| Option | Description |
|--------|-------------|
| --target <triple> | Specify target triple |
| -march=<arch> | Specify architecture (RISC-V) |
| -mabi=<abi> | Specify ABI (RISC-V) |

---

## Memory Safety

ZeoC provides memory safety through runtime checks:

### Null Pointer Checks
```c
#define ZEOC_ASSERT_NOT_NULL(ptr, msg) \
    do { if (!(ptr)) { fprintf(stderr, "ZeoC: null pointer: %s\n", msg); abort(); } } while(0)
```

### Bounds Checking
```c
#define ZEOC_ASSERT_BOUNDS(idx, len, msg) \
    do { if ((idx) >= (len)) { fprintf(stderr, "ZeoC: out of bounds: %s\n", msg); abort(); } } while(0)
```

### Safe Memory Allocation
```c
#define ZEOC_NEW(type) ((type*)malloc(sizeof(type)))
#define ZEOC_NEW_ARRAY(type, size) ((type*)calloc(size, sizeof(type)))
```

### Runtime Types
```c
// Dynamic array with metadata
typedef struct { 
    void* data; 
    size_t len; 
    size_t cap; 
} __zeoc_array;

// Optional type (like Rust's Option)
typedef struct { 
    bool __is_some; 
    void* __value; 
} __zeoc_option;
```

---

## Examples

### Hello World (C)
```c
#include <stdio.h>

int main() {
    printf("Hello, Axeon!\n");
    return 0;
}
```

### Hello World (ZeoC)
```zeoc
import std.io

fn main() -> int {
    print("Hello, Axeon!")
    return 0
}
```

### Struct Example (ZeoC)
```zeoc
import std.io

struct Point {
    x: int
    y: int
}

fn create_point(a: int, b: int) -> Point {
    let p: Point = Point { x: a, y: b }
    return p
}

fn main() -> int {
    let pt: Point = create_point(10, 20)
    print(pt.x)
    print(pt.y)
    return 0
}
```

### Pointer Example (ZeoC)
```zeoc
import std.io

fn main() -> int {
    let x: int = 42
    let p: *int = &x
    
    unsafe {
        // Raw pointer manipulation
    }
    
    print(x)
    return 0
}
```

---

## Version Information

```
AXEON PRO | Version 2.43
Target: x86_64-linux-gnu
Backend: native
```

---

## License

MIT License
