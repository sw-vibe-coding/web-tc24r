mod compiler;
mod editor;
mod highlight;

use std::cell::RefCell;
use std::rc::Rc;

use cor24_emulator::{AssembledLine, EmulatorCore};
use wasm_bindgen::JsCast;
use web_sys::{HtmlSelectElement, KeyboardEvent};
use yew::prelude::*;

use editor::Editor;

const DEFAULT_SOURCE: &str = r#"// Hello, World! — COR24 C with printf and LED
#include <stdio.h>

int main() {
    printf("Hello, World!\n");
    printf("2 + 2 = %d\n", 2 + 2);

    // Light LED D2 (active-low: write 0 to turn on)
    *(char *)0xFF0000 = 0;

    return 42;
}
"#;

/// Built-in interactive demos (inline source, not fetched from GitHub).
const INTERACTIVE_DEMOS: &[(&str, &str, &str)] = &[
    ("hello", "Hello, World! (printf + LED)", r#"// Hello, World! — COR24 C with printf and LED
#include <stdio.h>

int main() {
    printf("Hello, World!\n");
    printf("2 + 2 = %d\n", 2 + 2);

    // Light LED D2 (active-low: write 0 to turn on)
    *(char *)0xFF0000 = 0;

    return 42;
}
"#),

    ("echo", "UART echo (type to see characters)", r#"// UART echo — type in the terminal, characters echo back
// Demonstrates: interrupt-driven UART RX, polling UART TX
// Uses __attribute__((interrupt)) for the ISR

#define UART_DATA   0xFF0100
#define UART_STATUS 0xFF0101
#define INT_ENABLE  0xFF0010

void putc(int c) {
    while (*(char *)UART_STATUS & 0x80) {}
    *(char *)UART_DATA = c;
}

// ISR: called on each UART RX byte
__attribute__((interrupt))
void uart_isr() {
    int c = *(char *)UART_DATA;  // read & acknowledge
    putc(c);                      // echo back
    if (c == 13 || c == 10) {
        putc(62);  // '>'
        putc(32);  // ' '
    }
}

int main() {
    // Set interrupt vector
    asm("la r0,_uart_isr\nmov iv,r0");
    // Enable UART RX interrupt
    *(char *)INT_ENABLE = 1;

    putc(62); // '>'
    putc(32); // ' '

    // Spin forever (ISR handles input)
    while (1) {}
}
"#),

    ("led-switch", "LED follows switch S2", r#"// LED follows switch — press S2 to light LED D2
// Demonstrates: polling switch input, controlling LED output
// Click the S2 button below to toggle!

#define LED_REG  0xFF0000

int main() {
    while (1) {
        int sw = *(char *)LED_REG;
        // Switch is bit 0: 1=released, 0=pressed
        // LED is active-low: write 0=on, 1=off
        // So just write the switch state to LED — pressed=0=LED on
        *(char *)LED_REG = sw & 1;
    }
}
"#),

    ("counter", "Live counter on UART", r#"// Live counter — prints incrementing numbers
// Demonstrates: busy-wait loop, UART output

#include <stdio.h>

void delay() {
    int i = 0;
    while (i < 5000) { i++; }
}

int main() {
    int n = 0;
    while (1) {
        printf("%d\n", n);
        n++;
        delay();
    }
}
"#),

    ("adder", "Interactive adder (type two numbers)", r#"// Interactive adder — type two numbers separated by Enter
// Demonstrates: UART input parsing, printf output

#include <stdio.h>

int getc_poll() {
    while (!(*(char *)0xFF0101 & 0x01)) {}
    return *(char *)0xFF0100;
}

int read_int() {
    int n = 0;
    int started = 0;
    while (1) {
        int c = getc_poll();
        putchar(c);  // echo
        if (c >= 48 && c <= 57) {
            n = n * 10 + (c - 48);
            started = 1;
        } else if (started) {
            return n;
        }
    }
}

int main() {
    while (1) {
        printf("a? ");
        int a = read_int();
        printf("b? ");
        int b = read_int();
        printf("= %d\n", a + b);
    }
}
"#),
];

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
    ("demo40.c", "malloc/free (stdlib.h)"),
    ("demo41.c", "getc, atoi, string.h"),
    ("demo42.c", "nested struct (linked list)"),
    ("demo43.c", "Lisp-style cons cells"),
    ("demo44.c", "Lisp data types and printer"),
    ("demo45.c", "Lisp eval: (+ 40 2) => 42"),
    ("demo46.c", "unsigned int, shifts, comparisons"),
];

const RAW_BASE: &str =
    "https://raw.githubusercontent.com/sw-vibe-coding/tc24r/main/demos/";

const REG_NAMES: [&str; 8] = ["r0", "r1", "r2", "fp", "sp", "z", "iv", "ir"];

#[function_component(App)]
fn app() -> Html {
    let source = use_state(|| DEFAULT_SOURCE.to_string());

    // Compilation state
    let listing = use_state(Vec::<AssembledLine>::new);
    let compile_error = use_state(|| None::<compiler::CompileError>);

    // Emulator (mutable ref, survives re-renders)
    let emu: Rc<RefCell<EmulatorCore>> = use_mut_ref(EmulatorCore::new);

    // Emulator display state (updated each tick)
    let uart_output = use_state(String::new);
    let registers = use_state(|| [0u32; 8]);
    let pc_val = use_state(|| 0u32);
    let cond_flag = use_state(|| false);
    let led_state = use_state(|| 0u8);
    let running = use_state(|| false);
    let halted = use_state(|| false);
    let instr_count = use_state(|| 0u64);
    let status_msg = use_state(|| String::from("Ready"));
    let runtime_error_line = use_state(|| None::<usize>);

    // Switch S2
    let switch_pressed = use_state(|| false);

    // UART input buffer (keyboard → emulator, drained in run loop)
    let uart_input: Rc<RefCell<std::collections::VecDeque<u8>>> =
        use_mut_ref(std::collections::VecDeque::new);

    // Interval handle
    let interval_handle = use_mut_ref(|| None::<gloo_timers::callback::Interval>);

    // Loading demo
    let loading = use_state(|| false);

    // --- Callbacks ---

    let on_source_change = {
        let source = source.clone();
        Callback::from(move |value: String| source.set(value))
    };

    let on_run = {
        let source = source.clone();
        let listing = listing.clone();
        let compile_error = compile_error.clone();
        let emu = emu.clone();
        let uart_input = uart_input.clone();
        let uart_output = uart_output.clone();
        let registers = registers.clone();
        let pc_val = pc_val.clone();
        let cond_flag = cond_flag.clone();
        let led_state = led_state.clone();
        let running = running.clone();
        let halted = halted.clone();
        let instr_count = instr_count.clone();
        let status_msg = status_msg.clone();
        let runtime_error_line = runtime_error_line.clone();
        let interval_handle = interval_handle.clone();
        let switch_pressed = switch_pressed.clone();

        Callback::from(move |_: MouseEvent| {
            // Stop any existing run loop
            *interval_handle.borrow_mut() = None;

            // Compile
            let output = compiler::compile(&source);
            listing.set(output.listing.clone());
            runtime_error_line.set(None);

            if let Some(err) = output.error {
                compile_error.set(Some(err));
                running.set(false);
                halted.set(false);
                status_msg.set("Compile error".into());
                return;
            }
            compile_error.set(None);

            // Reset emulator and load binary
            {
                let mut e = emu.borrow_mut();
                *e = EmulatorCore::new();
                e.load_program(0, &output.bytes);
                e.load_program_extent(output.bytes.len() as u32);
                e.set_button_pressed(*switch_pressed);
                e.resume();
            }

            // Reset display state
            uart_output.set(String::new());
            registers.set([0u32; 8]);
            pc_val.set(0);
            cond_flag.set(false);
            led_state.set(0);
            halted.set(false);
            instr_count.set(0);
            status_msg.set("Running".into());
            running.set(true);

            // Clear input buffer
            uart_input.borrow_mut().clear();

            // Start run loop
            let emu = emu.clone();
            let uart_input = uart_input.clone();
            let uart_output = uart_output.clone();
            let registers = registers.clone();
            let pc_val = pc_val.clone();
            let cond_flag = cond_flag.clone();
            let led_state = led_state.clone();
            let running = running.clone();
            let halted = halted.clone();
            let instr_count = instr_count.clone();
            let status_msg = status_msg.clone();
            let runtime_error_line = runtime_error_line.clone();
            let listing = listing.clone();
            let interval_handle2 = interval_handle.clone();

            let interval = gloo_timers::callback::Interval::new(16, move || {
                let mut e = emu.borrow_mut();

                // Drain keyboard input buffer into UART RX when free
                {
                    let mut buf = uart_input.borrow_mut();
                    if !buf.is_empty()
                        && (e.read_byte(0xFF0101) & 0x01 == 0)
                        && let Some(byte) = buf.pop_front()
                    {
                        e.send_uart_byte(byte);
                    }
                }

                let batch = e.run_batch(10_000);

                // Update display state
                uart_output.set(e.get_uart_output().to_string());
                let mut regs = [0u32; 8];
                for (i, reg) in regs.iter_mut().enumerate() {
                    *reg = e.get_reg(i as u8);
                }
                registers.set(regs);
                pc_val.set(e.pc());
                cond_flag.set(e.condition_flag());
                led_state.set(e.get_led());
                instr_count.set(e.instructions_count());

                let stop = match batch.reason {
                    cor24_emulator::StopReason::Halted => {
                        halted.set(true);
                        status_msg.set("Halted".into());
                        true
                    }
                    cor24_emulator::StopReason::InvalidInstruction(op) => {
                        let pc = e.pc();
                        let line = compiler::pc_to_listing_line(&listing, pc);
                        runtime_error_line.set(line);
                        halted.set(true);
                        status_msg.set(format!("Invalid instruction: {op:#04x} at PC={pc:#06x}"));
                        true
                    }
                    cor24_emulator::StopReason::Paused => {
                        status_msg.set("Paused".into());
                        true
                    }
                    _ => false,
                };

                if stop {
                    running.set(false);
                    *interval_handle2.borrow_mut() = None;
                }
            });

            *interval_handle.borrow_mut() = Some(interval);
        })
    };

    let on_stop = {
        let emu = emu.clone();
        let interval_handle = interval_handle.clone();
        let running = running.clone();
        let status_msg = status_msg.clone();
        Callback::from(move |_: MouseEvent| {
            emu.borrow_mut().pause();
            *interval_handle.borrow_mut() = None;
            running.set(false);
            status_msg.set("Stopped".into());
        })
    };

    let on_key = {
        let uart_input = uart_input.clone();
        Callback::from(move |e: KeyboardEvent| {
            e.prevent_default();
            let key = e.key();
            let byte = if key.len() == 1 {
                key.as_bytes()[0]
            } else if key == "Enter" {
                b'\n'
            } else if key == "Backspace" {
                0x08
            } else {
                return;
            };
            uart_input.borrow_mut().push_back(byte);
        })
    };

    let on_switch_toggle = {
        let switch_pressed = switch_pressed.clone();
        let emu = emu.clone();
        Callback::from(move |_: MouseEvent| {
            let new_val = !*switch_pressed;
            switch_pressed.set(new_val);
            emu.borrow_mut().set_button_pressed(new_val);
        })
    };

    let on_demo_select = {
        let source = source.clone();
        let compile_error = compile_error.clone();
        let listing = listing.clone();
        let interval_handle = interval_handle.clone();
        let running = running.clone();
        let status_msg = status_msg.clone();
        let loading = loading.clone();
        Callback::from(move |e: Event| {
            let Some(select) = e.target().and_then(|t| t.dyn_into::<HtmlSelectElement>().ok()) else {
                return;
            };
            let value = select.value();
            if value.is_empty() { return; }
            select.set_value("");

            // Stop any running emulator
            *interval_handle.borrow_mut() = None;
            running.set(false);

            // Check interactive demos first (inline source)
            if let Some((_, _, src)) = INTERACTIVE_DEMOS.iter().find(|(id, _, _)| *id == value) {
                source.set(src.to_string());
                compile_error.set(None);
                listing.set(Vec::new());
                status_msg.set("Ready".into());
                return;
            }

            // Fetch from GitHub
            let url = format!("{RAW_BASE}{value}");
            let source = source.clone();
            let compile_error = compile_error.clone();
            let listing = listing.clone();
            let status_msg = status_msg.clone();
            let loading = loading.clone();
            loading.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                match gloo_net::http::Request::get(&url).send().await {
                    Ok(resp) if resp.ok() => {
                        if let Ok(text) = resp.text().await {
                            source.set(text);
                            compile_error.set(None);
                            listing.set(Vec::new());
                            status_msg.set("Ready".into());
                        }
                    }
                    _ => {
                        source.set(format!("// Failed to fetch {value}"));
                    }
                }
                loading.set(false);
            });
        })
    };

    // --- Error lines for highlighting ---
    let c_error_line = compile_error.as_ref()
        .filter(|e| e.source == compiler::ErrorSource::C)
        .and_then(|e| e.line);
    let asm_error_line = compile_error.as_ref()
        .filter(|e| e.source == compiler::ErrorSource::Assembler)
        .and_then(|e| e.line)
        .or(*runtime_error_line);

    // --- Render ---
    html! {
        <main style="display:flex; flex-direction:column; height:100vh; padding:16px; gap:12px;">
            // GitHub corner
            <a href="https://github.com/sw-vibe-coding/web-tc24r" aria-label="View source on GitHub"
                target="_blank" style="position:absolute; top:0; right:0; z-index:100;">
                <svg width="80" height="80" viewBox="0 0 250 250"
                    style="fill:#89b4fa; color:#1e1e2e;" aria-hidden="true">
                    <path d="M0,0 L115,115 L130,115 L142,142 L250,250 L250,0 Z" />
                    <path d="M128.3,109.0 C113.8,99.7 119.0,89.6 119.0,89.6 C122.0,82.7 120.5,78.6 \
                        120.5,78.6 C119.2,72.0 123.4,76.3 123.4,76.3 C127.3,80.9 125.5,87.3 125.5,87.3 \
                        C122.9,97.6 130.6,101.9 134.4,103.2" fill="currentColor"
                        style="transform-origin:130px 106px;" />
                    <path d="M115.0,115.0 C114.9,115.1 118.7,116.5 119.8,115.4 L133.7,101.6 C136.9,99.2 \
                        139.9,98.4 142.2,98.6 C133.8,88.0 127.5,74.4 143.8,58.0 C148.5,53.4 154.0,51.2 \
                        159.7,51.0 C160.3,49.4 163.2,43.6 171.4,40.1 C171.4,40.1 176.1,42.5 178.8,56.2 \
                        C183.1,58.6 187.2,61.8 190.9,65.4 C194.5,69.0 197.7,73.2 200.1,77.6 C213.8,80.2 \
                        216.3,84.9 216.3,84.9 C212.7,93.1 206.9,96.0 205.4,96.6 C205.1,102.4 203.0,107.8 \
                        198.3,112.5 C181.9,128.9 168.3,122.5 157.7,114.1 C157.9,116.9 156.7,120.9 \
                        152.7,124.9 L141.0,136.5 C139.8,137.7 141.6,141.9 141.8,141.8 Z"
                        fill="currentColor" />
                </svg>
            </a>

            <h1 style="font-size:1.4rem; color:#89b4fa;">
                {"web-tc24r"}
                <span style="font-size:0.8rem; color:#bac2de; margin-left:8px;">
                    {"COR24 compiler in your browser"}
                </span>
            </h1>

            <div style="display:flex; flex:1; gap:12px; min-height:0;">
                // C source editor
                <div style="flex:1; min-width:0; display:flex; flex-direction:column; gap:8px;">
                    <label style="font-size:0.9rem; color:#cdd6f4; font-weight:600;">{"C Source"}</label>
                    <Editor value={AttrValue::from((*source).clone())} on_change={on_source_change}
                            error_line={c_error_line} />
                </div>

                // Listing
                <div style="flex:1; min-width:0; display:flex; flex-direction:column; gap:8px;">
                    <label style="font-size:0.9rem; color:#cdd6f4; font-weight:600;">{"Listing"}</label>
                    { render_listing(&listing, asm_error_line) }
                </div>

                // Emulator panel
                <div style="flex:1; min-width:0; display:flex; flex-direction:column; gap:8px;">
                    <label style="font-size:0.9rem; color:#cdd6f4; font-weight:600;">{"Emulator"}</label>
                    <div style="flex:1; display:flex; flex-direction:column; gap:8px; \
                                background:#181825; border:1px solid #313244; border-radius:6px; \
                                padding:12px; overflow:auto;">

                        // Compile error
                        if let Some(err) = compile_error.as_ref() {
                            <div style="margin-bottom:8px;">
                                <div style="color:#f38ba8; font-weight:600; font-size:0.8rem; margin-bottom:2px;">
                                    { match err.source {
                                        compiler::ErrorSource::C => "C error".to_string(),
                                        compiler::ErrorSource::Header => {
                                            format!("Header error ({})",
                                                err.header.unwrap_or("unknown"))
                                        }
                                        compiler::ErrorSource::Assembler => "Assembler error".to_string(),
                                    }}
                                    if let Some(line) = err.line {
                                        {format!(" line {line}")}
                                    }
                                </div>
                                <pre style="color:#f38ba8; margin:0; white-space:pre-wrap; font-size:0.8rem;">
                                    {&err.message}
                                </pre>
                            </div>
                        }

                        // UART terminal (focusable for keyboard input)
                        <div style="flex:1; min-height:80px;">
                            <div style="color:#bac2de; font-size:0.8rem; margin-bottom:2px;">
                                {"UART"}
                                if *running {
                                    <span style="color:#a6adc8;">{" (type here for input)"}</span>
                                }
                            </div>
                            <div onkeydown={on_key} tabindex="0"
                                style="background:#11111b; color:#a6e3a1; padding:8px; border-radius:4px; \
                                       font-family:monospace; font-size:13px; white-space:pre-wrap; \
                                       min-height:40px; max-height:200px; overflow:auto; \
                                       outline:none; cursor:text; \
                                       border:1px solid transparent;">
                                { if uart_output.is_empty() && !*running && !*halted {
                                    html! { <span style="color:#a6adc8;">{"(no output)"}</span> }
                                } else {
                                    html! { {&*uart_output} }
                                }}
                            </div>
                        </div>

                        // Registers
                        <div>
                            <div style="color:#bac2de; font-size:0.8rem; margin-bottom:4px;">{"Registers"}</div>
                            <div style="display:grid; grid-template-columns:repeat(3,1fr); gap:4px; \
                                        font-family:monospace; font-size:12px;">
                                { for (0..8).map(|i| {
                                    html! {
                                        <div style="background:#11111b; padding:2px 6px; border-radius:3px; \
                                                    display:flex; justify-content:space-between;">
                                            <span style="color:#bac2de;">{REG_NAMES[i]}</span>
                                            <span style="color:#89b4fa;">{format!("{:06x}", registers[i])}</span>
                                        </div>
                                    }
                                }) }
                                <div style="background:#11111b; padding:2px 6px; border-radius:3px; \
                                            display:flex; justify-content:space-between;">
                                    <span style="color:#bac2de;">{"pc"}</span>
                                    <span style="color:#cba6f7;">{format!("{:06x}", *pc_val)}</span>
                                </div>
                                <div style="background:#11111b; padding:2px 6px; border-radius:3px; \
                                            display:flex; justify-content:space-between;">
                                    <span style="color:#bac2de;">{"c"}</span>
                                    <span style="color:#f9e2af;">{ if *cond_flag { "1" } else { "0" } }</span>
                                </div>
                            </div>
                        </div>

                        // Hardware I/O: LED + Switch
                        <div style="display:flex; gap:16px; align-items:center;">
                            // LED D2
                            <div style="display:flex; align-items:center; gap:6px;">
                                <span style="color:#bac2de; font-size:0.8rem;">{"LED D2"}</span>
                                <div style={format!("width:14px; height:14px; border-radius:50%; \
                                    background:{}; border:1px solid #585b70;",
                                    if *led_state & 1 == 0 { "#a6e3a1" } else { "#313244" }
                                )} />
                            </div>
                            // Switch S2
                            <div style="display:flex; align-items:center; gap:6px;">
                                <span style="color:#bac2de; font-size:0.8rem;">{"S2"}</span>
                                <button onclick={on_switch_toggle}
                                    style={format!("padding:2px 10px; border-radius:4px; font-size:0.8rem; \
                                        cursor:pointer; border:1px solid #585b70; \
                                        background:{}; color:{};",
                                        if *switch_pressed { "#a6e3a1" } else { "#313244" },
                                        if *switch_pressed { "#1e1e2e" } else { "#9399b2" },
                                    )}>
                                    { if *switch_pressed { "ON" } else { "OFF" } }
                                </button>
                            </div>
                        </div>

                        // Status bar
                        <div style="display:flex; justify-content:space-between; align-items:center; \
                                    font-size:0.8rem; color:#bac2de; border-top:1px solid #313244; \
                                    padding-top:6px;">
                            <span>{&*status_msg}</span>
                            <span>{format!("{} instructions", *instr_count)}</span>
                        </div>
                    </div>
                </div>
            </div>

            // Button bar
            <div style="display:flex; gap:12px; align-items:center;">
                <button onclick={on_run}
                    style="padding:8px 24px; background:#89b4fa; color:#1e1e2e; \
                           border:none; border-radius:6px; font-size:1rem; font-weight:600; cursor:pointer;">
                    {"Compile & Run"}
                </button>

                if *running {
                    <button onclick={on_stop}
                        style="padding:8px 24px; background:#f38ba8; color:#1e1e2e; \
                               border:none; border-radius:6px; font-size:1rem; font-weight:600; cursor:pointer;">
                        {"Stop"}
                    </button>
                }

                <select onchange={on_demo_select}
                    style="padding:6px 12px; background:#313244; color:#cdd6f4; border:1px solid #585b70; \
                           border-radius:6px; font-size:0.85rem; cursor:pointer;">
                    <option value="" selected=true disabled=true>
                        { if *loading { "Loading..." } else { "Load demo..." } }
                    </option>
                    <optgroup label="Interactive">
                        { for INTERACTIVE_DEMOS.iter().map(|(id, label, _)| html! {
                            <option value={*id}>{*label}</option>
                        }) }
                    </optgroup>
                    <optgroup label="tc24r demos">
                        { for DEMOS.iter().map(|(file, label)| html! {
                            <option value={*file}>{format!("{file} — {label}")}</option>
                        }) }
                    </optgroup>
                </select>
            </div>

            // Bundled headers (collapsible)
            <details style="font-size:0.8rem;">
                <summary style="color:#bac2de; cursor:pointer; user-select:none;">
                    {"Bundled headers (stdio.h, stdlib.h, string.h, cor24.h, stdbool.h)"}
                </summary>
                <div style="display:flex; gap:8px; margin-top:8px; max-height:300px; overflow:auto;">
                    { for compiler::HEADERS.iter().map(|(name, content)| html! {
                        <details style="flex:1; min-width:0;">
                            <summary style="color:#89b4fa; cursor:pointer; font-family:monospace; \
                                            font-size:0.85rem; padding:4px 8px; background:#181825; \
                                            border-radius:4px 4px 0 0; border:1px solid #313244;">
                                {*name}
                            </summary>
                            <pre style="margin:0; padding:8px; background:#11111b; color:#cdd6f4; \
                                        border:1px solid #313244; border-top:none; border-radius:0 0 4px 4px; \
                                        font-size:12px; line-height:1.4; white-space:pre-wrap; \
                                        max-height:250px; overflow:auto;">
                                {*content}
                            </pre>
                        </details>
                    }) }
                </div>
            </details>

            // Footer
            <div style="display:flex; gap:8px; align-items:center; flex-wrap:wrap; \
                        font-size:0.85rem; color:#bac2de; padding-top:4px;">
                <span>{"\u{00a9} 2026 Michael A. Wright"}</span>
                <span>{"\u{00b7}"}</span>
                <span>{"MIT License"}</span>
                <span>{"\u{00b7}"}</span>
                <a href="https://makerlisp.com" target="_blank"
                    style="color:#89b4fa; text-decoration:none;">{"COR24-TB"}</a>
                <span>{"\u{00b7}"}</span>
                <span>{env!("BUILD_SHA")}</span>
                <span>{"\u{00b7}"}</span>
                <span>{env!("BUILD_HOST")}</span>
                <span>{"\u{00b7}"}</span>
                <span>{env!("BUILD_TIMESTAMP")}</span>
            </div>
        </main>
    }
}

fn render_listing(listing: &[AssembledLine], error_line: Option<usize>) -> Html {
    if listing.is_empty() {
        return html! {
            <pre style="flex:1; background:#181825; color:#f9e2af; border:1px solid #313244; \
                        border-radius:6px; padding:12px; font-family:monospace; font-size:14px; \
                        overflow:auto; white-space:pre;" />
        };
    }

    fn format_listing_line(line: &AssembledLine) -> String {
        if line.bytes.is_empty() {
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
            <pre style="margin:0; padding:12px 8px 12px 0; text-align:right; color:#bac2de; \
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
                    if line.bytes.is_empty() {
                        html! { <div style={format!("color:#f9e2af;{bg}")}>{formatted}</div> }
                    } else {
                        let addr_end = 6;
                        let hex_start = 8;
                        let hex_end = hex_start + 14;
                        html! {
                            <div style={bg.to_string()}>
                                <span style="color:#bac2de;">{&formatted[..addr_end]}</span>
                                <span style="color:#bac2de;">{&formatted[addr_end..hex_start]}</span>
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

fn main() {
    yew::Renderer::<App>::new().render();
}
