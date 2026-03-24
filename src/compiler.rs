//! Compile pipeline: C source → COR24 assembly → machine code.

use cor24_emulator::{AssembledLine, Assembler};

#[derive(Clone, Copy, PartialEq)]
pub enum ErrorSource {
    C,
    Assembler,
}

pub struct CompileError {
    pub message: String,
    pub source: ErrorSource,
    /// 1-based line number in the relevant source (C or assembly).
    pub line: Option<usize>,
}

pub struct CompileOutput {
    pub listing: Vec<AssembledLine>,
    pub bytes: Vec<u8>,
    pub error: Option<CompileError>,
}

/// Convert a byte offset in source to a 1-based line number.
fn offset_to_line(source: &str, offset: usize) -> usize {
    source[..offset.min(source.len())].bytes().filter(|&b| b == b'\n').count() + 1
}

/// Find the 1-based listing line whose address range contains the given PC.
pub fn pc_to_listing_line(listing: &[AssembledLine], pc: u32) -> Option<usize> {
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
pub const HEADERS: &[(&str, &str)] = &[
    ("stdio.h", include_str!("../../tc24r/include/stdio.h")),
    ("stdlib.h", include_str!("../../tc24r/include/stdlib.h")),
    ("string.h", include_str!("../../tc24r/include/string.h")),
    ("cor24.h", include_str!("../../tc24r/include/cor24.h")),
    ("stdbool.h", include_str!("../../tc24r/include/stdbool.h")),
];

/// Expand `#include <...>` and `#include "..."` directives using bundled headers.
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

/// Compile C source to COR24 machine code. Does not execute.
pub fn compile(source: &str) -> CompileOutput {
    let expanded = expand_includes(source);
    let preprocessed = tc24r_preprocess::preprocess(&expanded, None, &[]);

    let c_err = |msg: String, line: Option<usize>| CompileOutput {
        listing: Vec::new(),
        bytes: Vec::new(),
        error: Some(CompileError { message: msg, source: ErrorSource::C, line }),
    };

    let tokens = match tc24r_lexer::Lexer::new(&preprocessed).tokenize() {
        Ok(t) => t,
        Err(e) => {
            let line = e.span.as_ref().map(|s| offset_to_line(&preprocessed, s.offset));
            return c_err(e.message.clone(), line);
        }
    };

    let program = match tc24r_parser::parse(tokens) {
        Ok(p) => p,
        Err(e) => {
            let line = e.span.as_ref().map(|s| offset_to_line(&preprocessed, s.offset));
            return c_err(e.message.clone(), line);
        }
    };

    let assembly = tc24r_codegen::Codegen::new().generate(&program);

    let mut assembler = Assembler::new();
    let result = assembler.assemble(&assembly);

    if !result.errors.is_empty() {
        let line = result.errors.first().and_then(|e| {
            e.strip_prefix("Line ")
                .and_then(|rest| rest.split(':').next())
                .and_then(|n| n.trim().parse::<usize>().ok())
        });
        return CompileOutput {
            listing: result.lines,
            bytes: Vec::new(),
            error: Some(CompileError {
                message: result.errors.join("\n"),
                source: ErrorSource::Assembler,
                line,
            }),
        };
    }

    CompileOutput {
        listing: result.lines,
        bytes: result.bytes,
        error: None,
    }
}
