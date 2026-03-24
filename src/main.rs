mod compiler;
mod editor;
mod highlight;

use wasm_bindgen::JsCast;
use web_sys::HtmlSelectElement;
use yew::prelude::*;

use editor::Editor;

const DEFAULT_SOURCE: &str = r#"// COR24 C — UART hello + LED
void putc(int c) {
    *(char *)0xFF0100 = c;
}

void led_on() {
    *(char *)0xFF0000 = 0;
}

int main() {
    putc(72);   // H
    putc(105);  // i
    putc(33);   // !
    led_on();
    return 42;
}
"#;

const DEMOS: &[(&str, &str)] = &[
    ("demo.c", "counter"),
    ("demo2.c", "char, pointers, casts, MMIO"),
    ("demo3.c", "hex literals, pointer arithmetic, strings"),
    ("demo4.c", "software divide and modulo"),
    ("demo5.c", "arrays"),
    ("demo6.c", "global char, pointer, array patterns"),
    ("demo7.c", "pointer subtraction"),
    ("demo8.c", "preprocessor #define"),
    ("demo9.c", "UART RX interrupt"),
    ("demo11.c", "logical AND/OR short-circuit"),
    ("demo12.c", "do...while loop"),
    ("demo13.c", "break and continue"),
    ("demo14.c", "increment and decrement"),
    ("demo15.c", "ternary operator"),
    ("demo16.c", "character literals"),
    ("demo17.c", "multi-declaration"),
    ("demo18.c", "sizeof operator"),
    ("demo19.c", "static and extern"),
    ("demo20.c", "statement expressions (GCC ext)"),
    ("demo21.c", "compound assignment operators"),
    ("demo22.c", "braceless control flow"),
    ("demo23.c", "enum"),
    ("demo24.c", "typedef"),
    ("demo25.c", "struct"),
    ("demo26.c", "switch/case"),
    ("demo27.c", "function prototypes"),
    ("demo28.c", "union"),
    ("demo29.c", "sizeof with array types"),
    ("demo30.c", "line continuation"),
    ("demo31.c", "tentative definitions"),
    ("demo32.c", "multi-declarator typedef"),
    ("demo33.c", "comma-separated struct/union members"),
    ("demo34.c", "multi-dimensional arrays"),
    ("demo35.c", "struct array members"),
    ("demo36.c", "forward-declared struct tags"),
    ("demo37.c", "anonymous struct/union members"),
    ("demo38.c", "struct brace initializer"),
    ("demo39.c", "printf and long branches"),
    ("demo42.c", "nested struct (linked list)"),
    ("demo43.c", "Lisp-style cons cells"),
    ("demo44.c", "Lisp data types and printer"),
    ("demo45.c", "Lisp eval: (+ 40 2) => 42"),
    ("demo46.c", "unsigned int, shifts, comparisons"),
];

const RAW_BASE: &str =
    "https://raw.githubusercontent.com/sw-vibe-coding/tc24r/main/demos/";

