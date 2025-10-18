use cemc::codegen::{CodeGen, link_program};
use cemc::parser::Parser;
use clap::{CommandFactory, Parser as ClapParser, Subcommand};
use std::fs;
use std::path::Path;
use std::process::Command;

/// Cem2 Compiler - A concatenative language with green threads and linear types
#[derive(ClapParser)]
#[command(name = "cem")]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile a Cem source file to an executable
    Compile {
        /// Input Cem source file
        #[arg(value_name = "INPUT")]
        input: String,

        /// Output executable name (default: input filename without extension)
        #[arg(short, long, value_name = "OUTPUT")]
        output: Option<String>,

        /// Keep intermediate LLVM IR file
        #[arg(long)]
        keep_ir: bool,
    },

    /// Generate shell completions for bash, zsh, fish, or powershell
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Compile {
            input,
            output,
            keep_ir,
        } => compile_command(&input, output.as_deref(), keep_ir),
        Commands::Completions { shell } => {
            generate_completions(shell);
            Ok(())
        }
    }
}

fn compile_command(
    input_file: &str,
    output_name: Option<&str>,
    keep_ir: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Determine output name
    let output_name = output_name.map(String::from).unwrap_or_else(|| {
        // Default: strip .cem extension and use as output name
        Path::new(input_file)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output")
            .to_string()
    });

    // Read source file
    let source = fs::read_to_string(input_file)
        .map_err(|e| format!("Failed to read {}: {}", input_file, e))?;

    // Parse
    println!("Parsing {}...", input_file);
    let mut parser = Parser::new_with_filename(&source, input_file);
    let program = parser.parse().map_err(|e| format!("Parse error: {}", e))?;

    // Build runtime first
    println!("Building runtime...");
    let status = Command::new("just").arg("build-runtime").status()?;

    if !status.success() {
        return Err("Failed to build runtime".into());
    }

    // Generate LLVM IR
    println!("Generating LLVM IR...");
    let mut codegen = CodeGen::new();

    // Find entry point (look for "main" word, or use first word if only one)
    let has_main = program.word_defs.iter().any(|w| w.name == "main");
    let entry_word = if has_main {
        Some("main")
    } else if program.word_defs.len() == 1 {
        println!(
            "Note: Using '{}' as entry point (no 'main' word found)",
            program.word_defs[0].name
        );
        Some(program.word_defs[0].name.as_str())
    } else {
        eprintln!("Error: No 'main' word found and multiple words defined");
        eprintln!("Either define a 'main' word or compile a file with only one word");
        std::process::exit(1);
    };

    let ir = codegen.compile_program_with_main(&program, entry_word)?;

    // Write IR to file
    let ir_file = format!("{}.ll", output_name);
    fs::write(&ir_file, &ir)?;
    if keep_ir {
        println!("Wrote LLVM IR to {}", ir_file);
    }

    // Link with runtime
    println!("Linking...");
    link_program(&ir, "target/release/libcem_runtime.a", &output_name)?;

    // Clean up IR file unless --keep-ir was specified
    if !keep_ir {
        fs::remove_file(&ir_file).ok();
    }

    println!("\n✅ Successfully compiled to ./{}", output_name);
    println!("Run it with: ./{}", output_name);

    Ok(())
}

fn generate_completions(shell: clap_complete::Shell) {
    let mut cmd = Cli::command();
    let bin_name = cmd.get_name().to_string();
    clap_complete::generate(shell, &mut cmd, bin_name, &mut std::io::stdout());
}
