/**
LLVM IR Code Generation via Text

This module generates LLVM IR as text (.ll files) and invokes clang
to produce executables. This approach is simpler and more portable than
using FFI bindings (inkwell).

See docs/LLVM_TEXT_IR.md for design rationale.

## Architecture

The code generator walks the AST and emits LLVM IR text:
- Words → Functions
- Literals → push_int/push_bool/push_string calls
- Word calls → Function calls
- Primitives → Runtime function calls

Example output:

```llvm
define ptr @square(ptr %stack) {
entry:
  %0 = call ptr @dup(ptr %stack)
  %1 = call ptr @multiply(ptr %0)
  ret ptr %1
}
```
*/
pub mod error;
pub mod ir;
pub mod linker;

pub use error::{CodegenError, CodegenResult};
pub use ir::IRGenerator;
pub use linker::{compile_to_object, link_program};

#[cfg(test)]
use crate::ast::SourceLoc;
use crate::ast::{Expr, Pattern, Program, WordDef};
use std::fmt::Write as _;
use std::process::Command;

/// Main code generator
pub struct CodeGen {
    output: String,
    string_globals: String, // Separate area for string constant declarations
    temp_counter: usize,
    string_counter: usize, // Separate counter for string constants (never reset)
    current_block: String, // Track the current basic block label we're emitting into
    metadata_counter: usize, // Counter for debug metadata IDs
    file_metadata: std::collections::HashMap<String, usize>, // filename -> metadata ID
    compile_unit_id: Option<usize>, // ID of the DICompileUnit metadata node
    word_subprograms: Vec<(String, usize, usize, usize)>, // (word_name, file_id, line, subprogram_id)
    current_subprogram_id: Option<usize>, // ID of the current function's DISubprogram
    debug_locations: std::collections::HashMap<(usize, usize, usize, usize), usize>, // (file_id, line, col, scope) -> DILocation ID
    string_constants: std::collections::HashMap<String, String>, // string content -> global name (@.str.N)
    variant_tags: std::collections::HashMap<String, u32>, // variant_name -> tag (index in type definition)
    variant_field_counts: std::collections::HashMap<String, usize>, // variant_name -> number of fields
}

impl CodeGen {
    /// Create a new code generator
    pub fn new() -> Self {
        CodeGen {
            output: String::new(),
            string_globals: String::new(),
            temp_counter: 0,
            string_counter: 0,
            current_block: "entry".to_string(),
            metadata_counter: 0,
            file_metadata: std::collections::HashMap::new(),
            compile_unit_id: None,
            word_subprograms: Vec::new(),
            current_subprogram_id: None,
            debug_locations: std::collections::HashMap::new(),
            string_constants: std::collections::HashMap::new(),
            variant_tags: std::collections::HashMap::new(),
            variant_field_counts: std::collections::HashMap::new(),
        }
    }

    /// Generate a fresh temporary variable name (without % prefix)
    fn fresh_temp(&mut self) -> String {
        let name = format!("{}", self.temp_counter);
        self.temp_counter += 1;
        name
    }

    /// Escape a string for LLVM IR string literals
    /// LLVM IR requires hex escaping for non-printable characters
    fn escape_llvm_string(s: &str) -> String {
        let mut result = String::new();
        for ch in s.chars() {
            match ch {
                // Printable ASCII except backslash and quotes
                ' '..='!' | '#'..='[' | ']'..='~' => result.push(ch),
                // Escape backslash
                '\\' => result.push_str(r"\\"),
                // Escape quote
                '"' => result.push_str(r#"\""#),
                // All other characters as hex escapes
                _ => {
                    for byte in ch.to_string().as_bytes() {
                        result.push_str(&format!(r"\{:02X}", byte));
                    }
                }
            }
        }
        result
    }

    /// Map operator symbols to valid LLVM function names
    /// LLVM doesn't allow symbols like +, -, <, > as function names
    /// Also maps hyphenated Cem names to underscore C names
    fn map_operator_to_function(name: &str) -> String {
        match name {
            // Arithmetic operators (match runtime function names)
            "+" => "add".to_string(),
            "-" => "subtract".to_string(),
            "*" => "multiply".to_string(),
            "/" => "divide".to_string(),
            // Comparison operators (match runtime function names)
            "<" => "lt".to_string(),
            ">" => "gt".to_string(),
            "<=" => "le".to_string(),
            ">=" => "ge".to_string(),
            "=" => "eq".to_string(),
            "!=" => "ne".to_string(),
            // Special functions
            "exit" => "exit_op".to_string(), // Avoid conflict with stdlib exit()
            // For hyphenated names, replace hyphens with underscores
            _ => name.replace('-', "_"),
        }
    }

    /// Compile a complete program to LLVM IR
    pub fn compile_program(&mut self, program: &Program) -> CodegenResult<String> {
        self.compile_program_with_main(program, None)
    }

    /// Compile a complete program to LLVM IR with optional main() function
    ///
    /// # Arguments
    /// * `program` - The AST program to compile
    /// * `entry_word` - Optional name of word to call from main(). If None, no main() is generated.
    ///   If Some("word_name"), generates main() that calls that word and prints result.
    pub fn compile_program_with_main(
        &mut self,
        program: &Program,
        entry_word: Option<&str>,
    ) -> CodegenResult<String> {
        // Emit module header
        writeln!(&mut self.output, "; Cem Compiler - Generated LLVM IR")
            .map_err(|e| CodegenError::InternalError(e.to_string()))?;
        writeln!(&mut self.output).map_err(|e| CodegenError::InternalError(e.to_string()))?;

        // Note: We intentionally omit the target triple to let clang use its default.
        // This avoids "overriding the module target triple" warnings that occur when
        // the IR triple doesn't exactly match clang's compilation target.

        // Declare runtime functions
        self.emit_runtime_declarations()?;

        // Build variant tag map and field count map from type definitions
        // Each variant gets a u32 tag corresponding to its index in the type's variant list
        for typedef in &program.type_defs {
            for (idx, variant) in typedef.variants.iter().enumerate() {
                self.variant_tags.insert(variant.name.clone(), idx as u32);
                self.variant_field_counts
                    .insert(variant.name.clone(), variant.fields.len());
            }
        }

        // Collect all unique source files from the program
        let mut source_files = std::collections::HashSet::new();
        for word in &program.word_defs {
            source_files.insert(word.loc.file.as_ref());
        }

        // Emit debug metadata setup
        self.emit_debug_info_header(&source_files)?;

        // Emit all word definitions
        for word in &program.word_defs {
            self.compile_word(word)?;
        }

        // Generate main() if requested
        if let Some(word_name) = entry_word {
            self.emit_main_function(word_name)?;
        }

        // Emit debug metadata footer (compile unit and module flags)
        self.emit_debug_info_footer()?;

        // Prepend string constants to output
        let final_output = self.string_globals.clone() + &self.output;

        Ok(final_output)
    }

    /// Get the target triple by querying clang
    ///
    /// Note: Currently unused. We intentionally omit target triple from IR
    /// to let clang use its default and avoid "overriding module target" warnings.
    ///
    /// # Returns
    ///
    /// The target triple string (e.g., "x86_64-apple-darwin" or "x86_64-redhat-linux-gnu")
    ///
    /// # Errors
    ///
    /// Returns an error if clang is not found or fails to report its target
    #[allow(dead_code)]
    fn get_target_triple() -> Result<String, std::io::Error> {
        let output = Command::new("clang").arg("-dumpmachine").output()?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(std::io::Error::other("clang -dumpmachine failed"))
        }
    }

