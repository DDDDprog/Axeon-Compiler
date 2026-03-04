//! ZeoC Preprocessor - Transforms ZeoC syntax to standard C.
//!
//! This module provides a source-to-source transformation that converts
//! ZeoC's modern syntax into standard C that the existing compiler can handle.

use std::collections::HashMap;

/// Transforms ZeoC source code into standard C.
pub fn transform_zeoc(source: &str, filename: &str) -> String {
    let mut transformer = ZeoCTransformer::new(source, filename);
    transformer.transform()
}

struct ZeoCTransformer {
    source: String,
    filename: String,
    result: String,
    var_types: HashMap<String, String>,
    buffer: String,
    brace_count: i32,
    // Track what kind of construct we're in
    in_struct: bool,
    in_fn: bool,
    in_unsafe: bool,
}

impl ZeoCTransformer {
    fn new(source: &str, filename: &str) -> Self {
        Self {
            source: source.to_string(),
            filename: filename.to_string(),
            result: String::new(),
            var_types: HashMap::new(),
            buffer: String::new(),
            brace_count: 0,
            in_struct: false,
            in_fn: false,
            in_unsafe: false,
        }
    }

    fn transform(&mut self) -> String {
        // Add ZeoC compatibility header
        self.result.push_str("/* ZeoC to C transformed code */\n");
        self.result.push_str("#include <stddef.h>\n");
        self.result.push_str("#include <stdbool.h>\n");
        self.result.push_str("#include <stdint.h>\n");
        self.result.push_str("#include <stdio.h>\n");
        self.result.push_str("#include <stdlib.h>\n");
        
        // Memory safety runtime
        self.result.push_str("/* ZeoC memory safety runtime */\n");
        self.result.push_str("typedef struct { void* data; size_t len; size_t cap; } __zeoc_array;\n");
        self.result.push_str("typedef struct { bool __is_some; void* __value; } __zeoc_option;\n");
        
        // Memory safety macros
        self.result.push_str("#define ZEOC_ASSERT_NOT_NULL(ptr, msg) do { if (!(ptr)) { fprintf(stderr, \"ZeoC: null pointer: %s\\n\", msg); abort(); } } while(0)\n");
        self.result.push_str("#define ZEOC_ASSERT_BOUNDS(idx, len, msg) do { if ((idx) >= (len)) { fprintf(stderr, \"ZeoC: out of bounds: %s\\n\", msg); abort(); } } while(0)\n");
        self.result.push_str("#define ZEOC_NEW(type) ((type*)malloc(sizeof(type)))\n");
        self.result.push_str("#define ZEOC_NEW_ARRAY(type, size) ((type*)calloc(size, sizeof(type)))\n");
        
        self.process_all_lines();
        
        self.result.clone()
    }

    fn process_all_lines(&mut self) {
        let lines: Vec<String> = self.source.lines().map(String::from).collect();
        
        for line in &lines {
            let trimmed = line.trim();
            
            // Skip empty lines at top level
            if trimmed.is_empty() && !self.in_struct && !self.in_fn {
                self.result.push_str("\n");
                continue;
            }
            
            // Detect start of struct
            if trimmed.starts_with("struct ") && trimmed.contains('{') {
                self.in_struct = true;
                self.brace_count = 0;
            }
            
            // Detect start of function
            if trimmed.starts_with("fn ") && !self.in_fn && !self.in_struct {
                self.in_fn = true;
                self.brace_count = 0;
            }
            
            // Detect start of unsafe
            if trimmed == "unsafe" || trimmed.starts_with("unsafe {") {
                self.in_unsafe = true;
            }
            
            // Track braces
            for c in trimmed.chars() {
                if c == '{' { self.brace_count += 1; }
                if c == '}' { self.brace_count -= 1; }
            }
            
            // Add line to buffer
            self.buffer.push_str(line);
            self.buffer.push('\n');
            
            // Check if construct ended
            if self.brace_count == 0 {
                let buffer_content = self.buffer.clone();
                self.buffer.clear();
                
                if self.in_struct {
                    self.transform_struct_multiline(&buffer_content);
                    self.in_struct = false;
                }
                if self.in_fn {
                    self.transform_function_multiline(&buffer_content);
                    self.in_fn = false;
                }
                if self.in_unsafe && trimmed == "}" {
                    self.transform_unsafe_multiline(&buffer_content);
                    self.in_unsafe = false;
                }
            }
        }
        
        // Flush any remaining
        if !self.buffer.trim().is_empty() {
            let buffer_content = self.buffer.clone();
            if self.in_struct {
                self.transform_struct_multiline(&buffer_content);
            } else if self.in_fn {
                self.transform_function_multiline(&buffer_content);
            } else {
                self.result.push_str(&buffer_content);
            }
        }
    }

