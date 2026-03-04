# Axeon Compiler - ZeoC Language

**ZeoC** - A modern C-like programming language that compiles to machine code via Axeon.

## What is ZeoC?

ZeoC is a modern systems programming language with:
- **Modern Syntax**: `fn`, `let`, `struct`, `import`
- **Type Safety**: Explicit type annotations
- **C-compatible**: Compiles to C then to machine code
- **Fast**: Native performance via Axeon compiler

## Key Features

- **Production Grade**: Fully self-contained - no external GNU tools required
- **Multi-architecture**: x86-64, i686, ARM64, RISC-V
- **Full Pipeline**: Preprocess → Lex → Parse → Sema → IR → Codegen → ASM → Link
- **Built-in Linker**: Static/dynamic linking support
- **Built-in Assembler**: Integrated assembler for all architectures

## Building

### Prerequisites

- Rust (1.70+)
- GCC or Clang (for system libraries)
- Linux (build and test environment)

### Build

```bash
# Debug build
cargo build

# Release build
cargo build --release
```

### Running Tests

```bash
cargo test
```

## Usage

### Basic Compilation (ZeoC)

```bash
# Compile a ZeoC file to an executable
axeon source.zc -o output

# Compile to assembly
axeon source.zc -S -o output.s

# Compile to object file
axeon source.zc -c -o output.o
```

### Basic Compilation (C)

```bash
# Compile a C file to an executable
axeon source.c -o output

# Compile to assembly
axeon source.c -S -o output.s

# Compile to object file
axeon source.c -c -o output.o
```

### Cross-Compilation

The compiler automatically detects the target architecture from the binary name:

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

### Command-Line Options

- `-o <file>` - Place output into `<file>`
- `-S` - Compile to assembly (don't assemble)
- `-c` - Compile to object file (don't link)
- `-v` - Enable verbose output
- `-O0`, `-O1`, `-O2`, `-O3` - Optimization levels
- `-g` - Generate debug information
- `-Wall` - Enable all warnings

## Architecture

The compiler consists of several components:

```
src/
├── frontend/          # C frontend
│   ├── lexer/       # Lexical analysis
│   ├── parser/      # Syntax parsing
│   ├── preprocessor/# C preprocessor
│   └── sema/        # Semantic analysis
├── ir/              # Intermediate representation
├── backend/          # Code generation
│   ├── x86_common/ # x86-64 common code
│   ├── i686/       # i686 backend
│   ├── arm/        # ARM/ARM64 backend
│   └── elf/        # ELF file format
├── assembler/        # Assembler
├── linker/          # Linker
└── driver/          # Compilation driver
```

## Demo

```bash
# Build the demo
axeon demo/main.c demo/greeting.c -o demo/main

# Run it
./demo/main
```

## License

MIT License
