/**
Linker integration - calls clang to produce executables

This module handles:
- Writing .ll files to disk
- Invoking clang with appropriate flags
- Linking with C runtime
*/
use super::{CodegenError, CodegenResult};
use std::fs;
use std::path::Path;
use std::process::Command;

/// Validate a file path to prevent command injection
///
/// Rejects paths that:
/// - Start with '-' (would be interpreted as flags)
/// - Contain '..' (directory traversal)
fn validate_path(path: &str) -> CodegenResult<()> {
    if path.starts_with('-') {
        return Err(CodegenError::LinkerError {
            message: format!("Invalid path '{}': cannot start with '-'", path),
        });
    }

    let path_obj = Path::new(path);
    for component in path_obj.components() {
        if component.as_os_str() == ".." {
            return Err(CodegenError::LinkerError {
                message: format!("Invalid path '{}': cannot contain '..'", path),
            });
        }
    }

    Ok(())
}

/// Link LLVM IR with C runtime to produce executable
///
/// # Arguments
/// * `ir_code` - The LLVM IR as a string
/// * `runtime_lib` - Path to libcem_runtime.a
/// * `output` - Output executable path
///
/// # Example
/// ```no_run
/// use cemc::codegen::link_program;
///
/// let ir = "define ptr @main(ptr %stack) { ... }";
/// link_program(ir, "runtime/libcem_runtime.a", "program").unwrap();
/// ```
pub fn link_program(ir_code: &str, runtime_lib: &str, output: &str) -> CodegenResult<()> {
    // Validate paths to prevent command injection
    validate_path(runtime_lib)?;
    validate_path(output)?;

    // Write IR to temporary .ll file
    let ll_file = format!("{}.ll", output);
    fs::write(&ll_file, ir_code).map_err(|e| CodegenError::LinkerError {
        message: format!("Failed to write {}: {}", ll_file, e),
    })?;

    // Call clang to compile and link
    let status = Command::new("clang")
        .arg(&ll_file)
        .arg(runtime_lib)
        .arg("-o")
        .arg(output)
        .arg("-O2") // Enable optimizations for musttail
        .arg("-Wno-override-module") // Suppress target triple override warning
        .status()
        .map_err(|e| CodegenError::LinkerError {
            message: format!("Failed to execute clang: {}", e),
        })?;

    if !status.success() {
        return Err(CodegenError::LinkerError {
            message: format!("clang exited with status: {}", status),
        });
    }

    // Keep .ll file for inspection but report success
    println!("Generated: {}", ll_file);
    println!("Executable: {}", output);

    Ok(())
}

/// Link program with default runtime location
pub fn link_program_default(ir_code: &str, output: &str) -> CodegenResult<()> {
    link_program(ir_code, "target/release/libcem_runtime.a", output)
}

/// Compile LLVM IR to object file without linking
///
/// This is useful for testing IR generation without needing a complete program with main()
pub fn compile_to_object(ir_code: &str, output: &str) -> CodegenResult<()> {
    // Validate path to prevent command injection
    validate_path(output)?;

    // Write IR to temporary .ll file
    let ll_file = format!("{}.ll", output);
    fs::write(&ll_file, ir_code).map_err(|e| CodegenError::LinkerError {
        message: format!("Failed to write {}: {}", ll_file, e),
    })?;

    // Call clang to compile to object file
    let status = Command::new("clang")
        .arg("-c")
        .arg(&ll_file)
        .arg("-o")
        .arg(format!("{}.o", output))
        .arg("-O2") // Enable optimizations
        .arg("-Wno-override-module") // Suppress target triple override warning
        .status()
        .map_err(|e| CodegenError::LinkerError {
            message: format!("Failed to execute clang: {}", e),
        })?;

    if !status.success() {
        return Err(CodegenError::LinkerError {
            message: format!("clang exited with status: {}", status),
        });
    }

    println!("Generated: {}", ll_file);
    println!("Object file: {}.o", output);

    Ok(())
}

/// Verify that clang is available
pub fn check_clang() -> CodegenResult<String> {
    let output = Command::new("clang")
        .arg("--version")
        .output()
        .map_err(|e| CodegenError::LinkerError {
            message: format!("clang not found. Please install LLVM/clang: {}", e),
        })?;

    let version = String::from_utf8_lossy(&output.stdout);
    Ok(version.lines().next().unwrap_or("unknown").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_clang() {
        let version = check_clang().unwrap();
        assert!(version.contains("clang") || version.contains("LLVM"));
    }
}