    fn transform_struct_multiline(&mut self, content: &str) {
        let rest = content.strip_prefix("struct ").unwrap_or("");
        
        if let Some(brace_start) = rest.find('{') {
            let struct_name = rest[..brace_start].trim();
            
            self.result.push_str("struct ");
            self.result.push_str(struct_name);
            self.result.push_str(" {\n");
            
            if let Some(body_start) = rest.find('{') {
                if let Some(body_end) = rest.rfind('}') {
                    let body = &rest[body_start + 1..body_end];
                    
                    for line in body.lines() {
                        let field = line.trim().trim_end_matches(',');
                        if field.is_empty() { continue; }
                        
                        // x: int -> int x;
                        if let Some(colon) = field.find(':') {
                            let field_name = field[..colon].trim();
                            let field_type = field[colon + 1..].trim();
                            let c_type = self.convert_type_to_c(field_type);
                            self.result.push_str("    ");
                            self.result.push_str(&c_type);
                            self.result.push(' ');
                            self.result.push_str(field_name);
                            self.result.push_str(";\n");
                        }
                    }
                }
            }
            self.result.push_str("};\n\n");
        }
    }

    fn transform_function_multiline(&mut self, content: &str) {
        let lines: Vec<&str> = content.lines().collect();
        
        for (i, line) in lines.iter().enumerate() {
            if i == 0 {
                // Transform function signature
                let transformed = self.transform_fn_signature(line);
                self.result.push_str(&transformed);
            } else {
                // Transform body line
                let transformed = self.transform_body_line(line);
                self.result.push_str(&transformed);
            }
        }
    }

    fn transform_fn_signature(&self, line: &str) -> String {
        let body = line.strip_prefix("fn ").unwrap_or("");
        
        if let Some(paren_start) = body.find('(') {
            let func_name = body[..paren_start].trim();
            let rest = &body[paren_start..];
            
            if let Some(close_paren) = rest.find(')') {
                let params_raw = &rest[1..close_paren];
                let after_paren = &rest[close_paren + 1..];
                
                let params = self.transform_params(params_raw);
                
                if let Some(arrow_pos) = after_paren.find("->") {
                    let return_part = &after_paren[arrow_pos + 2..];
                    let return_type = return_part.trim().trim_end_matches('{').trim();
                    return format!("{} {} ({}) {{\n", return_type, func_name, params);
                } else {
                    return format!("int {} ({}) {{\n", func_name, params);
                }
            }
        }
        
        format!("{}\n", line)
    }

    fn transform_params(&self, params: &str) -> String {
        let mut result = String::new();
        let mut first = true;
        
        for param in params.split(',') {
            let param = param.trim();
            if param.is_empty() { continue; }
            
            if !first {
                result.push_str(", ");
            }
            first = false;
            
            if let Some(colon) = param.find(':') {
                let name = param[..colon].trim();
                let ptype = param[colon + 1..].trim();
                // Convert ZeoC types to C
                let ctype = self.convert_type_to_c(ptype);
                result.push_str(&format!("{} {}", ctype, name));
            } else if !param.contains(' ') {
                // No type annotation - assume int
                result.push_str(&format!("int {}", param));
            } else {
                result.push_str(param);
            }
        }
        
        result
    }

