mod compiler;
mod editor;
mod highlight;

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
                    <Editor value={AttrValue::from((*source).clone())} on_change={on_source_change} />
                </div>

                // Generated assembly
                <div style="flex:1; display:flex; flex-direction:column; gap:8px;">
                    <label style="font-size:0.85rem; color:#a6adc8;">{"Generated Assembly"}</label>
                    <pre style="flex:1; background:#181825; color:#f9e2af; border:1px solid #313244; \
                                border-radius:6px; padding:12px; font-family:monospace; font-size:14px; \
                                overflow:auto; white-space:pre-wrap;">
                        { result.as_ref().map(|r| r.assembly.as_str()).unwrap_or("") }
                    </pre>
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

            <button onclick={on_run}
                style="align-self:flex-start; padding:8px 24px; background:#89b4fa; color:#1e1e2e; \
                       border:none; border-radius:6px; font-size:1rem; font-weight:600; cursor:pointer;">
                {"Compile & Run"}
            </button>
        </main>
    }
}

fn render_output(result: Option<&compiler::CompileResult>) -> Html {
    let Some(r) = result else {
        return html! { <span style="color:#6c7086;">{"Click Compile & Run to see output"}</span> };
    };

    html! {
        <>
            // Error message (red)
            if let Some(err) = &r.error {
                <pre style="color:#f38ba8; margin:0 0 8px; white-space:pre-wrap;">{err}</pre>
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
