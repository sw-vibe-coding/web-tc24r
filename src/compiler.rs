//! Compile-and-run pipeline: C source → COR24 assembly → machine code → execution.

use cor24_emulator::{AssembledLine, Assembler, EmulatorCore};

#[derive(Clone, Copy, PartialEq)]
pub enum ErrorSource {
    C,
    Assembler,
    Runtime,
}

pub struct CompileError {
    pub message: String,
    pub source: ErrorSource,
    /// 1-based line number in the relevant source (C or assembly).
    pub line: Option<usize>,
}

pub struct CompileResult {
    pub listing: Vec<AssembledLine>,
    pub uart: String,
    pub error: Option<CompileError>,
    pub status: Option<String>,
    pub instructions: Option<u64>,
    pub registers: Option<[u32; 3]>,
    pub leds: Option<u8>,
}

/// Convert a byte offset in source to a 1-based line number.
fn offset_to_line(source: &str, offset: usize) -> usize {
    source[..offset.min(source.len())].bytes().filter(|&b| b == b'\n').count() + 1
}


/// Find the 1-based listing line whose address range contains the given PC.
fn pc_to_listing_line(listing: &[AssembledLine], pc: u32) -> Option<usize> {
    for (i, line) in listing.iter().enumerate() {
        if !line.bytes.is_empty() {
            let start = line.address;
            let end = start + line.bytes.len() as u32;
            if pc >= start && pc < end {
                return Some(i + 1);
            }
        }
    }
    None
}

/// Bundled tc24r freestanding headers for in-browser #include expansion.
const HEADERS: &[(&str, &str)] = &[
    ("stdio.h", include_str!("../../tc24r/include/stdio.h")),
    ("stdlib.h", include_str!("../../tc24r/include/stdlib.h")),
    ("string.h", include_str!("../../tc24r/include/string.h")),
    ("cor24.h", include_str!("../../tc24r/include/cor24.h")),
    ("stdbool.h", include_str!("../../tc24r/include/stdbool.h")),
];

/// Expand `#include <...>` and `#include "..."` directives using bundled headers.
/// Respects `#pragma once` — each header is included at most once.
fn expand_includes(source: &str) -> String {
    let mut included = std::collections::HashSet::new();
    let mut output = String::with_capacity(source.len() * 2);
    expand_includes_inner(source, &mut included, &mut output);
    output
}

fn expand_includes_inner(source: &str, included: &mut std::collections::HashSet<&'static str>, output: &mut String) {
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(name) = parse_include(trimmed) {
            if let Some((key, content)) = HEADERS.iter().find(|(k, _)| *k == name)
                && !included.contains(key)
            {
                if content.lines().any(|l| l.trim() == "#pragma once") {
                    included.insert(key);
                }
                expand_includes_inner(content, included, output);
            }
            // Skip unrecognized includes silently
        } else {
            output.push_str(line);
            output.push('\n');
        }
    }
}

fn parse_include(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("#include")?.trim();
    if let Some(inner) = rest.strip_prefix('<').and_then(|r| r.strip_suffix('>')) {
        Some(inner.trim())
    } else if let Some(inner) = rest.strip_prefix('"').and_then(|r| r.strip_suffix('"')) {
        Some(inner.trim())
    } else {
        None
    }
}

/// Compile C source to COR24 assembly, assemble, and run.
pub fn compile_and_run(source: &str) -> CompileResult {
    // Stage 1: Expand includes, then preprocess
    let expanded = expand_includes(source);
    let preprocessed = tc24r_preprocess::preprocess(&expanded, None, &[]);

    let c_err = |msg: String, line: Option<usize>| CompileResult {
        listing: Vec::new(),
        uart: String::new(),
        error: Some(CompileError { message: msg, source: ErrorSource::C, line }),
        status: None,
        instructions: None,
        registers: None,
        leds: None,
    };

    // Stage 2: Lex
    let tokens = match tc24r_lexer::Lexer::new(&preprocessed).tokenize() {
        Ok(t) => t,
        Err(e) => {
            let line = e.span.as_ref().map(|s| offset_to_line(&preprocessed, s.offset));
            return c_err(e.message.clone(), line);
        }
    };

    // Stage 3: Parse
    let program = match tc24r_parser::parse(tokens) {
        Ok(p) => p,
        Err(e) => {
            let line = e.span.as_ref().map(|s| offset_to_line(&preprocessed, s.offset));
            return c_err(e.message.clone(), line);
        }
    };

    // Stage 4: Code generation (C → COR24 assembly)
    let assembly = tc24r_codegen::Codegen::new().generate(&program);

    // Stage 5: Assemble (assembly → machine code)
    let mut assembler = Assembler::new();
    let result = assembler.assemble(&assembly);

    if !result.errors.is_empty() {
        // Try to extract a line number from "Line N:" pattern in the first error.
        let line = result.errors.first().and_then(|e| {
            e.strip_prefix("Line ")
                .and_then(|rest| rest.split(':').next())
                .and_then(|n| n.trim().parse::<usize>().ok())
        });
        return CompileResult {
            listing: result.lines,
            uart: String::new(),
            error: Some(CompileError {
                message: result.errors.join("\n"),
                source: ErrorSource::Assembler,
                line,
            }),
            status: None,
            instructions: None,
            registers: None,
            leds: None,
        };
    }

    let listing = result.lines;

    // Stage 6: Execute
    let mut emu = EmulatorCore::new();
    emu.load_program(0, &result.bytes);
    emu.load_program_extent(result.bytes.len() as u32);
    emu.resume();

    let batch = emu.run_batch(1_000_000);

    let uart = emu.get_uart_output().to_string();
    let pc = emu.pc();

    // Map PC to 1-based listing line number.
    let pc_line = pc_to_listing_line(&listing, pc);

    let runtime_err = |msg: String| Some(CompileError {
        message: msg,
        source: ErrorSource::Runtime,
        line: pc_line,
    });

    let (status, error): (Option<String>, Option<CompileError>) = match batch.reason {
        cor24_emulator::StopReason::Halted => (Some("Halted".into()), None),
        cor24_emulator::StopReason::CycleLimit => {
            (None, runtime_err(format!("Instruction limit reached (1M) at PC={pc:#06x}")))
        }
        cor24_emulator::StopReason::Breakpoint(addr) => {
            (Some(format!("Breakpoint at {addr:#06x}")), None)
        }
        cor24_emulator::StopReason::InvalidInstruction(op) => {
            (None, runtime_err(format!("Invalid instruction: {op:#04x} at PC={pc:#06x}")))
        }
        cor24_emulator::StopReason::Paused => (Some("Paused".into()), None),
    };

    let leds = emu.get_led();

    CompileResult {
        listing,
        uart,
        error,
        status,
        instructions: Some(emu.instructions_count()),
        registers: Some([emu.get_reg(0), emu.get_reg(1), emu.get_reg(2)]),
        leds: if leds != 0 { Some(leds) } else { None },
    }
}