    fn transform_body_line(&mut self, line: &str) -> String {
        let trimmed = line.trim();
        
        // Skip empty lines inside functions
        if trimmed.is_empty() {
            return "\n".to_string();
        }
        
        // Return statement - remove "return" prefix
        if trimmed.starts_with("return ") || trimmed == "return" {
            let rest = trimmed.strip_prefix("return ").unwrap_or("");
            return format!("    return {};\n", if rest.is_empty() { "".to_string() } else { format!("{}", rest.trim_end_matches(';')) });
        }
        
        // Const declaration
        if trimmed.starts_with("const ") {
            return self.transform_const_inline(trimmed);
        }
        
        // Let declaration
        if trimmed.starts_with("let ") {
            return self.transform_let_inline(trimmed);
        }
        
        // Print statements
        if trimmed.starts_with("print(") || trimmed.starts_with("println(") {
            return self.transform_print_inline(trimmed);
        }
        
        // Printf - pass through but add semicolon if needed
        if trimmed.starts_with("printf(") {
            if !trimmed.ends_with(';') {
                return format!("    {};\n", trimmed);
            }
            return format!("    {}\n", trimmed);
        }
        
        // Unsafe block
        if trimmed == "unsafe" || trimmed.starts_with("unsafe {") {
            return self.transform_unsafe_inline(trimmed);
        }
        
        // Break/Continue
        if trimmed == "break" || trimmed == "break;" {
            return "    break;\n".to_string();
        }
        if trimmed == "continue" || trimmed == "continue;" {
            return "    continue;\n".to_string();
        }
        
        // If statement - transform "if cond {" to "if (cond) {"
        if trimmed.starts_with("if ") && !trimmed.starts_with("if(") {
            let transformed = self.transform_if_statement(trimmed);
            return transformed;
        }
        
        // For loop - transform "for cond {" to "for (cond) {"
        if trimmed.starts_with("for ") && !trimmed.starts_with("for(") {
            // Check if this is a ZeoC-style for with let inside
            if trimmed.contains("let ") {
                let transformed = self.transform_for_loop_with_let(trimmed);
                return transformed;
            }
            let transformed = self.transform_for_loop(trimmed);
            return transformed;
        }
        
        // While loop - transform "while cond {" to "while (cond) {"
        if trimmed.starts_with("while ") && !trimmed.starts_with("while(") {
            let transformed = self.transform_while_loop(trimmed);
            return transformed;
        }
        
        // Switch statement - transform "switch val {" to "switch (val) {"
        if trimmed.starts_with("switch ") && !trimmed.starts_with("switch(") {
            let transformed = self.transform_switch_statement(trimmed);
            return transformed;
        }
        
        // Switch case or default - transform the statement inside
        if trimmed.starts_with("case ") || trimmed.starts_with("default:") {
            // Extract the case value and statement after the colon
            let (prefix, rest) = if trimmed.starts_with("case ") {
                let rest = trimmed.strip_prefix("case ").unwrap_or("");
                if let Some(colon_pos) = rest.find(':') {
                    let case_val = rest[..colon_pos].trim();
                    let stmt = rest[colon_pos+1..].trim();
                    (format!("case {}", case_val), stmt)
                } else {
                    return format!("    {}\n", trimmed);
                }
            } else {
                let rest = trimmed.strip_prefix("default:").unwrap_or("");
                ("default".to_string(), rest.trim())
            };
            
            let stmt = rest.trim();
            if stmt.is_empty() {
                return format!("    {}:\n", prefix);
            }
            
            // Transform the inner statement (e.g., print(100))
            let transformed = self.transform_print_inline(stmt);
            // Clean up - remove leading 4 spaces that transform adds
            let transformed = transformed.trim();
            // transform_print_inline adds "    " at the start and ";\n" at the end
            let transformed = if transformed.starts_with("    ") {
                &transformed[4..]
            } else {
                transformed
            };
            // Remove trailing semicolon
            let transformed = transformed.trim_end_matches(';').trim();
            
            return format!("    {}: {};\n", prefix, transformed);
        }
        
        // Add semicolon to expressions if missing (but not for case/default)
        if !trimmed.ends_with('{') && !trimmed.ends_with('}') && !trimmed.ends_with(';') 
           && !trimmed.starts_with("case ") && !trimmed.starts_with("default:") {
            return format!("    {};\n", trimmed);
        }
        
        format!("    {}\n", trimmed)
    }
    