#[function_component(App)]
fn app() -> Html {
    let source = use_state(|| DEFAULT_SOURCE.to_string());
    let result = use_state(|| None::<compiler::CompileResult>);

    let on_source_change = {
        let source = source.clone();
        Callback::from(move |value: String| {
            source.set(value);
        })
    };

    let on_run = {
        let source = source.clone();
        let result = result.clone();
        Callback::from(move |_: MouseEvent| {
            result.set(Some(compiler::compile_and_run(&source)));
        })
    };

    let loading = use_state(|| false);

    let on_demo_select = {
        let source = source.clone();
        let result = result.clone();
        let loading = loading.clone();
        Callback::from(move |e: Event| {
            let Some(select) = e.target().and_then(|t| t.dyn_into::<HtmlSelectElement>().ok()) else {
                return;
            };
            let filename = select.value();
            if filename.is_empty() {
                return;
            }
            // Reset select to placeholder.
            select.set_value("");

            let url = format!("{RAW_BASE}{filename}");
            let source = source.clone();
            let result = result.clone();
            let loading = loading.clone();
            loading.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                match gloo_net::http::Request::get(&url).send().await {
                    Ok(resp) if resp.ok() => {
                        if let Ok(text) = resp.text().await {
                            source.set(text);
                            result.set(None);
                        }
                    }
                    _ => {
                        source.set(format!("// Failed to fetch {filename}"));
                        result.set(None);
                    }
                }
                loading.set(false);
            });
        })
    };

    // Extract error line for the appropriate panel.
    let c_error_line = result.as_ref().and_then(|r| {
        r.error.as_ref().filter(|e| e.source == compiler::ErrorSource::C).and_then(|e| e.line)
    });
    let asm_error_line = result.as_ref().and_then(|r| {
        r.error.as_ref()
            .filter(|e| e.source == compiler::ErrorSource::Assembler
                     || e.source == compiler::ErrorSource::Runtime)
            .and_then(|e| e.line)
    });

    html! {
        <main style="display:flex; flex-direction:column; height:100vh; padding:16px; gap:12px;">
            <h1 style="font-size:1.4rem; color:#89b4fa;">
                {"web-tc24r"}
                <span style="font-size:0.8rem; color:#6c7086; margin-left:8px;">
                    {"COR24 compiler in your browser"}
                </span>
            </h1>

            <div style="display:flex; flex:1; gap:12px; min-height:0;">
                // C source editor
                <div style="flex:1; display:flex; flex-direction:column; gap:8px;">
                    <label style="font-size:0.85rem; color:#a6adc8;">{"C Source"}</label>
                    <Editor value={AttrValue::from((*source).clone())} on_change={on_source_change}
                            error_line={c_error_line} />
                </div>

                // Generated assembly
                <div style="flex:1; display:flex; flex-direction:column; gap:8px;">
                    <label style="font-size:0.85rem; color:#a6adc8;">{"Listing"}</label>
                    { render_listing(result.as_ref().map(|r| r.listing.as_slice()).unwrap_or(&[]), asm_error_line) }
                </div>

                // Execution output
                <div style="flex:1; display:flex; flex-direction:column; gap:8px;">
                    <label style="font-size:0.85rem; color:#a6adc8;">{"Output"}</label>
                    <div style="flex:1; background:#181825; border:1px solid #313244; \
                                border-radius:6px; padding:12px; font-family:monospace; font-size:14px; \
                                overflow:auto;">
                        { render_output(result.as_ref()) }
                    </div>
                </div>
            </div>

            <div style="display:flex; gap:12px; align-items:center;">
                <button onclick={on_run}
                    style="padding:8px 24px; background:#89b4fa; color:#1e1e2e; \
                           border:none; border-radius:6px; font-size:1rem; font-weight:600; cursor:pointer;">
                    {"Compile & Run"}
                </button>

                <select onchange={on_demo_select}
                    style="padding:6px 12px; background:#313244; color:#cdd6f4; border:1px solid #45475a; \
                           border-radius:6px; font-size:0.85rem; cursor:pointer;">
                    <option value="" selected=true disabled=true>
                        { if *loading { "Loading..." } else { "Load demo..." } }
                    </option>
                    { for DEMOS.iter().map(|(file, label)| html! {
                        <option value={*file}>{format!("{file} — {label}")}</option>
                    }) }
                </select>
            </div>
        </main>
    }
}

