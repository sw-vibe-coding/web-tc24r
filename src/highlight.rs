//! Simple C syntax highlighter for the browser editor.
//!
//! Produces a Vec of colored spans from C source text, used to render
//! highlighted code in a `<pre>` element beneath the editor textarea.

/// A colored fragment of source text.
pub struct Span {
    pub text: String,
    pub color: &'static str,
}

// Catppuccin Mocha palette
const KEYWORD: &str = "#cba6f7"; // mauve
const TYPE_KW: &str = "#89b4fa"; // blue
const NUMBER: &str = "#fab387"; // peach
const STRING: &str = "#a6e3a1"; // green
const COMMENT: &str = "#a6adc8"; // overlay0
const PREPROC: &str = "#f38ba8"; // red
const PLAIN: &str = "#cdd6f4"; // text
const FUNC_CALL: &str = "#f9e2af"; // yellow

const KEYWORDS: &[&str] = &[
    "break", "case", "continue", "default", "do", "else", "enum", "extern", "for", "goto", "if",
    "return", "sizeof", "static", "struct", "switch", "typedef", "union", "volatile", "while",
];

const TYPE_KEYWORDS: &[&str] = &[
    "char", "const", "double", "float", "int", "long", "short", "signed", "unsigned", "void",
];

/// Highlight C source code into colored spans.
pub fn highlight(source: &str) -> Vec<Span> {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut spans = Vec::new();
    let mut i = 0;

    while i < len {
        let ch = bytes[i];

        // Line comments
        if ch == b'/' && i + 1 < len && bytes[i + 1] == b'/' {
            let start = i;
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            spans.push(Span {
                text: source[start..i].to_string(),
                color: COMMENT,
            });
            continue;
        }

        // Block comments
        if ch == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
            let start = i;
            i += 2;
            while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            if i + 1 < len {
                i += 2;
            }
            spans.push(Span {
                text: source[start..i].to_string(),
                color: COMMENT,
            });
            continue;
        }

        // Preprocessor directives
        if ch == b'#' {
            let start = i;
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            spans.push(Span {
                text: source[start..i].to_string(),
                color: PREPROC,
            });
            continue;
        }

        // String literals
        if ch == b'"' {
            let start = i;
            i += 1;
            while i < len && bytes[i] != b'"' {
                if bytes[i] == b'\\' && i + 1 < len {
                    i += 1;
                }
                i += 1;
            }
            if i < len {
                i += 1;
            }
            spans.push(Span {
                text: source[start..i].to_string(),
                color: STRING,
            });
            continue;
        }

        // Char literals
        if ch == b'\'' {
            let start = i;
            i += 1;
            while i < len && bytes[i] != b'\'' {
                if bytes[i] == b'\\' && i + 1 < len {
                    i += 1;
                }
                i += 1;
            }
            if i < len {
                i += 1;
            }
            spans.push(Span {
                text: source[start..i].to_string(),
                color: STRING,
            });
            continue;
        }

        // Numbers (decimal and hex)
        if ch.is_ascii_digit()
            || (ch == b'0' && i + 1 < len && (bytes[i + 1] == b'x' || bytes[i + 1] == b'X'))
        {
            let start = i;
            if ch == b'0' && i + 1 < len && (bytes[i + 1] == b'x' || bytes[i + 1] == b'X') {
                i += 2;
                while i < len && bytes[i].is_ascii_hexdigit() {
                    i += 1;
                }
            } else {
                while i < len && bytes[i].is_ascii_digit() {
                    i += 1;
                }
            }
            spans.push(Span {
                text: source[start..i].to_string(),
                color: NUMBER,
            });
            continue;
        }

        // Identifiers and keywords
        if ch.is_ascii_alphabetic() || ch == b'_' {
            let start = i;
            while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            let word = &source[start..i];

            // Check if this is a function call (identifier followed by '(')
            let mut peek = i;
            while peek < len && bytes[peek].is_ascii_whitespace() && bytes[peek] != b'\n' {
                peek += 1;
            }
            let is_call = peek < len && bytes[peek] == b'(';

            let color = if KEYWORDS.contains(&word) {
                KEYWORD
            } else if TYPE_KEYWORDS.contains(&word) {
                TYPE_KW
            } else if is_call {
                FUNC_CALL
            } else {
                PLAIN
            };

            spans.push(Span {
                text: word.to_string(),
                color,
            });
            continue;
        }

        // Everything else (whitespace, operators, punctuation)
        let start = i;
        i += 1;
        // Batch consecutive plain characters
        while i < len
            && !bytes[i].is_ascii_alphanumeric()
            && bytes[i] != b'_'
            && bytes[i] != b'/'
            && bytes[i] != b'#'
            && bytes[i] != b'"'
            && bytes[i] != b'\''
        {
            i += 1;
        }
        spans.push(Span {
            text: source[start..i].to_string(),
            color: PLAIN,
        });
    }

    spans
}