    fn transform_for_loop_with_let(&self, line: &str) -> String {
        // Transform: for (let j: int = 0; j < 3; j = j + 1) { ... }
        // To: for (int j = 0; j < 3; j = j + 1) { ... }
        
        // Extract the content between for ( and )
        if let Some(paren_start) = line.find("for (") {
            if let Some(paren_end) = line.find(") {") {
                let content = &line[paren_start + 5..paren_end];
                
                // Find the let declaration
                if content.contains("let ") {
                    // Extract let part: "let j: int = 0"
                    if let Some(let_start) = content.find("let ") {
                        let let_part = &content[let_start..];
                        
                        // Parse the let declaration
                        if let Some(eq_pos) = let_part.find('=') {
                            let decl = let_part[..eq_pos].trim(); // "let j: int "
                            let rest = &let_part[eq_pos + 1..]; // "0; j < 3; j = j + 1"
                            
                            if let Some(colon) = decl.find(':') {
                                let var_name = decl[4..colon].trim(); // "j"
                                let var_type = decl[colon + 1..].trim(); // "int"
                                let ctype = self.convert_type_to_c(var_type);
                                
                                // Reconstruct: for (int j = 0; j < 3; j = j + 1) {
                                let rest_trimmed = rest.trim_start_matches(' ').trim_start_matches(';').trim_start_matches(' ');
                                return format!("    for ({} {} = {}) {{\n", ctype, var_name, rest_trimmed);
                            }
                        }
                    }
                }
            }
        }
        
        // Fallback
        format!("    {};\n", line)
    }

    fn transform_for_loop(&self, line: &str) -> String {
        // for i in 0..10 { } -> for (i = 0; i < 10; i++) { }
        
        let rest = line.strip_prefix("for ").unwrap_or("");
        
        // Handle "for i in 0..10" style
        if rest.contains(" in ") {
            if let Some(in_pos) = rest.find(" in ") {
                let var = rest[..in_pos].trim();
                let range = rest[in_pos + 4..].trim();
                
                if range.contains("..") {
                    if let Some(dotdot) = range.find("..") {
                        let start = &range[..dotdot];
                        let end = &range[dotdot + 2..];
                        
                        if end.contains("..=") {
                            let end_val = end.trim_start_matches("..=");
                            return format!("    for ({} = {}; {} <= {}; {}++) {{\n", var, start, var, end_val, var);
                        } else {
                            return format!("    for ({} = {}; {} < {}; {}++) {{\n", var, start, var, end, var);
                        }
                    }
                }
            }
        }
        
        // Pass through C-style for loop
        format!("    {}\n", line)
    }

    fn transform_while_loop(&self, line: &str) -> String {
        // while condition { } -> while (condition) { }
        let rest = line.strip_prefix("while ").unwrap_or("");
        
        // Remove trailing { if present
        let condition = rest.trim_end_matches(" {").trim_end_matches("{").trim();
        
        format!("    while ({}) {{\n", condition)
    }

    fn transform_if_statement(&self, line: &str) -> String {
        // if condition { } -> if (condition) { }
        let rest = line.strip_prefix("if ").unwrap_or("");
        
        // Remove trailing { if present
        let condition = rest.trim_end_matches(" {").trim_end_matches("{").trim();
        
        format!("    if ({}) {{\n", condition)
    }