fn render_listing(listing: &[cor24_emulator::AssembledLine], error_line: Option<usize>) -> Html {
    use cor24_emulator::AssembledLine;

    if listing.is_empty() {
        return html! {
            <pre style="flex:1; background:#181825; color:#f9e2af; border:1px solid #313244; \
                        border-radius:6px; padding:12px; font-family:monospace; font-size:14px; \
                        overflow:auto; white-space:pre;" />
        };
    }

    fn format_listing_line(line: &AssembledLine) -> String {
        if line.bytes.is_empty() {
            // Label-only or blank line — no address/hex
            format!("{:>22}{}", "", line.source)
        } else {
            let hex: String = line.bytes.iter().map(|b| format!("{b:02x} ")).collect();
            format!("{:06x}  {:<14}{}", line.address, hex.trim_end(), line.source)
        }
    }

    let width = listing.len().to_string().len();

    html! {
        <div style="flex:1; display:flex; background:#181825; border:1px solid #313244; \
                    border-radius:6px; overflow:auto; font-family:monospace; font-size:13px; \
                    line-height:1.5;">
            <pre style="margin:0; padding:12px 8px 12px 0; text-align:right; color:#6c7086; \
                        user-select:none; background:#11111b; border-right:1px solid #313244; \
                        white-space:pre;">
                { for listing.iter().enumerate().map(|(i, _)| {
                    let n = i + 1;
                    let is_err = error_line == Some(n);
                    let style = if is_err { "color:#f38ba8; background:rgba(243,139,168,0.15);" } else { "" };
                    html! { <span {style}>{format!("{:>width$}\n", n)}</span> }
                }) }
            </pre>
            <pre style="margin:0; padding:12px; white-space:pre; flex:1;">
                { for listing.iter().enumerate().map(|(i, line)| {
                    let n = i + 1;
                    let is_err = error_line == Some(n);
                    let bg = if is_err { "background:rgba(243,139,168,0.15);" } else { "" };
                    let formatted = format_listing_line(line);
                    // Color: address in blue, hex in green, source in yellow
                    if line.bytes.is_empty() {
                        html! { <div style={format!("color:#f9e2af;{bg}")}>{formatted}</div> }
                    } else {
                        let addr_end = 6;
                        let hex_start = 8;
                        let hex_end = hex_start + 14;
                        html! {
                            <div style={bg.to_string()}>
                                <span style="color:#6c7086;">{&formatted[..addr_end]}</span>
                                <span style="color:#6c7086;">{&formatted[addr_end..hex_start]}</span>
                                <span style="color:#a6e3a1;">{&formatted[hex_start..hex_end.min(formatted.len())]}</span>
                                if formatted.len() > hex_end {
                                    <span style="color:#f9e2af;">{&formatted[hex_end..]}</span>
                                }
                            </div>
                        }
                    }
                }) }
            </pre>
        </div>
    }
}

fn render_output(result: Option<&compiler::CompileResult>) -> Html {
    let Some(r) = result else {
        return html! { <span style="color:#6c7086;">{"Click Compile & Run to see output"}</span> };
    };

    html! {
        <>
            // Error message (red) with source label
            if let Some(err) = &r.error {
                <div style="margin:0 0 8px;">
                    <div style="color:#f38ba8; font-weight:600; font-size:0.8rem; margin-bottom:2px;">
                        { match err.source {
                            compiler::ErrorSource::C => "C error",
                            compiler::ErrorSource::Assembler => "Assembler error",
                            compiler::ErrorSource::Runtime => "Runtime error",
                        }}
                        if let Some(line) = err.line {
                            {format!(" (line {line})")}
                        }
                    </div>
                    <pre style="color:#f38ba8; margin:0; white-space:pre-wrap;">{&err.message}</pre>
                </div>
            }

            // UART output
            if !r.uart.is_empty() {
                <div style="margin-bottom:8px;">
                    <div style="color:#6c7086; font-size:0.75rem; margin-bottom:4px;">{"UART"}</div>
                    <pre style="color:#a6e3a1; margin:0; background:#11111b; padding:8px; \
                                border-radius:4px; white-space:pre-wrap;">{&r.uart}</pre>
                </div>
            }

            // Status + execution info
            if let Some(status) = &r.status {
                <div style="color:#cdd6f4; font-size:0.8rem; margin-bottom:4px;">
                    {status}
                </div>
            }

            if let Some(instr) = r.instructions {
                <div style="color:#6c7086; font-size:0.75rem; margin-bottom:4px;">
                    {format!("{instr} instructions executed")}
                </div>
            }

            // Registers
            if let Some(regs) = &r.registers {
                <div style="display:flex; gap:12px; margin-bottom:8px;">
                    { for regs.iter().enumerate().map(|(i, &v)| html! {
                        <div style="background:#11111b; padding:4px 8px; border-radius:4px;">
                            <span style="color:#6c7086; font-size:0.7rem;">{format!("r{i}")}</span>
                            <span style="color:#89b4fa; margin-left:4px;">{format!("{v:#x}")}</span>
                        </div>
                    }) }
                </div>
            }

            // LED indicators
            if let Some(leds) = r.leds {
                <div style="display:flex; gap:4px; align-items:center;">
                    <span style="color:#6c7086; font-size:0.7rem; margin-right:4px;">{"LEDs"}</span>
                    { for (0..8).rev().map(|bit| {
                        let on = (leds >> bit) & 1 != 0;
                        let color = if on { "#a6e3a1" } else { "#313244" };
                        html! {
                            <div style={format!("width:12px; height:12px; border-radius:50%; \
                                                  background:{color};")} />
                        }
                    }) }
                </div>
            }
        </>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
