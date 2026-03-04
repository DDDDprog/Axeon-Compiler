# ZeoC Language Documentation

**Version: 2.43**
**Axeon Compiler - Modern C-like Programming Language**

---

## Table of Contents

1. [Introduction](#introduction)
2. [Getting Started](#getting-started)
3. [Basic Syntax](#basic-syntax)
4. [Data Types](#data-types)
5. [Variables](#variables)
6. [Functions](#functions)
7. [Control Flow](#control-flow)
8. [Pointers & Arrays](#pointers--arrays)
9. [Structs](#structs)
10. [Standard Library](#standard-library)
11. [Examples](#examples)

---

## Introduction

**ZeoC** is a modern, systems programming language that compiles to C, then to machine code using the Axeon compiler. It combines the performance and low-level control of C with modern language features.

### Features

- Modern syntax with `fn`, `let`, `struct`
- Type safety with explicit type annotations
- Pointer and memory management
- Integrated standard library imports
- Compiles to efficient machine code via Axeon

---

## Getting Started

### Installation

```bash
# Build Axeon compiler
cd Axeon-Compiler
cargo build --release

# Compile a ZeoC program
./target/release/axeon program.zc -o program
```

### Your First Program

```zeoc
import std.io

fn main() -> int {
    print("Hello, ZeoC!")
    return 0
}
```

Save as `hello.zc` and run:
```bash
./target/release/axeon hello.zc -o hello
./hello
```

---

## Basic Syntax

### Comments

```zeoc
// Single line comment

/*
 * Multi-line
 * comment
 */
```

### Statements

Statements end with semicolons (can be auto-added):

```zeoc
let x: int = 10
let y: int = 20
let sum: int = x + y
```

---

## Data Types

### Basic Types

| ZeoC Type | C Equivalent | Description |
|------------|--------------|-------------|
| `int` | `int` | Integer |
| `float` | `float` | Single precision |
| `double` | `double` | Double precision |
| `char` | `char` | Character |
| `bool` | `int` | Boolean |
| `void` | `void` | No value |

### Integer Types

| ZeoC Type | C Equivalent | Size |
|------------|--------------|------|
| `i8` | `int8_t` | 8-bit |
| `i16` | `int16_t` | 16-bit |
| `i32` | `int32_t` | 32-bit |
| `i64` | `int64_t` | 64-bit |
| `u8` | `uint8_t` | 8-bit unsigned |
| `u16` | `uint16_t` | 16-bit unsigned |
| `u32` | `uint32_t` | 32-bit unsigned |
| `u64` | `uint64_t` | 64-bit unsigned |

### Size Types

| ZeoC Type | C Equivalent | Description |
|------------|--------------|-------------|
| `usize` | `size_t` | Unsigned size |
| `isize` | `ssize_t` | Signed size |

### String

```zeoc
let name: string = "Hello"  // Becomes: char* name = "Hello";
```

---

## Variables

### Declaration

```zeoc
let x: int = 10           // With type and initialization
let y: int                // Declaration only (default int)
let name: string = "ZeoC" // String
```

### Mutable Variables

All variables are mutable by default:

```zeoc
let count: int = 0
count = count + 1  // Modify value
```

### Constants

```zeoc
const MAX: int = 100
```

---

## Functions

### Function Declaration

```zeoc
fn add(a: int, b: int) -> int {
    return a + b
}
```

### Return Type

Use `-> type` for return type:

```zeoc
fn get_value() -> int {
    return 42
}
```

### Void Functions

```zeoc
fn greet() -> void {
    print("Hello!")
}
```

### Main Function

```zeoc
fn main() -> int {
    // Your code here
    return 0
}
```

---

## Control Flow

### If-Else

```zeoc
fn max(a: int, b: int) -> int {
    if (a > b) {
        return a
    } else {
        return b
    }
}
```

### While Loop

```zeoc
fn factorial(n: int) -> int {
    let result: int = 1
    let i: int = 1
    
    while (i <= n) {
        result = result * i
        i = i + 1
    }
    
    return result
}
```

### For Loop

```zeoc
fn sum_to(n: int) -> int {
    let sum: int = 0
    
    for (int i = 0; i < n; i = i + 1) {
        sum = sum + i
    }
    
    return sum
}
```

### Range-based For (Experimental)

```zeoc
// for i in 0..10 becomes for (i = 0; i < 10; i++)
for i in 0..10 {
    print(i)
}

// Inclusive range: 0..=10 (i <= 10)
for i in 0..=10 {
    print(i)
}
```

### Switch

```zeoc
fn grade(score: int) -> int {
    switch (score) {
        case 90: return 4
        case 80: return 3
        case 70: return 2
        case 60: return 1
        default: return 0
    }
}
```

### Break & Continue

```zeoc
for (int i = 0; i < 10; i = i + 1) {
    if (i == 5) {
        break      // Exit loop
    }
}

for (int i = 0; i < 10; i = i + 1) {
    if (i % 2 == 0) {
        continue  // Skip iteration
    }
    print(i)
}
```

---

## Pointers & Arrays

### Pointers

```zeoc
fn main() -> int {
    let x: int = 42
    let p: *int = &x    // Pointer to x
    
    return x
}
```

- `*int` becomes `int*`
- `&x` gets address of x

### Arrays (as Pointers)

```zeoc
fn main() -> int {
    let arr: [int] = &x  // Dynamic array (becomes int*)
    
    return arr
}
```

- `[int]` becomes `int*`
- Arrays are pointers in ZeoC

### Pointer Operations

```zeoc
fn main() -> int {
    let x: int = 10
    let p: *int = &x
    
    // Dereference (in unsafe/embedded C)
    // *p = 20
    
    return x
}
```

---

## Structs

### Definition

```zeoc
struct Point {
    x: int,
    y: int
}
```

### Usage

```zeoc
struct Point {
    x: int,
    y: int
}

fn main() -> int {
    let p: Point = Point { x: 10, y: 20 }
    
    print(p.x)
    print(p.y)
    
    return 0
}
```

---

## Standard Library

### Available Imports

```zeoc
import std.io    // printf, basic I/O
import std.string // string functions
import std.memory // malloc, free
import std.math   // math functions
```

### Print

```zeoc
import std.io

fn main() -> int {
    print("Hello!")        // Print string
    print(42)             // Print integer
    println("With newline") // Print with newline
    
    return 0
}
```

---

## Examples

### Hello World

```zeoc
import std.io

fn main() -> int {
    print("Hello, ZeoC!")
    return 0
}
```

### Fibonacci

```zeoc
import std.io

fn fib(n: int) -> int {
    if (n <= 1) {
        return n
    }
    return fib(n - 1) + fib(n - 2)
}

fn main() -> int {
    let i: int = 0
    
    while (i < 10) {
        print(fib(i))
        i = i + 1
    }
    
    return 0
}
```

### Point Struct

```zeoc
import std.io

struct Point {
    x: int,
    y: int
}

fn distance(p1: Point, p2: Point) -> int {
    let dx: int = p2.x - p1.x
    let dy: int = p2.y - p1.y
    return dx * dx + dy * dy
}

fn main() -> int {
    let p1: Point = Point { x: 0, y: 0 }
    let p2: Point = Point { x: 3, y: 4 }
    
    print(distance(p1, p2))
    
    return 0
}
```

---

## Language Keywords

| Keyword | Description |
|---------|-------------|
| `fn` | Function declaration |
| `let` | Variable declaration |
| `struct` | Structure definition |
| `import` | Include standard library |
| `return` | Return from function |
| `if` | Conditional |
| `else` | Alternative branch |
| `while` | While loop |
| `for` | For loop |
| `switch` | Switch statement |
| `case` | Switch case |
| `default` | Default case |
| `break` | Break loop |
| `continue` | Continue loop |
| `unsafe` | Unsafe block (raw pointers) |

---

## Compilation

### Basic Compilation

```bash
# Compile to executable
axeon source.zc -o output

# Compile to assembly
axeon source.zc -S -o output.s

# Compile to object file
axeon source.zc -c -o output.o
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

## Version History

- **2.43**: Current version
  - Unified C + ZeoC pipeline
  - Full control flow support
  - Pointer and array types
  - Struct support

---

For more information, see: [AXEON_COMPLETE_DOC.md](./AXEON_COMPLETE_DOC.md)