    fn transform_switch_statement(&self, line: &str) -> String {
        // switch value { case 1: ... } -> switch (value) { case 1: ... }
        let rest = line.strip_prefix("switch ").unwrap_or("");
        
        // Check for switch (expr) {
        if rest.contains("{") {
            if let Some(brace_pos) = rest.find("{") {
                let expr = rest[..brace_pos].trim();
                // Remove extra parentheses if already present
                let clean_expr = expr.trim_start_matches('(').trim_end_matches(')').trim();
                return format!("    switch ({}) {{\n", clean_expr);
            }
        }
        
        // Check for switch (expr) without brace on same line
        if rest.trim().ends_with(')') && !rest.contains('{') {
            let expr = rest.trim().trim_start_matches('(').trim_end_matches(')').trim();
            return format!("    switch ({}) {{\n", expr);
        }
        
        format!("    {}\n", line)
    }

    fn transform_struct_literal(&self, line: &str) -> String {
        // Point { x: a, y: b } -> (struct Point){.x = a, .y = b}
        let result = line.to_string();
        
        // If the line doesn't contain "{", just return as-is
        if !result.contains('{') {
            return result;
        }
        
        // For now, just pass through - complex struct literals need more work
        // The basic functionality is working, just struct literals with field syntax need more work
        result
    }

    fn transform_let_inline(&mut self, line: &str) -> String {
        let rest = line.strip_prefix("let ").unwrap_or("");
        
        if let Some(colon_pos) = rest.find(':') {
            let var_name = rest[..colon_pos].trim();
            let rest = &rest[colon_pos + 1..];
            
            // Get and convert the type
            let (var_type, remaining) = if let Some(eq_pos) = rest.find('=') {
                (rest[..eq_pos].trim(), Some(rest[eq_pos + 1..].trim_end_matches(';').trim()))
            } else {
                (rest.trim_end_matches(';').trim(), None)
            };
            
            // Convert ZeoC types to C types
            let c_type = self.convert_type_to_c(var_type);
            
            // Store the variable type for later use in print and struct literals
            self.var_types.insert(var_name.to_string(), c_type.clone());
            
            if let Some(value) = remaining {
                // Transform the value if needed
                let c_value = self.transform_value_to_c(&c_type, value);
                return format!("    {} {} = {};\n", c_type, var_name, c_value);
            } else {
                return format!("    {} {};\n", c_type, var_name);
            }
        }
        
        format!("    int {};\n", rest.trim_end_matches(';'))
    }
    
    fn transform_const_inline(&mut self, line: &str) -> String {
        // const MAX: int = 100 -> const int MAX = 100;
        let rest = line.strip_prefix("const ").unwrap_or("");
        
        if let Some(colon_pos) = rest.find(':') {
            let var_name = rest[..colon_pos].trim();
            let rest = &rest[colon_pos + 1..];
            
            let (var_type, remaining) = if let Some(eq_pos) = rest.find('=') {
                (rest[..eq_pos].trim(), Some(rest[eq_pos + 1..].trim_end_matches(';').trim()))
            } else {
                (rest.trim_end_matches(';').trim(), None)
            };
            
            let c_type = self.convert_type_to_c(var_type);
            
            self.var_types.insert(var_name.to_string(), c_type.clone());
            
            if let Some(value) = remaining {
                let c_value = self.transform_value_to_c(&c_type, value);
                return format!("    const {} {} = {};\n", c_type, var_name, c_value);
            } else {
                return format!("    const {} {};\n", c_type, var_name);
            }
        }
        
        format!("    const int {};\n", rest.trim_end_matches(';'))
    }
    