    /// Emit declarations for all runtime functions
    fn emit_runtime_declarations(&mut self) -> CodegenResult<()> {
        writeln!(&mut self.output, "; Runtime function declarations")
            .map_err(|e| CodegenError::InternalError(e.to_string()))?;

        // Stack operations (ptr -> ptr)
        for func in &["dup", "drop", "swap", "over", "rot", "nip", "tuck"] {
            writeln!(&mut self.output, "declare ptr @{}(ptr)", func)
                .map_err(|e| CodegenError::InternalError(e.to_string()))?;
        }

        // Arithmetic (ptr -> ptr)
        for func in &["add", "subtract", "multiply", "divide"] {
            writeln!(&mut self.output, "declare ptr @{}(ptr)", func)
                .map_err(|e| CodegenError::InternalError(e.to_string()))?;
        }

        // Comparisons (ptr -> ptr)
        for func in &["lt", "gt", "le", "ge", "eq", "ne"] {
            writeln!(&mut self.output, "declare ptr @{}(ptr)", func)
                .map_err(|e| CodegenError::InternalError(e.to_string()))?;
        }

        // Push operations
        writeln!(&mut self.output, "declare ptr @push_int(ptr, i64)")
            .map_err(|e| CodegenError::InternalError(e.to_string()))?;
        writeln!(&mut self.output, "declare ptr @push_bool(ptr, i1)")
            .map_err(|e| CodegenError::InternalError(e.to_string()))?;
        writeln!(&mut self.output, "declare ptr @push_string(ptr, ptr)")
            .map_err(|e| CodegenError::InternalError(e.to_string()))?;
        writeln!(&mut self.output, "declare ptr @push_quotation(ptr, ptr)")
            .map_err(|e| CodegenError::InternalError(e.to_string()))?;
        writeln!(&mut self.output, "declare ptr @push_variant(ptr, i32, ptr)")
            .map_err(|e| CodegenError::InternalError(e.to_string()))?;

        // Control flow operations
        writeln!(&mut self.output, "declare ptr @call_quotation(ptr)")
            .map_err(|e| CodegenError::InternalError(e.to_string()))?;

        // String operations
        writeln!(&mut self.output, "declare ptr @string_length(ptr)")
            .map_err(|e| CodegenError::InternalError(e.to_string()))?;
        writeln!(&mut self.output, "declare ptr @string_concat(ptr)")
            .map_err(|e| CodegenError::InternalError(e.to_string()))?;
        writeln!(&mut self.output, "declare ptr @string_equal(ptr)")
            .map_err(|e| CodegenError::InternalError(e.to_string()))?;

        // Type conversions
        writeln!(&mut self.output, "declare ptr @int_to_string(ptr)")
            .map_err(|e| CodegenError::InternalError(e.to_string()))?;
        writeln!(&mut self.output, "declare ptr @bool_to_string(ptr)")
            .map_err(|e| CodegenError::InternalError(e.to_string()))?;

        // Exit operation
        writeln!(&mut self.output, "declare void @exit_op(ptr)")
            .map_err(|e| CodegenError::InternalError(e.to_string()))?;

        // Scheduler operations (testing)
        writeln!(&mut self.output, "declare ptr @test_yield(ptr)")
            .map_err(|e| CodegenError::InternalError(e.to_string()))?;

        // I/O operations (async)
        writeln!(&mut self.output, "declare ptr @write_line(ptr)")
            .map_err(|e| CodegenError::InternalError(e.to_string()))?;
        writeln!(&mut self.output, "declare ptr @read_line(ptr)")
            .map_err(|e| CodegenError::InternalError(e.to_string()))?;

        // Scheduler operations
        writeln!(&mut self.output, "declare void @scheduler_init()")
            .map_err(|e| CodegenError::InternalError(e.to_string()))?;
        writeln!(&mut self.output, "declare ptr @scheduler_run()")
            .map_err(|e| CodegenError::InternalError(e.to_string()))?;
        writeln!(&mut self.output, "declare void @scheduler_shutdown()")
            .map_err(|e| CodegenError::InternalError(e.to_string()))?;
        writeln!(&mut self.output, "declare i64 @strand_spawn(ptr, ptr)")
            .map_err(|e| CodegenError::InternalError(e.to_string()))?;

        // Utility functions
        writeln!(&mut self.output, "declare void @print_stack(ptr)")
            .map_err(|e| CodegenError::InternalError(e.to_string()))?;
        writeln!(&mut self.output, "declare void @free_stack(ptr)")
            .map_err(|e| CodegenError::InternalError(e.to_string()))?;
        writeln!(&mut self.output, "declare void @runtime_error(ptr)")
            .map_err(|e| CodegenError::InternalError(e.to_string()))?;
        writeln!(&mut self.output, "declare ptr @alloc_cell()")
            .map_err(|e| CodegenError::InternalError(e.to_string()))?;

        // LLVM intrinsics
        writeln!(
            &mut self.output,
            "declare void @llvm.memcpy.p0.p0.i64(ptr noalias nocapture writeonly, ptr noalias nocapture readonly, i64, i1 immarg)"
        )
        .map_err(|e| CodegenError::InternalError(e.to_string()))?;

        writeln!(&mut self.output).map_err(|e| CodegenError::InternalError(e.to_string()))?;
        Ok(())
    }

    /// Emit a main() function that calls an entry word
    ///
    /// Generates:
    /// ```llvm
    /// define i32 @main() {
    /// entry:
    ///   %stack = call ptr @entry_word(ptr null)
    ///   call void @print_stack(ptr %stack)
    ///   call void @free_stack(ptr %stack)
    ///   ret i32 0
    /// }
    /// ```
    fn emit_main_function(&mut self, entry_word: &str) -> CodegenResult<()> {
        // Avoid name collision - if entry word is "main", it was renamed to "cem_main"
        let function_name = if entry_word == "main" {
            "cem_main"
        } else {
            entry_word
        };

        writeln!(&mut self.output, "; Main function")
            .map_err(|e| CodegenError::InternalError(e.to_string()))?;
        writeln!(&mut self.output, "define i32 @main() {{")
            .map_err(|e| CodegenError::InternalError(e.to_string()))?;
        writeln!(&mut self.output, "entry:")
            .map_err(|e| CodegenError::InternalError(e.to_string()))?;

        // Initialize scheduler for async I/O
        writeln!(&mut self.output, "  call void @scheduler_init()")
            .map_err(|e| CodegenError::InternalError(e.to_string()))?;

        // Spawn entry word as a strand
        writeln!(
            &mut self.output,
            "  call i64 @strand_spawn(ptr @{}, ptr null)",
            function_name
        )
        .map_err(|e| CodegenError::InternalError(e.to_string()))?;

        // Run scheduler (returns final stack from main strand)
        writeln!(&mut self.output, "  %stack = call ptr @scheduler_run()")
            .map_err(|e| CodegenError::InternalError(e.to_string()))?;

        // Shutdown scheduler
        writeln!(&mut self.output, "  call void @scheduler_shutdown()")
            .map_err(|e| CodegenError::InternalError(e.to_string()))?;

        // Clean up
        writeln!(&mut self.output, "  call void @free_stack(ptr %stack)")
            .map_err(|e| CodegenError::InternalError(e.to_string()))?;

        writeln!(&mut self.output, "  ret i32 0")
            .map_err(|e| CodegenError::InternalError(e.to_string()))?;
        writeln!(&mut self.output, "}}").map_err(|e| CodegenError::InternalError(e.to_string()))?;
        writeln!(&mut self.output).map_err(|e| CodegenError::InternalError(e.to_string()))?;
        Ok(())
    }

