//! Compile-and-run pipeline: C source → COR24 assembly → machine code → execution.

use cor24_emulator::{Assembler, EmulatorCore};

pub struct CompileResult {
    pub assembly: String,
    pub output: String,
}

/// Convert a byte offset in source to a 1-based line number.
fn offset_to_line(source: &str, offset: usize) -> usize {
    source[..offset.min(source.len())].bytes().filter(|&b| b == b'\n').count() + 1
}

/// Format a CompileError with line number if span is available.
fn format_error(stage: &str, source: &str, message: &str, span: Option<&tc24r_span::Span>) -> String {
    match span {
        Some(s) => format!("{stage} error (line {}): {message}", offset_to_line(source, s.offset)),
        None => format!("{stage} error: {message}"),
    }
}

/// Compile C source to COR24 assembly, assemble, and run.
pub fn compile_and_run(source: &str) -> CompileResult {
    // Stage 1: Preprocess (no includes in browser)
    let preprocessed = tc24r_preprocess::preprocess(source, None, &[]);

    // Stage 2: Lex
    let tokens = match tc24r_lexer::Lexer::new(&preprocessed).tokenize() {
        Ok(t) => t,
        Err(e) => {
            return CompileResult {
                assembly: String::new(),
                output: format_error("Lexer", &preprocessed, &e.message, e.span.as_ref()),
            };
        }
    };

    // Stage 3: Parse
    let program = match tc24r_parser::parse(tokens) {
        Ok(p) => p,
        Err(e) => {
            return CompileResult {
                assembly: String::new(),
                output: format_error("Parser", &preprocessed, &e.message, e.span.as_ref()),
            };
        }
    };

    // Stage 4: Code generation (C → COR24 assembly)
    let assembly = tc24r_codegen::Codegen::new().generate(&program);

    // Stage 5: Assemble (assembly → machine code)
    let mut assembler = Assembler::new();
    let result = assembler.assemble(&assembly);

    if !result.errors.is_empty() {
        return CompileResult {
            assembly: assembly.clone(),
            output: format!(
                "Assembly generated but assembler errors:\n{}",
                result.errors.join("\n")
            ),
        };
    }

    // Stage 6: Execute
    let mut emu = EmulatorCore::new();
    emu.load_program(0, &result.bytes);
    emu.load_program_extent(result.bytes.len() as u32);
    emu.resume();

    let batch = emu.run_batch(100_000);

    let mut output = String::new();

    // Collect UART output
    let uart = emu.get_uart_output();
    if !uart.is_empty() {
        output.push_str(uart);
    }

    // Show execution status
    let status = match batch.reason {
        cor24_emulator::StopReason::Halted => "Program halted normally",
        cor24_emulator::StopReason::CycleLimit => "Stopped: instruction limit reached (100k)",
        cor24_emulator::StopReason::Breakpoint(addr) => {
            return CompileResult {
                assembly,
                output: format!("{output}\n[Breakpoint at {addr:#06x}]"),
            };
        }
        cor24_emulator::StopReason::InvalidInstruction(op) => {
            return CompileResult {
                assembly,
                output: format!("{output}\n[Invalid instruction: {op:#04x}]"),
            };
        }
        cor24_emulator::StopReason::Paused => "Paused",
    };

    output.push_str(&format!("\n--- {status} ---"));
    output.push_str(&format!(
        "\nInstructions: {} | Registers: r0={:#x} r1={:#x} r2={:#x}",
        emu.instructions_count(),
        emu.get_reg(0),
        emu.get_reg(1),
        emu.get_reg(2),
    ));

    // Show LED state if any LEDs are lit
    let leds = emu.get_led();
    if leds != 0 {
        output.push_str(&format!("\nLEDs: {leds:08b}"));
    }

    CompileResult { assembly, output }
}