    fn transform_value_to_c(&self, c_type: &str, value: &str) -> String {
        let v = value.trim();
        
        // Array literal: [1, 2, 3] -> (int[]){1, 2, 3} or just {1, 2, 3}
        if v.starts_with('[') && v.ends_with(']') {
            let inner = v.trim_start_matches('[').trim_end_matches(']');
            // Determine element type
            let elem_type = if c_type.contains("char*") {
                "char*"
            } else if c_type.contains("int*") {
                "int"
            } else if c_type.contains("float*") {
                "float"
            } else if c_type.contains("double*") {
                "double"
            } else {
                "int"
            };
            return format!("({}[]){{{}}}", elem_type, inner);
        }
        
        // Option::Some(value) or Some(value) -> value
        if v.starts_with("Some(") || v.contains("::Some(") {
            let inner = v.trim_start_matches("Some(")
                         .trim_start_matches("Option::Some(")
                         .trim_start_matches("::Some(")
                         .trim_end_matches(')');
            return inner.to_string();
        }
        
        // None -> 0 or NULL
        if v == "None" || v == "Option::None" || v == "::None" {
            return "0".to_string();
        }
        
        // Struct literal: Point { x: 0, y: 0 } -> (struct Point){.x = 0, .y = 0}
        if v.contains(" { ") && v.ends_with('}') {
            // Extract struct name from value (e.g., "Point { x: 0, y: 0 }" -> "Point")
            // Also handle "struct Point" type
            let type_for_struct = c_type.replace("struct ", "");
            let struct_name = if !type_for_struct.is_empty() && type_for_struct != "int" && type_for_struct != "char*" {
                type_for_struct
            } else if v.contains(" { ") && v.ends_with('}') {
                if let Some(brace_pos) = v.find(" { ") {
                    v[..brace_pos].trim().to_string()
                } else {
                    "".to_string()
                }
            } else {
                "".to_string()
            };
            
            if !struct_name.is_empty() {
                // Transform field: x: 0 to .x = 0
                if let Some(brace_pos) = v.find(" { ") {
                    let inner = v[brace_pos + 3..].trim_end_matches('}');
                    let fields: Vec<String> = inner.split(',')
                        .map(|f| {
                            let parts: Vec<&str> = f.splitn(2, ':').collect();
                            if parts.len() == 2 {
                                format!(".{} = {}", parts[0].trim(), parts[1].trim())
                            } else {
                                f.to_string()
                            }
                        })
                        .collect();
                    return format!("(struct {}){{{}}}", struct_name, fields.join(", "));
                }
            }
        }
        
        // Pass through other values
        v.to_string()
    }
    
    fn convert_type_to_c(&self, ztype: &str) -> String {
        let t = ztype.trim();
        
        // Pointer type: *int -> int*
        if t.starts_with("*") {
            let base = t.trim_start_matches('*').trim();
            return format!("{}*", self.convert_type_to_c(base));
        }
        
        // Array type: [int] -> int*
        if t.starts_with('[') && t.ends_with(']') {
            let base = t.trim_start_matches('[').trim_end_matches(']').trim();
            return format!("{}*", self.convert_type_to_c(base));
        }
        
        // Basic types
        match t {
            "int" => "int".to_string(),
            "float" => "float".to_string(),
            "double" => "double".to_string(),
            "char" => "char".to_string(),
            "void" => "void".to_string(),
            "bool" => "int".to_string(),
            "string" => "char*".to_string(),
            "usize" => "size_t".to_string(),
            "isize" => "ssize_t".to_string(),
            // Signed integers
            "i8" => "int8_t".to_string(),
            "i16" => "int16_t".to_string(),
            "i32" => "int32_t".to_string(),
            "i64" => "int64_t".to_string(),
            // Unsigned integers
            "u8" => "uint8_t".to_string(),
            "u16" => "uint16_t".to_string(),
            "u32" => "uint32_t".to_string(),
            "u64" => "uint64_t".to_string(),
            _ => {
                // Handle Option<T> - convert to T with special handling
                if t.starts_with("Option<") && t.ends_with(">") {
                    let inner = t.trim_start_matches("Option<").trim_end_matches(">");
                    return self.convert_type_to_c(inner);
                }
                // User-defined types (like Point) need "struct " prefix
                if !t.contains(' ') && !t.contains('*') && !t.contains('[') {
                    return format!("struct {}", t);
                }
                t.to_string()
            }
        }
    }