    /// Emit debug info header: DIFile nodes for each source file
    fn emit_debug_info_header(
        &mut self,
        source_files: &std::collections::HashSet<&str>,
    ) -> CodegenResult<()> {
        writeln!(&mut self.output, "; Debug Information")
            .map_err(|e| CodegenError::InternalError(e.to_string()))?;
        writeln!(&mut self.output).map_err(|e| CodegenError::InternalError(e.to_string()))?;

        // Emit DIFile for each unique source file
        for filename in source_files {
            let metadata_id = self.fresh_metadata_id();
            self.file_metadata.insert(filename.to_string(), metadata_id);

            // Split filename into directory and basename
            let path = std::path::Path::new(filename);
            let (directory, basename) = if let Some(parent) = path.parent() {
                let dir = parent.to_string_lossy();
                let base = path
                    .file_name()
                    .map(|s| s.to_string_lossy())
                    .unwrap_or_default();
                (dir.to_string(), base.to_string())
            } else {
                (".".to_string(), filename.to_string())
            };

            // Escape strings for LLVM IR (backslashes and quotes)
            let escaped_basename = basename.replace('\\', r"\\").replace('"', r#"\""#);
            let escaped_directory = directory.replace('\\', r"\\").replace('"', r#"\""#);

            writeln!(
                &mut self.output,
                "!{} = !DIFile(filename: \"{}\", directory: \"{}\")",
                metadata_id, escaped_basename, escaped_directory
            )
            .map_err(|e| CodegenError::InternalError(e.to_string()))?;
        }

        writeln!(&mut self.output).map_err(|e| CodegenError::InternalError(e.to_string()))?;

        Ok(())
    }

    /// Emit debug info footer: DICompileUnit, DISubprograms, and module flags
    fn emit_debug_info_footer(&mut self) -> CodegenResult<()> {
        writeln!(&mut self.output, "; Debug Info Compile Unit")
            .map_err(|e| CodegenError::InternalError(e.to_string()))?;

        let cu_id = self.fresh_metadata_id();
        self.compile_unit_id = Some(cu_id);

        // Get the main compile unit file (deterministic: pick lowest ID)
        // For empty programs, create a placeholder file
        let main_file_id = if self.file_metadata.is_empty() {
            let placeholder_id = self.fresh_metadata_id();
            writeln!(
                &mut self.output,
                "!{} = !DIFile(filename: \"<empty>\", directory: \".\")",
                placeholder_id
            )
            .map_err(|e| CodegenError::InternalError(e.to_string()))?;
            placeholder_id
        } else {
            // Use the file with the smallest ID for determinism
            *self.file_metadata.values().min().unwrap()
        };

        writeln!(&mut self.output,
            "!{} = distinct !DICompileUnit(language: DW_LANG_C, file: !{}, producer: \"Cem Compiler\", isOptimized: false, runtimeVersion: 0, emissionKind: FullDebug)",
            cu_id, main_file_id
        ).map_err(|e| CodegenError::InternalError(e.to_string()))?;

        // Emit DISubprogram for each word
        writeln!(&mut self.output).map_err(|e| CodegenError::InternalError(e.to_string()))?;
        writeln!(&mut self.output, "; DISubprogram metadata for each word")
            .map_err(|e| CodegenError::InternalError(e.to_string()))?;

        // Pre-allocate type IDs to avoid borrow checker issues
        let type_ids: Vec<usize> = (0..self.word_subprograms.len())
            .map(|_| self.fresh_metadata_id())
            .collect();

        for (i, (word_name, file_id, line, subprogram_id)) in
            self.word_subprograms.iter().enumerate()
        {
            let type_id = type_ids[i];
            writeln!(&mut self.output,
                "!{} = distinct !DISubprogram(name: \"{}\", scope: !{}, file: !{}, line: {}, type: !{}, scopeLine: {}, flags: DIFlagPrototyped, spFlags: DISPFlagDefinition, unit: !{})",
                subprogram_id, word_name, file_id, file_id, line, type_id, line, cu_id
            ).map_err(|e| CodegenError::InternalError(e.to_string()))?;
        }

        // Emit stub type metadata for each function type
        writeln!(&mut self.output).map_err(|e| CodegenError::InternalError(e.to_string()))?;
        writeln!(&mut self.output, "; Type metadata (stubs)")
            .map_err(|e| CodegenError::InternalError(e.to_string()))?;
        for type_id in type_ids {
            writeln!(
                &mut self.output,
                "!{} = !DISubroutineType(types: !{{}})",
                type_id
            )
            .map_err(|e| CodegenError::InternalError(e.to_string()))?;
        }

        // Emit DILocation metadata for each source location
        if !self.debug_locations.is_empty() {
            writeln!(&mut self.output).map_err(|e| CodegenError::InternalError(e.to_string()))?;
            writeln!(&mut self.output, "; DILocation metadata")
                .map_err(|e| CodegenError::InternalError(e.to_string()))?;

            // Sort locations by ID for consistent output
            let mut locations: Vec<_> = self.debug_locations.iter().collect();
            locations.sort_by_key(|(_, loc_id)| *loc_id);

            for ((_file_id, line, column, scope_id), loc_id) in locations {
                writeln!(
                    &mut self.output,
                    "!{} = !DILocation(line: {}, column: {}, scope: !{})",
                    loc_id, line, column, scope_id
                )
                .map_err(|e| CodegenError::InternalError(e.to_string()))?;
            }
        }

        // Emit module flags for debug info
        writeln!(&mut self.output).map_err(|e| CodegenError::InternalError(e.to_string()))?;
        writeln!(&mut self.output, "!llvm.dbg.cu = !{{!{}}}", cu_id)
            .map_err(|e| CodegenError::InternalError(e.to_string()))?;

        let flags_id = self.fresh_metadata_id();
        writeln!(&mut self.output, "!llvm.module.flags = !{{!{}}}", flags_id)
            .map_err(|e| CodegenError::InternalError(e.to_string()))?;
        writeln!(
            &mut self.output,
            "!{} = !{{i32 2, !\"Debug Info Version\", i32 3}}",
            flags_id
        )
        .map_err(|e| CodegenError::InternalError(e.to_string()))?;

        writeln!(&mut self.output).map_err(|e| CodegenError::InternalError(e.to_string()))?;

        Ok(())
    }

    /// Generate a fresh metadata ID
    fn fresh_metadata_id(&mut self) -> usize {
        let id = self.metadata_counter;
        self.metadata_counter += 1;
        id
    }

    /// Get or create a DILocation for the given source location
    /// Returns the metadata ID to use in !dbg annotations
    fn get_debug_location(&mut self, loc: &crate::ast::SourceLoc) -> Option<usize> {
        // Only create debug locations if we're inside a function
        let subprogram_id = self.current_subprogram_id?;

        // Get file ID
        let file_id = self.file_metadata.get(loc.file.as_ref()).copied()?;

        // Include subprogram in the key to allow same line/col in different functions
        let key = (file_id, loc.line, loc.column, subprogram_id);
        if let Some(&loc_id) = self.debug_locations.get(&key) {
            return Some(loc_id);
        }

        // Create new DILocation
        let loc_id = self.fresh_metadata_id();
        self.debug_locations.insert(key, loc_id);

        Some(loc_id)
    }

    /// Format a !dbg annotation for the given source location
    /// Returns ", !dbg !N" if location is available, empty string otherwise
    fn dbg_annotation(&mut self, loc: &crate::ast::SourceLoc) -> String {
        if let Some(loc_id) = self.get_debug_location(loc) {
            format!(", !dbg !{}", loc_id)
        } else {
            String::new()
        }
    }

    /// Register a word for debug metadata emission
    /// Allocates a subprogram ID and stores info for later emission
    /// Returns the subprogram ID to attach to the function
    fn register_word_subprogram(&mut self, word: &WordDef) -> CodegenResult<usize> {
        let subprogram_id = self.fresh_metadata_id();

        // Get the file metadata ID for this word's source location
        let file_id = self
            .file_metadata
            .get(word.loc.file.as_ref())
            .copied()
            .unwrap_or(0);

        self.word_subprograms
            .push((word.name.clone(), file_id, word.loc.line, subprogram_id));

        Ok(subprogram_id)
    }

    /// Compile a word definition to LLVM function
    fn compile_word(&mut self, word: &WordDef) -> CodegenResult<()> {
        self.temp_counter = 0; // Reset for each function
        self.current_block = "entry".to_string(); // Reset to entry block

        // Register this word for debug metadata (allocates ID for later emission)
        let subprogram_id = self.register_word_subprogram(word)?;

        // Set current subprogram for debug location generation
        self.current_subprogram_id = Some(subprogram_id);

        // Map word name to function name (handles operators and hyphenated names)
        // Also avoid name collision with C main() - prefix Cem "main" word with "cem_"
        let function_name = if word.name == "main" {
            "cem_main".to_string()
        } else {
            Self::map_operator_to_function(&word.name)
        };

        // Emit function definition with debug metadata attachment
        writeln!(
            &mut self.output,
            "define ptr @{}(ptr %stack) !dbg !{} {{",
            function_name, subprogram_id
        )
        .map_err(|e| CodegenError::InternalError(e.to_string()))?;
        writeln!(&mut self.output, "entry:")
            .map_err(|e| CodegenError::InternalError(e.to_string()))?;

        // Compile all expressions in the word body
        let (final_stack, _ends_with_musttail) = self.compile_expr_sequence(&word.body, "stack")?;

        // Check if all paths have already terminated (match/if with all branches returning)
        // This is the OPPOSITE of check_all_paths_returned:
        //   check_all_paths_returned returns true if caller SHOULD emit ret (WordCall case)
        //   We want to know if all paths ALREADY emitted ret (Match/If case)
        let all_paths_already_terminated = word
            .body
            .last()
            .is_some_and(|e| self.check_all_branches_already_returned(e));

        // Emit ret unless all paths have already emitted ret
        if !all_paths_already_terminated {
            writeln!(&mut self.output, "  ret ptr %{}", final_stack)
                .map_err(|e| CodegenError::InternalError(e.to_string()))?;
        }

        writeln!(&mut self.output, "}}").map_err(|e| CodegenError::InternalError(e.to_string()))?;
        writeln!(&mut self.output).map_err(|e| CodegenError::InternalError(e.to_string()))?;

        // Clear current subprogram
        self.current_subprogram_id = None;

        Ok(())
    }

    /// Check if an expression will have all code paths return (needs caller to emit ret)
    /// Returns true if the expression needs the caller to emit ret (WordCall)
    /// or if all branches end with expressions that need ret (Match/If with all branches returning)
    fn check_all_paths_returned(&self, expr: &Expr) -> bool {
        match expr {
            // A word call (non-variant) in tail position will be compiled as musttail
            // The parent context (match branch or word body) will emit the ret statement
            Expr::WordCall(name, _) => !self.variant_tags.contains_key(name),

            // Match emits ret for each branch if all branches end with musttail
            Expr::Match { branches, .. } => branches.iter().all(|b| {
                b.body
                    .last()
                    .is_some_and(|e| self.check_all_paths_returned(e))
            }),

            // If emits ret for both branches if both end with musttail
            Expr::If {
                then_branch,
                else_branch,
                ..
            } => {
                let then_musttail = if let Expr::Quotation(exprs, _) = &**then_branch {
                    exprs
                        .last()
                        .is_some_and(|e| self.check_all_paths_returned(e))
                } else {
                    false
                };
                let else_musttail = if let Expr::Quotation(exprs, _) = &**else_branch {
                    exprs
                        .last()
                        .is_some_and(|e| self.check_all_paths_returned(e))
                } else {
                    false
                };
                then_musttail && else_musttail
            }

            _ => false,
        }
    }

    /// Check if all branches of a Match/If have already emitted ret
    /// This is different from check_all_paths_returned:
    ///   - WordCall: false (needs ret to be emitted)
    ///   - Match with all branches WordCall: true (all branches already emitted ret)
    fn check_all_branches_already_returned(&self, expr: &Expr) -> bool {
        match expr {
            // WordCall needs ret to be emitted, hasn't already returned
            Expr::WordCall(_, _) => false,

            // Match has all branches returned if all end with expressions that return
            Expr::Match { branches, .. } => branches.iter().all(|b| {
                b.body
                    .last()
                    .is_some_and(|e| self.check_all_paths_returned(e))
            }),

            // If has all branches returned if both end with expressions that return
            Expr::If {
                then_branch,
                else_branch,
                ..
            } => {
                let then_returned = if let Expr::Quotation(exprs, _) = &**then_branch {
                    exprs
                        .last()
                        .is_some_and(|e| self.check_all_paths_returned(e))
                } else {
                    false
                };
                let else_returned = if let Expr::Quotation(exprs, _) = &**else_branch {
                    exprs
                        .last()
                        .is_some_and(|e| self.check_all_paths_returned(e))
                } else {
                    false
                };
                then_returned && else_returned
            }

            _ => false,
        }
    }

    /// Compile a branch quotation (quotation inside then/else)
    /// Returns (result_var, ends_with_musttail)
    ///
    /// ends_with_musttail is true if the last expression in the quotation
    /// is a WordCall in tail position (which will be compiled as a musttail call)
    fn compile_branch_quotation(
        &mut self,
        quot: &Expr,
        initial_stack: &str,
    ) -> CodegenResult<(String, bool)> {
        match quot {
            Expr::Quotation(exprs, _loc) => self.compile_expr_sequence(exprs, initial_stack),
            _ => Err(CodegenError::InternalError(
                "If branches must be quotations".to_string(),
            )),
        }
    }

    /// Compile a sequence of expressions (used for quotations and match branches)
    /// Returns (final_stack_var, ends_with_musttail)
    ///
    /// ends_with_musttail is true if the sequence ends with a WordCall in tail position
    /// (which will be compiled as a musttail call). The caller should emit `ret ptr %stack`.
    ///
    /// If the sequence ends with a Match/If where all branches return, ends_with_musttail
    /// is false but all code paths have already terminated. The caller should check
    /// check_all_paths_returned() to determine this case.
    fn compile_expr_sequence(
        &mut self,
        exprs: &[Expr],
        initial_stack: &str,
    ) -> CodegenResult<(String, bool)> {
        let mut stack_var = initial_stack.to_string();
        let len = exprs.len();

        // Empty sequences don't end with musttail
        if len == 0 {
            return Ok((stack_var, false));
        }

        let mut ends_with_musttail = false;

        for (i, expr) in exprs.iter().enumerate() {
            let is_tail = i == len - 1; // Track tail position in branch
            stack_var = self.compile_expr_with_context(expr, &stack_var, is_tail)?;

            // Check if the last expression is a WordCall in tail position
            if is_tail
                && let Expr::WordCall(name, _) = expr
                && !self.variant_tags.contains_key(name)
            {
                ends_with_musttail = true;
            }
        }
        Ok((stack_var, ends_with_musttail))
    }

    /// Compile a single expression with tail-call context
    fn compile_expr_with_context(
        &mut self,
        expr: &Expr,
        stack: &str,
        in_tail_position: bool,
    ) -> CodegenResult<String> {
        match expr {
            // Tail-call optimization: if in tail position and calling a word, use musttail
            // BUT: variant constructors are not actual functions, so they can't be tail-called
            Expr::WordCall(name, loc)
                if in_tail_position && !self.variant_tags.contains_key(name) =>
            {
                let result = self.fresh_temp();
                let dbg = self.dbg_annotation(loc);
                let func_name = Self::map_operator_to_function(name);
                writeln!(
                    &mut self.output,
                    "  %{} = musttail call ptr @{}(ptr %{}){}",
                    result, func_name, stack, dbg
                )
                .map_err(|e| CodegenError::InternalError(e.to_string()))?;
                Ok(result)
            }
            // Otherwise, delegate to normal compile_expr
            _ => self.compile_expr(expr, stack),
        }
    }

    /// Compile a single expression, returning the new stack variable name
    fn compile_expr(&mut self, expr: &Expr, stack: &str) -> CodegenResult<String> {
        match expr {
            Expr::IntLit(n, loc) => {
                let result = self.fresh_temp();
                let dbg = self.dbg_annotation(loc);
                writeln!(
                    &mut self.output,
                    "  %{} = call ptr @push_int(ptr %{}, i64 {}){}",
                    result, stack, n, dbg
                )
                .map_err(|e| CodegenError::InternalError(e.to_string()))?;
                Ok(result)
            }

            Expr::BoolLit(b, loc) => {
                let result = self.fresh_temp();
                let value = if *b { 1 } else { 0 };
                let dbg = self.dbg_annotation(loc);
                writeln!(
                    &mut self.output,
                    "  %{} = call ptr @push_bool(ptr %{}, i1 {}){}",
                    result, stack, value, dbg
                )
                .map_err(|e| CodegenError::InternalError(e.to_string()))?;
                Ok(result)
            }

            Expr::StringLit(s, loc) => {
                // String deduplication: Check if we've already emitted this exact string content.
                // Without this, identical strings like "hello" appearing multiple times in the
                // source would create separate @.str.N globals for each occurrence, bloating
                // the binary. By reusing the same global, we reduce IR size and memory usage.
                let str_global = if let Some(existing) = self.string_constants.get(s) {
                    existing.clone()
                } else {
                    // Create new global string constant
                    let str_global = format!("@.str.{}", self.string_counter);
                    self.string_counter += 1; // Increment for the string global itself

                    let escaped = Self::escape_llvm_string(s);
                    // Length is original byte count - escaping is just text representation.
                    // E.g., "a\"b" is 3 bytes even though we write it as 5 chars in IR text.
                    // UTF-8 chars like "😀" (4 bytes) escape to "\F0\9F\98\80" but still represent 4 bytes.
                    let str_len = s.len() + 1; // +1 for null terminator

                    // Emit global to string_globals area
                    let global_decl = format!(
                        "{} = private unnamed_addr constant [{} x i8] c\"{}\\00\"\n",
                        str_global, str_len, escaped
                    );
                    self.string_globals.push_str(&global_decl);

                    // Remember this string for deduplication in future occurrences
                    self.string_constants.insert(s.clone(), str_global.clone());
                    str_global
                };

                let str_len = s.len() + 1; // +1 for null terminator

                // Allocate temps in the order they'll be used in the IR
                let ptr_temp = self.fresh_temp();
                let result = self.fresh_temp();
                let dbg = self.dbg_annotation(loc);

                writeln!(
                    &mut self.output,
                    "  %{} = getelementptr inbounds [{} x i8], ptr {}, i32 0, i32 0{}",
                    ptr_temp, str_len, str_global, dbg
                )
                .map_err(|e| CodegenError::InternalError(e.to_string()))?;
                writeln!(
                    &mut self.output,
                    "  %{} = call ptr @push_string(ptr %{}, ptr %{}){}",
                    result, stack, ptr_temp, dbg
                )
                .map_err(|e| CodegenError::InternalError(e.to_string()))?;

                Ok(result)
            }

            Expr::WordCall(name, loc) => {
                // Check if this is a variant constructor
                if let Some(&tag) = self.variant_tags.get(name) {
                    // This is a variant constructor - emit push_variant call
                    let field_count = self.variant_field_counts.get(name).copied().unwrap_or(0);
                    let dbg = self.dbg_annotation(loc);

                    match field_count {
                        0 => {
                            // Unit variant (no fields) - pass NULL as data
                            let result = self.fresh_temp();
                            writeln!(
                                &mut self.output,
                                "  %{} = call ptr @push_variant(ptr %{}, i32 {}, ptr null){}",
                                result, stack, tag, dbg
                            )
                            .map_err(|e| CodegenError::InternalError(e.to_string()))?;
                            Ok(result)
                        }
                        1 => {
                            // Single-field variant - we need to allocate a new cell for the field
                            // and store that as the variant's data (the variant owns this cell)

                            // Allocate a new cell to store the field value
                            let field_cell = self.fresh_temp();
                            writeln!(
                                &mut self.output,
                                "  %{} = call ptr @alloc_cell(){}",
                                field_cell, dbg
                            )
                            .map_err(|e| CodegenError::InternalError(e.to_string()))?;

                            // Copy the entire StackCell from top of stack to the new cell
                            // StackCell is 32 bytes: { i32 tag, [4 x i8] padding, [16 x i8] union, ptr next }
                            writeln!(
                                &mut self.output,
                                "  call void @llvm.memcpy.p0.p0.i64(ptr align 8 %{}, ptr align 8 %{}, i64 32, i1 false)",
                                field_cell, stack
                            )
                            .map_err(|e| CodegenError::InternalError(e.to_string()))?;

                            // Clear the 'next' pointer in the copied cell (it's not part of a stack)
                            let next_ptr = self.fresh_temp();
                            writeln!(
                                &mut self.output,
                                "  %{} = getelementptr inbounds {{ i32, [4 x i8], [16 x i8], ptr }}, ptr %{}, i32 0, i32 3",
                                next_ptr, field_cell
                            )
                            .map_err(|e| CodegenError::InternalError(e.to_string()))?;

                            writeln!(&mut self.output, "  store ptr null, ptr %{}", next_ptr)
                                .map_err(|e| CodegenError::InternalError(e.to_string()))?;

                            // Get rest of stack (pop the field)
                            let rest_ptr = self.fresh_temp();
                            writeln!(
                                &mut self.output,
                                "  %{} = getelementptr inbounds {{ i32, [4 x i8], [16 x i8], ptr }}, ptr %{}, i32 0, i32 3",
                                rest_ptr, stack
                            )
                            .map_err(|e| CodegenError::InternalError(e.to_string()))?;

                            let rest = self.fresh_temp();
                            writeln!(
                                &mut self.output,
                                "  %{} = load ptr, ptr %{}",
                                rest, rest_ptr
                            )
                            .map_err(|e| CodegenError::InternalError(e.to_string()))?;

                            // Push variant with the allocated cell as data
                            let result = self.fresh_temp();
                            writeln!(
                                &mut self.output,
                                "  %{} = call ptr @push_variant(ptr %{}, i32 {}, ptr %{}){}",
                                result, rest, tag, field_cell, dbg
                            )
                            .map_err(|e| CodegenError::InternalError(e.to_string()))?;
                            Ok(result)
                        }
                        _ => {
                            // Multi-field variants (2+ fields)
                            // Strategy: Chain the fields as a linked list
                            // For Cons(head, tail): stack has [tail, head]
                            // 1. Pop and allocate each field in reverse order
                            // 2. Link them together: field1.next = field2.next = ... = null
                            // 3. Create variant with first field as data

                            let mut field_cells = Vec::new();
                            let mut current_stack = stack.to_string();

                            // Pop and allocate each field
                            for _i in 0..field_count {
                                let field_cell = self.fresh_temp();
                                let dbg = self.dbg_annotation(loc);
                                writeln!(
                                    &mut self.output,
                                    "  %{} = call ptr @alloc_cell(){}",
                                    field_cell, dbg
                                )
                                .map_err(|e| CodegenError::InternalError(e.to_string()))?;

                                // Copy StackCell from top of stack to new cell
                                writeln!(
                                    &mut self.output,
                                    "  call void @llvm.memcpy.p0.p0.i64(ptr align 8 %{}, ptr align 8 %{}, i64 32, i1 false)",
                                    field_cell, current_stack
                                )
                                .map_err(|e| CodegenError::InternalError(e.to_string()))?;

                                field_cells.push(field_cell);

                                // Get rest of stack (pop this field)
                                let rest_ptr = self.fresh_temp();
                                writeln!(
                                    &mut self.output,
                                    "  %{} = getelementptr inbounds {{ i32, [4 x i8], [16 x i8], ptr }}, ptr %{}, i32 0, i32 3",
                                    rest_ptr, current_stack
                                )
                                .map_err(|e| CodegenError::InternalError(e.to_string()))?;

                                let rest = self.fresh_temp();
                                writeln!(
                                    &mut self.output,
                                    "  %{} = load ptr, ptr %{}",
                                    rest, rest_ptr
                                )
                                .map_err(|e| CodegenError::InternalError(e.to_string()))?;

                                current_stack = rest.to_string();
                            }

                            // Link fields together: field[0].next = field[1], field[1].next = field[2], etc.
                            // Last field gets null
                            for i in 0..field_count {
                                let next_ptr = self.fresh_temp();
                                writeln!(
                                    &mut self.output,
                                    "  %{} = getelementptr inbounds {{ i32, [4 x i8], [16 x i8], ptr }}, ptr %{}, i32 0, i32 3",
                                    next_ptr, field_cells[i]
                                )
                                .map_err(|e| CodegenError::InternalError(e.to_string()))?;

                                if i + 1 < field_count {
                                    // Link to next field
                                    writeln!(
                                        &mut self.output,
                                        "  store ptr %{}, ptr %{}",
                                        field_cells[i + 1], next_ptr
                                    )
                                    .map_err(|e| CodegenError::InternalError(e.to_string()))?;
                                } else {
                                    // Last field gets null
                                    writeln!(&mut self.output, "  store ptr null, ptr %{}", next_ptr)
                                        .map_err(|e| CodegenError::InternalError(e.to_string()))?;
                                }
                            }

                            // Create variant with first field as data pointer
                            let result = self.fresh_temp();
                            let dbg = self.dbg_annotation(loc);
                            writeln!(
                                &mut self.output,
                                "  %{} = call ptr @push_variant(ptr %{}, i32 {}, ptr %{}){}",
                                result, current_stack, tag, field_cells[0], dbg
                            )
                            .map_err(|e| CodegenError::InternalError(e.to_string()))?;
                            Ok(result)
                        }
                    }
                } else {
                    // Regular word call
                    let result = self.fresh_temp();
                    let dbg = self.dbg_annotation(loc);
                    let func_name = Self::map_operator_to_function(name);
                    writeln!(
                        &mut self.output,
                        "  %{} = call ptr @{}(ptr %{}){}",
                        result, func_name, stack, dbg
                    )
                    .map_err(|e| CodegenError::InternalError(e.to_string()))?;
                    Ok(result)
                }
            }

            Expr::Quotation(exprs, _loc) => {
                // Generate an anonymous function for the quotation
                let quot_name = format!("quot_{}", self.temp_counter);
                let saved_counter = self.temp_counter;
                self.temp_counter += 1;

                // Save current output
                let saved_output = self.output.clone();
                self.output.clear();

                // Generate the quotation function
                writeln!(&mut self.output, "define ptr @{}(ptr %stack) {{", quot_name)
                    .map_err(|e| CodegenError::InternalError(e.to_string()))?;
                writeln!(&mut self.output, "entry:")
                    .map_err(|e| CodegenError::InternalError(e.to_string()))?;

                // Compile the quotation body
                let mut stack_var = "stack".to_string();
                let len = exprs.len();
                for (i, expr) in exprs.iter().enumerate() {
                    let is_tail = i == len - 1;
                    stack_var = self.compile_expr_with_context(expr, &stack_var, is_tail)?;

                    // If last expression is a musttail call, return its result
                    if is_tail && let Expr::WordCall(_, _) = expr {
                        writeln!(&mut self.output, "  ret ptr %{}", stack_var)
                            .map_err(|e| CodegenError::InternalError(e.to_string()))?;
                    }
                }

                // If we didn't return via musttail, return normally
                if len == 0 || !matches!(exprs.last(), Some(Expr::WordCall(_, _))) {
                    writeln!(&mut self.output, "  ret ptr %{}", stack_var)
                        .map_err(|e| CodegenError::InternalError(e.to_string()))?;
                }

                writeln!(&mut self.output, "}}")
                    .map_err(|e| CodegenError::InternalError(e.to_string()))?;
                writeln!(&mut self.output)
                    .map_err(|e| CodegenError::InternalError(e.to_string()))?;

                // Prepend the quotation function to saved output
                let quot_func = self.output.clone();
                self.output = saved_output + &quot_func;

                // Restore temp counter for current function
                self.temp_counter = saved_counter + 1;

                // Now push the function pointer onto the stack
                let result = self.fresh_temp();
                writeln!(
                    &mut self.output,
                    "  %{} = call ptr @push_quotation(ptr %{}, ptr @{})",
                    result, stack, quot_name
                )
                .map_err(|e| CodegenError::InternalError(e.to_string()))?;
                Ok(result)
            }

            Expr::Match { branches, loc: _ } => {
                // Pattern matching on variants
                //
                // Ownership semantics:
                // - The variant cell is consumed (popped from stack)
                // - For unit variants (None): rest of stack becomes initial stack for branch
                // - For single-field variants (Some(T)): field data is unwrapped onto stack
                //   by linking the data cell to rest of stack
                //
                // Strategy: extract variant tag, switch on tag, each case executes branch body

                if branches.is_empty() {
                    return Err(CodegenError::InternalError(
                        "Empty match expression".to_string(),
                    ));
                }

                // Generate labels for each branch and merge point
                let match_id = self.temp_counter;
                let merge_label = format!("match_merge_{}", match_id);
                let default_label = format!("match_default_{}", match_id);

                // Extract variant tag from stack top
                // StackCell layout: { i32 tag, [4 x i8] padding, [16 x i8] union, ptr next }
                // Variant is stored in union as: { i32 variant_tag, ptr variant_data }
                // So variant_tag is at union offset 0 (field 2, index 0-3)

                // Get pointer to variant tag within the union
                let variant_tag_ptr = self.fresh_temp();
                writeln!(
                    &mut self.output,
                    "  %{} = getelementptr inbounds {{ i32, [4 x i8], [16 x i8], ptr }}, ptr %{}, i32 0, i32 2, i32 0",
                    variant_tag_ptr, stack
                )
                .map_err(|e| CodegenError::InternalError(e.to_string()))?;

                // Load variant tag as i32 (first 4 bytes of union)
                let variant_tag = self.fresh_temp();
                writeln!(
                    &mut self.output,
                    "  %{} = load i32, ptr %{}",
                    variant_tag, variant_tag_ptr
                )
                .map_err(|e| CodegenError::InternalError(e.to_string()))?;

                // Get rest of stack (next pointer at field index 3)
                let rest_ptr = self.fresh_temp();
                writeln!(
                    &mut self.output,
                    "  %{} = getelementptr inbounds {{ i32, [4 x i8], [16 x i8], ptr }}, ptr %{}, i32 0, i32 3",
                    rest_ptr, stack
                )
                .map_err(|e| CodegenError::InternalError(e.to_string()))?;

                let rest_var = self.fresh_temp();
                writeln!(
                    &mut self.output,
                    "  %{} = load ptr, ptr %{}",
                    rest_var, rest_ptr
                )
                .map_err(|e| CodegenError::InternalError(e.to_string()))?;

                // Extract variant data pointer (for single-field variants)
                // Variant data is at union offset 8 (after the 4-byte tag + 4-byte padding)
                // We need this to unwrap the variant in branches
                let variant_data_ptr = self.fresh_temp();
                writeln!(
                    &mut self.output,
                    "  %{} = getelementptr inbounds {{ i32, [4 x i8], [16 x i8], ptr }}, ptr %{}, i32 0, i32 2, i32 8",
                    variant_data_ptr, stack
                )
                .map_err(|e| CodegenError::InternalError(e.to_string()))?;

                let variant_data = self.fresh_temp();
                writeln!(
                    &mut self.output,
                    "  %{} = load ptr, ptr %{}",
                    variant_data, variant_data_ptr
                )
                .map_err(|e| CodegenError::InternalError(e.to_string()))?;

                // Generate switch statement
                write!(
                    &mut self.output,
                    "  switch i32 %{}, label %{} [",
                    variant_tag, default_label
                )
                .map_err(|e| CodegenError::InternalError(e.to_string()))?;

                // Add switch cases for each branch
                for (idx, branch) in branches.iter().enumerate() {
                    let Pattern::Variant { name } = &branch.pattern;
                    // Look up variant tag from type environment
                    let tag_value = self.variant_tags.get(name).copied().ok_or_else(|| {
                        CodegenError::InternalError(format!("Unknown variant: {}", name))
                    })?;
                    let case_label = format!("match_case_{}_{}", match_id, idx);
                    writeln!(
                        &mut self.output,
                        "\n    i32 {}, label %{}",
                        tag_value, case_label
                    )
                    .map_err(|e| CodegenError::InternalError(e.to_string()))?;
                }
                writeln!(&mut self.output, "  ]")
                    .map_err(|e| CodegenError::InternalError(e.to_string()))?;

                // Generate code for each branch
                let mut branch_results = Vec::new();
                let mut branch_predecessors = Vec::new();
                let mut all_branches_musttail = true;

                for (idx, branch) in branches.iter().enumerate() {
                    let case_label = format!("match_case_{}_{}", match_id, idx);

                    writeln!(&mut self.output, "{}:", case_label)
                        .map_err(|e| CodegenError::InternalError(e.to_string()))?;
                    self.current_block = case_label.clone();

                    // Determine the initial stack for this branch
                    // For variants with data, we need to "unwrap" by linking data cell to rest
                    let Pattern::Variant { name } = &branch.pattern;
                    let field_count = self.variant_field_counts.get(name).copied().unwrap_or(0);

                    let initial_stack = if field_count == 0 {
                        // Unit variant (e.g., None) - no data, just use rest
                        rest_var.clone()
                    } else if field_count == 1 {
                        // Single-field variant (e.g., Some(T)) - link data cell to rest
                        // We need to set data->next = rest
                        let data_next_ptr = self.fresh_temp();
                        writeln!(
                            &mut self.output,
                            "  %{} = getelementptr inbounds {{ i32, [4 x i8], [16 x i8], ptr }}, ptr %{}, i32 0, i32 3",
                            data_next_ptr, variant_data
                        )
                        .map_err(|e| CodegenError::InternalError(e.to_string()))?;

                        writeln!(
                            &mut self.output,
                            "  store ptr %{}, ptr %{}",
                            rest_var, data_next_ptr
                        )
                        .map_err(|e| CodegenError::InternalError(e.to_string()))?;

                        variant_data.clone()
                    } else {
                        // Multi-field variant (e.g., Cons(T, List(T)))
                        // The fields are already chained: data -> field1 -> field2 -> ... -> null
                        // We need to find the last field and link it to rest
                        // Then the stack becomes: field1 field2 ... fieldN rest...

                        // Walk the chain to find the last field
                        let mut current_field = variant_data.clone();
                        for i in 0..field_count - 1 {
                            let next_ptr = self.fresh_temp();
                            writeln!(
                                &mut self.output,
                                "  %{} = getelementptr inbounds {{ i32, [4 x i8], [16 x i8], ptr }}, ptr %{}, i32 0, i32 3",
                                next_ptr, current_field
                            )
                            .map_err(|e| CodegenError::InternalError(e.to_string()))?;

                            let next_field = self.fresh_temp();
                            writeln!(
                                &mut self.output,
                                "  %{} = load ptr, ptr %{}",
                                next_field, next_ptr
                            )
                            .map_err(|e| CodegenError::InternalError(e.to_string()))?;

                            // For the last iteration, we'll need to update this field's next pointer
                            if i == field_count - 2 {
                                // This is the last field, link it to rest
                                let last_next_ptr = self.fresh_temp();
                                writeln!(
                                    &mut self.output,
                                    "  %{} = getelementptr inbounds {{ i32, [4 x i8], [16 x i8], ptr }}, ptr %{}, i32 0, i32 3",
                                    last_next_ptr, next_field
                                )
                                .map_err(|e| CodegenError::InternalError(e.to_string()))?;

                                writeln!(
                                    &mut self.output,
                                    "  store ptr %{}, ptr %{}",
                                    rest_var, last_next_ptr
                                )
                                .map_err(|e| CodegenError::InternalError(e.to_string()))?;
                            }

                            current_field = next_field;
                        }

                        // Return the first field as initial stack
                        variant_data.clone()
                    };

                    let (branch_stack, ends_with_musttail) =
                        self.compile_expr_sequence(&branch.body, &initial_stack)?;

                    let predecessor = self.current_block.clone();

                    // Check if this branch terminates (either via musttail or nested match/if)
                    let Pattern::Variant { name: _ } = &branch.pattern;
                    let branch_last_expr = branch.body.last();
                    let branch_terminates = ends_with_musttail
                        || branch_last_expr.is_some_and(|e| self.check_all_paths_returned(e));

                    if branch_terminates {
                        // Branch terminates - emit ret if needed
                        if ends_with_musttail {
                            writeln!(&mut self.output, "  ret ptr %{}", branch_stack)
                                .map_err(|e| CodegenError::InternalError(e.to_string()))?;
                        }
                        // If all paths already returned, no ret needed
                    } else {
                        // Branch doesn't terminate - needs merge point
                        all_branches_musttail = false;
                        writeln!(&mut self.output, "  br label %{}", merge_label)
                            .map_err(|e| CodegenError::InternalError(e.to_string()))?;
                        branch_results.push(branch_stack);
                        branch_predecessors.push(predecessor);
                    }
                }

                // Default case (should never be reached if match is exhaustive)
                writeln!(&mut self.output, "{}:", default_label)
                    .map_err(|e| CodegenError::InternalError(e.to_string()))?;
                writeln!(
                    &mut self.output,
                    "  call void @runtime_error(ptr @.str.match_error)"
                )
                .map_err(|e| CodegenError::InternalError(e.to_string()))?;
                writeln!(&mut self.output, "  unreachable")
                    .map_err(|e| CodegenError::InternalError(e.to_string()))?;

                // Add error string to string globals if not already present
                if !self.string_constants.contains_key("match_error") {
                    let error_msg = "match: non-exhaustive pattern (internal error)";
                    let escaped = Self::escape_llvm_string(error_msg);
                    let str_len = error_msg.len() + 1;
                    let global_decl = format!(
                        "@.str.match_error = private unnamed_addr constant [{} x i8] c\"{}\\00\"\n",
                        str_len, escaped
                    );
                    self.string_globals.push_str(&global_decl);
                    // Mark as added to prevent duplicates
                    self.string_constants
                        .insert("match_error".to_string(), "@.str.match_error".to_string());
                }

                // Merge point
                if !all_branches_musttail {
                    writeln!(&mut self.output, "{}:", merge_label)
                        .map_err(|e| CodegenError::InternalError(e.to_string()))?;
                    self.current_block = merge_label;

                    // Build phi node from branches that didn't return
                    let result = self.fresh_temp();
                    write!(&mut self.output, "  %{} = phi ptr", result)
                        .map_err(|e| CodegenError::InternalError(e.to_string()))?;

                    for (stack_val, pred) in branch_results.iter().zip(branch_predecessors.iter()) {
                        write!(&mut self.output, " [ %{}, %{} ],", stack_val, pred)
                            .map_err(|e| CodegenError::InternalError(e.to_string()))?;
                    }
                    // Remove trailing comma
                    self.output.pop();
                    writeln!(&mut self.output)
                        .map_err(|e| CodegenError::InternalError(e.to_string()))?;

                    Ok(result)
                } else {
                    // All branches ended with musttail and return - no merge point needed
                    // This is actually unreachable code after the match, so return a dummy value
                    Ok(rest_var) // Won't be used since all branches returned
                }
            }

            Expr::If {
                then_branch,
                else_branch,
                loc: _,
            } => {
                // Stack top must be a Bool
                // Strategy: extract bool, branch to then/else, both produce same stack effect

                // Generate unique labels
                let then_label = format!("then_{}", self.temp_counter);
                let else_label = format!("else_{}", self.temp_counter);
                let merge_label = format!("merge_{}", self.temp_counter);
                self.temp_counter += 1;

                // Extract boolean value from stack top
                // StackCell C layout (from runtime/stack.h):
                //   - tag: i32 at offset 0 (4 bytes)
                //   - padding: 4 bytes (for union alignment)
                //   - value union at offset 8 (16 bytes total - largest member is variant struct)
                //   - next: ptr at offset 24 (8 bytes)
                // LLVM struct: { i32, [4 x i8], [16 x i8], ptr } = 32 bytes

                // Get bool value from union at offset 8 (field index 2)
                // Bool is stored as i8 in the first byte of the 16-byte union
                let bool_ptr = self.fresh_temp();
                writeln!(&mut self.output, "  %{} = getelementptr inbounds {{ i32, [4 x i8], [16 x i8], ptr }}, ptr %{}, i32 0, i32 2, i32 0", bool_ptr, stack)
                    .map_err(|e| CodegenError::InternalError(e.to_string()))?;
                let bool_val = self.fresh_temp();
                writeln!(
                    &mut self.output,
                    "  %{} = load i8, ptr %{}",
                    bool_val, bool_ptr
                )
                .map_err(|e| CodegenError::InternalError(e.to_string()))?;

                // Use fresh temp for cond to avoid collisions in nested ifs
                let cond_var = self.fresh_temp();
                writeln!(
                    &mut self.output,
                    "  %{} = trunc i8 %{} to i1",
                    cond_var, bool_val
                )
                .map_err(|e| CodegenError::InternalError(e.to_string()))?;

                // Get rest of stack (next pointer at field index 3)
                let rest_ptr = self.fresh_temp();
                writeln!(&mut self.output, "  %{} = getelementptr inbounds {{ i32, [4 x i8], [16 x i8], ptr }}, ptr %{}, i32 0, i32 3", rest_ptr, stack)
                    .map_err(|e| CodegenError::InternalError(e.to_string()))?;

                // Use fresh temp for rest to avoid collisions in nested ifs
                let rest_var = self.fresh_temp();
                writeln!(
                    &mut self.output,
                    "  %{} = load ptr, ptr %{}",
                    rest_var, rest_ptr
                )
                .map_err(|e| CodegenError::InternalError(e.to_string()))?;

                // Branch using the condition variable
                writeln!(
                    &mut self.output,
                    "  br i1 %{}, label %{}, label %{}",
                    cond_var, then_label, else_label
                )
                .map_err(|e| CodegenError::InternalError(e.to_string()))?;

                // Then branch
                writeln!(&mut self.output, "{}:", then_label)
                    .map_err(|e| CodegenError::InternalError(e.to_string()))?;
                self.current_block = then_label.clone();
                let (then_stack, then_is_musttail) =
                    self.compile_branch_quotation(then_branch, &rest_var)?;

                // Capture the actual block that will branch to merge (after any nested ifs)
                let then_predecessor = self.current_block.clone();

                // If then branch ends with musttail, emit return instead of branch
                if then_is_musttail {
                    writeln!(&mut self.output, "  ret ptr %{}", then_stack)
                        .map_err(|e| CodegenError::InternalError(e.to_string()))?;
                } else {
                    writeln!(&mut self.output, "  br label %{}", merge_label)
                        .map_err(|e| CodegenError::InternalError(e.to_string()))?;
                }

                // Else branch
                writeln!(&mut self.output, "{}:", else_label)
                    .map_err(|e| CodegenError::InternalError(e.to_string()))?;
                self.current_block = else_label.clone();
                let (else_stack, else_is_musttail) =
                    self.compile_branch_quotation(else_branch, &rest_var)?;

                // Capture the actual block that will branch to merge (after any nested ifs)
                let else_predecessor = self.current_block.clone();

                // If else branch ends with musttail, emit return instead of branch
                if else_is_musttail {
                    writeln!(&mut self.output, "  ret ptr %{}", else_stack)
                        .map_err(|e| CodegenError::InternalError(e.to_string()))?;
                } else {
                    writeln!(&mut self.output, "  br label %{}", merge_label)
                        .map_err(|e| CodegenError::InternalError(e.to_string()))?;
                }

                // Merge point - only if at least one branch doesn't end with musttail
                if !then_is_musttail || !else_is_musttail {
                    writeln!(&mut self.output, "{}:", merge_label)
                        .map_err(|e| CodegenError::InternalError(e.to_string()))?;
                    self.current_block = merge_label.clone();

                    // Build phi node based on which branches contribute
                    let result = self.fresh_temp();
                    if !then_is_musttail && !else_is_musttail {
                        // Both branches merge - use actual predecessors
                        writeln!(
                            &mut self.output,
                            "  %{} = phi ptr [ %{}, %{} ], [ %{}, %{} ]",
                            result, then_stack, then_predecessor, else_stack, else_predecessor
                        )
                        .map_err(|e| CodegenError::InternalError(e.to_string()))?;
                    } else if !then_is_musttail {
                        // Only then branch merges (else returned)
                        writeln!(
                            &mut self.output,
                            "  %{} = phi ptr [ %{}, %{} ]",
                            result, then_stack, then_predecessor
                        )
                        .map_err(|e| CodegenError::InternalError(e.to_string()))?;
                    } else {
                        // Only else branch merges (then returned)
                        writeln!(
                            &mut self.output,
                            "  %{} = phi ptr [ %{}, %{} ]",
                            result, else_stack, else_predecessor
                        )
                        .map_err(|e| CodegenError::InternalError(e.to_string()))?;
                    }
                    Ok(result)
                } else {
                    // Both branches end with musttail and return - no merge point needed
                    // This is actually unreachable code after the if, so return a dummy value
                    Ok(then_stack) // Won't be used since both branches returned
                }
            }
        }
    }

    /// Get the generated LLVM IR
    pub fn emit_ir(&self) -> String {
        self.output.clone()
    }
}

impl Default for CodeGen {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::types::{Effect, StackType, Type};

    #[test]
    fn test_codegen_simple() {
        let mut codegen = CodeGen::new();

        // : five ( -- Int ) 5 ;
        let word = WordDef {
            name: "five".to_string(),
            effect: Effect {
                inputs: StackType::Empty,
                outputs: StackType::Empty.push(Type::Int),
            },
            body: vec![Expr::IntLit(5, SourceLoc::unknown())],
            loc: SourceLoc::unknown(),
        };

        let program = Program {
            type_defs: vec![],
            word_defs: vec![word],
        };

        let ir = codegen.compile_program(&program).unwrap();

        assert!(ir.contains("define ptr @five"));
        assert!(ir.contains("call ptr @push_int"));
        assert!(ir.contains("i64 5"));
        assert!(ir.contains("ret ptr"));
    }

    #[test]
    fn test_codegen_word_call() {
        let mut codegen = CodeGen::new();

        // : double ( Int -- Int ) dup + ;
        let word = WordDef {
            name: "double".to_string(),
            effect: Effect {
                inputs: StackType::Empty.push(Type::Int),
                outputs: StackType::Empty.push(Type::Int),
            },
            body: vec![
                Expr::WordCall("dup".to_string(), SourceLoc::unknown()),
                Expr::WordCall("add".to_string(), SourceLoc::unknown()),
            ],
            loc: SourceLoc::unknown(),
        };

        let program = Program {
            type_defs: vec![],
            word_defs: vec![word],
        };

        let ir = codegen.compile_program(&program).unwrap();

        assert!(ir.contains("@double"));
        assert!(ir.contains("call ptr @dup"));
        assert!(ir.contains("call ptr @add"));
    }

    #[test]
    fn test_no_target_triple_in_generated_ir() {
        let mut codegen = CodeGen::new();

        let word = WordDef {
            name: "test".to_string(),
            effect: Effect {
                inputs: StackType::Empty,
                outputs: StackType::Empty,
            },
            body: vec![],
            loc: SourceLoc::unknown(),
        };

        let program = Program {
            type_defs: vec![],
            word_defs: vec![word],
        };

        let ir = codegen.compile_program(&program).unwrap();

        // Verify that target triple is NOT present in the IR
        // We intentionally omit it to let clang use its default and avoid warnings
        assert!(
            !ir.contains("target triple"),
            "IR should not contain target triple declaration"
        );
    }

    #[test]
    fn test_codegen_quotation() {
        let mut codegen = CodeGen::new();

        // : test ( -- Int ) [ 5 10 add ] call_quotation ;
        let word = WordDef {
            name: "test".to_string(),
            effect: Effect {
                inputs: StackType::Empty,
                outputs: StackType::Empty.push(Type::Int),
            },
            body: vec![
                Expr::Quotation(
                    vec![
                        Expr::IntLit(5, SourceLoc::unknown()),
                        Expr::IntLit(10, SourceLoc::unknown()),
                        Expr::WordCall("add".to_string(), SourceLoc::unknown()),
                    ],
                    SourceLoc::unknown(),
                ),
                Expr::WordCall("call_quotation".to_string(), SourceLoc::unknown()),
            ],
            loc: SourceLoc::unknown(),
        };

        let program = Program {
            type_defs: vec![],
            word_defs: vec![word],
        };

        let ir = codegen.compile_program(&program).unwrap();

        // Verify quotation function is generated
        assert!(
            ir.contains("define ptr @quot_"),
            "Should generate quotation function"
        );
        // Verify quotation is pushed
        assert!(
            ir.contains("call ptr @push_quotation"),
            "Should push quotation"
        );
        // Verify quotation contains the body
        assert!(
            ir.contains("call ptr @push_int"),
            "Quotation should push integers"
        );
        assert!(ir.contains("call ptr @add"), "Quotation should call add");
        // Verify call_quotation is called
        assert!(
            ir.contains("call ptr @call_quotation"),
            "Should call call_quotation"
        );
    }
}