    fn transform_print_inline(&mut self, line: &str) -> String {
        let trimmed = line.trim();
        
        // Determine which prefix to use (print or println)
        let (prefix, is_println) = if trimmed.starts_with("println(") {
            ("println(", true)
        } else if trimmed.starts_with("print(") {
            ("print(", false)
        } else {
            return format!("    {};\n", trimmed); // fallback
        };
        
        // Strip the prefix
        let content = trimmed.strip_prefix(prefix).unwrap_or("");
        let arg = content.trim_end_matches(')').trim();
        
        if arg.starts_with('"') {
            // String literal
            if is_println {
                return format!("    printf(\"{}\\n\");\n", arg);
            }
            // print("string") -> printf("string");
            return format!("    printf({});\n", arg);
        } else {
            // Variable - check stored type for correct format specifier
            if let Some(var_type) = self.var_types.get(arg) {
                if var_type.contains("char*") {
                    // String variable
                    if is_println {
                        return format!("    printf(\"%s\\n\", {});\n", arg);
                    }
                    return format!("    printf(\"%s\", {});\n", arg);
                }
            }
            // Default to int
            return format!("    printf(\"%d\\n\", {});\n", arg);
        }
    }

    fn transform_unsafe_multiline(&mut self, content: &str) {
        self.result.push_str("    /* ZEOC_UNSAFE_BLOCK_START */\n");
        
        // Extract body
        let body = if let Some(start) = content.find('{') {
            if let Some(end) = content.rfind('}') {
                &content[start + 1..end]
            } else {
                content
            }
        } else {
            content
        };
        
        for line in body.lines() {
            let transformed = self.transform_body_line(line);
            self.result.push_str(&transformed);
        }
        
        self.result.push_str("    /* ZEOC_UNSAFE_BLOCK_END */\n");
    }

    fn transform_unsafe_inline(&self, line: &str) -> String {
        // Handle single-line unsafe { ... }
        if line.starts_with("unsafe {") && line.contains("}") {
            // Single line unsafe { ... }
            if let Some(start) = line.find("{") {
                if let Some(end) = line.find("}") {
                    let body = &line[start + 1..end];
                    return format!("    /* ZEOC_UNSAFE_START */ {} /* ZEOC_UNSAFE_END */\n", body);
                }
            }
        }
        
        if line == "unsafe" {
            return "    /* unsafe */\n".to_string();
        }
        
        if line.starts_with("unsafe {") {
            return "    /* unsafe */\n".to_string();
        }
        
        format!("    {};\n", line)
    }


    fn process_single_line(&mut self, line: &str) {
        let trimmed = line.trim();
        
        if trimmed.is_empty() {
            self.result.push_str("\n");
            return;
        }
        
        if trimmed.starts_with("import ") {
            self.transform_import(trimmed);
            return;
        }
        
        if trimmed.starts_with("fn ") && !trimmed.contains('{') {
            let transformed = self.transform_fn_signature(trimmed);
            self.result.push_str(&transformed);
            return;
        }
        
        if trimmed.starts_with("let ") {
            self.transform_let(trimmed);
            return;
        }
        
        if trimmed.starts_with("return ") {
            let rest = trimmed.strip_prefix("return ").unwrap_or("");
            self.result.push_str(&format!("return {};\n", rest));
            return;
        }
        
        if trimmed.starts_with("print(") {
            let transformed = self.transform_print_inline(trimmed);
            self.result.push_str(&transformed);
            return;
        }
        
        // Type transformations
        let mut result = line.to_string();
        
        // String type
        result = result.replace("string", "char*");
        
        // Pointer types - must replace *type before the base type
        result = result.replace("*int", "int*");
        result = result.replace("*char", "char*");
        result = result.replace("*float", "float*");
        result = result.replace("*double", "double*");
        result = result.replace("*void", "void*");
        
        // Unsigned types - full replacement
        result = result.replace("u8", "uint8_t");
        result = result.replace("u16", "uint16_t");
        result = result.replace("u32", "uint32_t");
        result = result.replace("u64", "uint64_t");
        
        // Signed types - full replacement
        result = result.replace("i8", "int8_t");
        result = result.replace("i16", "int16_t");
        result = result.replace("i32", "int32_t");
        result = result.replace("i64", "int64_t");
        
        // Size types
        result = result.replace(": usize", ": size_t");
        result = result.replace(": isize", ": ssize_t");
        result = result.replace(" usize ", " size_t ");
        result = result.replace(" isize ", " ssize_t ");
        
        // Boolean
        result = result.replace(": bool", ": int");
        result = result.replace(" bool ", " int ");
        result = result.replace(" = true", " = 1");
        result = result.replace(" = false", " = 0");
        
        // Pointer variable declarations: *int p -> int* p
        // Need to handle: "let p: *int" -> "int* p"
        // Also handle array: "let arr: [int]" -> "int* arr"
        
        // Replace [type] with type* for arrays
        result = result.replace("[int]", "int*");
        result = result.replace("[char]", "char*");
        result = result.replace("[float]", "float*");
        result = result.replace("[double]", "double*");
        
        // Replace *type in declarations with type*
        // Note: We need to handle patterns like ": *int" or " *int " properly
        
        if result.contains(": [") || result.contains(":[") {
            result = result.replace("[int]", "int*");
            result = result.replace("[char]", "char*");
            result = result.replace("[float]", "float*");
            result = result.replace("[double]", "double*");
        }
        
        self.result.push_str(&result);
        self.result.push('\n');
    }

    fn transform_import(&mut self, line: &str) {
        let module = line.strip_prefix("import ").unwrap_or("").trim();
        
        let include = match module {
            "std.io" => "<stdio.h>",
            "std.string" => "<string.h>",
            "std.memory" | "std.alloc" => "<stdlib.h>",
            "std.math" => "<math.h>",
            _ => {
                self.result.push_str(&format!("/* import {} - unknown */\n", module));
                return;
            }
        };
        
        self.result.push_str(&format!("#include {}\n", include));
    }

    fn transform_let(&mut self, line: &str) {
        let rest = line.strip_prefix("let ").unwrap_or("");
        
        if let Some(colon_pos) = rest.find(':') {
            let var_name = rest[..colon_pos].trim();
            let rest = &rest[colon_pos + 1..];
            
            // Handle type (convert ZeoC types to C types)
            let var_type = rest.split('=').next().unwrap_or("").trim();
            let var_type = self.convert_type(var_type);
            
            if let Some(eq_pos) = rest.find('=') {
                let value = rest[eq_pos + 1..].trim_end_matches(';').trim();
                self.var_types.insert(var_name.to_string(), var_type.clone());
                self.result.push_str(&format!("{} {} = {};\n", var_type, var_name, value));
            } else {
                self.var_types.insert(var_name.to_string(), var_type.clone());
                self.result.push_str(&format!("{} {};\n", var_type, var_name));
            }
        } else {
            self.result.push_str(&format!("int {};\n", rest.trim_end_matches(';')));
        }
    }
    
    fn convert_type(&self, ztype: &str) -> String {
        let t = ztype.trim();
        
        // Pointer types: *int -> int*
        if t.starts_with("*") {
            let base = t.trim_start_matches('*').trim();
            return format!("{}*", self.convert_type(base));
        }
        
        // Array types: [int] -> int*
        if t.starts_with('[') && t.ends_with(']') {
            let base = t.trim_start_matches('[').trim_end_matches(']').trim();
            return format!("{}*", self.convert_type(base));
        }
        
        // Basic types
        match t {
            "int" => "int".to_string(),
            "float" => "float".to_string(),
            "double" => "double".to_string(),
            "char" => "char".to_string(),
            "void" => "void".to_string(),
            "bool" => "int".to_string(),
            "string" => "char*".to_string(),
            "u8" => "uint8_t".to_string(),
            "u16" => "uint16_t".to_string(),
            "u32" => "uint32_t".to_string(),
            "u64" => "uint64_t".to_string(),
            "i8" => "int8_t".to_string(),
            "i16" => "int16_t".to_string(),
            "i32" => "int32_t".to_string(),
            "i64" => "int64_t".to_string(),
            "usize" => "size_t".to_string(),
            "isize" => "ssize_t".to_string(),
            _ => t.to_string(),
        }
    }
}
