//! Code editor component with syntax highlighting.
//!
//! Uses the overlay technique: a transparent `<textarea>` sits on top of a
//! `<pre>` that shows the highlighted code. The textarea handles input while
//! the pre provides the colors.

use wasm_bindgen::JsCast;
use web_sys::HtmlTextAreaElement;
use yew::prelude::*;

use crate::highlight;

#[derive(Properties, PartialEq)]
pub struct EditorProps {
    pub value: AttrValue,
    pub on_change: Callback<String>,
    /// 1-based line number to highlight as an error.
    #[prop_or_default]
    pub error_line: Option<usize>,
}

#[function_component(Editor)]
pub fn editor(props: &EditorProps) -> Html {
    let pre_ref = use_node_ref();

    let on_input = {
        let on_change = props.on_change.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(target) = e.target()
                && let Ok(textarea) = target.dyn_into::<HtmlTextAreaElement>()
            {
                on_change.emit(textarea.value());
            }
        })
    };

    let line_count = props.value.chars().filter(|&c| c == '\n').count() + 1;
    let gutter_width = format!("{}ch", line_count.to_string().len() + 2);

    let spans = highlight::highlight(&props.value);
    let error_line = props.error_line;

    // Split spans into lines for per-line error highlighting.
    let mut lines: Vec<Vec<(&str, &str)>> = vec![vec![]];
    for span in &spans {
        let color = span.color;
        let mut remaining = span.text.as_str();
        while let Some(nl) = remaining.find('\n') {
            lines.last_mut().unwrap().push((&remaining[..nl], color));
            lines.push(vec![]);
            remaining = &remaining[nl + 1..];
        }
        if !remaining.is_empty() {
            lines.last_mut().unwrap().push((remaining, color));
        }
    }

    let highlighted: Html = lines
        .iter()
        .enumerate()
        .map(|(i, line_spans)| {
            let line_num = i + 1;
            let is_error = error_line == Some(line_num);
            let bg = if is_error { "background:rgba(243,139,168,0.15);" } else { "" };
            html! {
                <div style={format!("min-height:1.5em;{bg}")}>
                    { for line_spans.iter().map(|(text, color)| html! {
                        <span style={format!("color:{color}")}>{*text}</span>
                    }) }
                </div>
            }
        })
        .collect::<Html>();

    let container_style = "\
        position: relative; \
        flex: 1; \
        min-height: 0; \
        border: 1px solid #313244; \
        border-radius: 6px; \
        overflow: hidden; \
        background: #181825; \
        display: flex;";

    let gutter_style = format!(
        "width: {gutter_width}; \
         min-width: {gutter_width}; \
         background: #11111b; \
         color: #a6adc8; \
         font-family: 'SF Mono', 'Fira Code', 'Cascadia Code', monospace; \
         font-size: 14px; \
         line-height: 1.5; \
         padding: 12px 8px 12px 0; \
         text-align: right; \
         user-select: none; \
         overflow: hidden; \
         border-right: 1px solid #313244; \
         box-sizing: border-box;"
    );

    // Shared text styling for both layers
    let text_style = "\
        font-family: 'SF Mono', 'Fira Code', 'Cascadia Code', monospace; \
        font-size: 14px; \
        line-height: 1.5; \
        padding: 12px; \
        tab-size: 4; \
        white-space: pre-wrap; \
        word-wrap: break-word; \
        overflow-wrap: break-word;";

    let edit_area_style = "\
        position: relative; \
        flex: 1; \
        min-width: 0;";

    let pre_style = format!(
        "{text_style} \
         position: absolute; \
         top: 0; left: 0; right: 0; bottom: 0; \
         margin: 0; \
         overflow: auto; \
         pointer-events: none; \
         color: #cdd6f4;"
    );

    let textarea_style = format!(
        "{text_style} \
         position: absolute; \
         top: 0; left: 0; \
         width: 100%; height: 100%; \
         background: transparent; \
         color: transparent; \
         caret-color: #f5e0dc; \
         border: none; \
         outline: none; \
         resize: none; \
         z-index: 1;"
    );

    let gutter_ref = use_node_ref();

    // Sync scroll from textarea to both pre and gutter
    let on_scroll = {
        let pre_ref = pre_ref.clone();
        let gutter_ref = gutter_ref.clone();
        Callback::from(move |e: Event| {
            if let Some(textarea) = e.target().and_then(|t| t.dyn_into::<HtmlTextAreaElement>().ok())
            {
                let scroll_top = textarea.scroll_top();
                if let Some(pre) = pre_ref.cast::<web_sys::HtmlElement>() {
                    pre.set_scroll_top(scroll_top);
                    pre.set_scroll_left(textarea.scroll_left());
                }
                if let Some(gutter) = gutter_ref.cast::<web_sys::HtmlElement>() {
                    gutter.set_scroll_top(scroll_top);
                }
            }
        })
    };

    html! {
        <div style={container_style}>
            <div ref={gutter_ref} style={gutter_style}>
                { for (1..=line_count).map(|n| {
                    let is_error = error_line == Some(n);
                    let style = if is_error {
                        "color:#f38ba8; background:rgba(243,139,168,0.15);"
                    } else {
                        ""
                    };
                    html! { <div {style}>{n}</div> }
                }) }
            </div>
            <div style={edit_area_style}>
                <pre ref={pre_ref} style={pre_style}>
                    <code>{highlighted}</code>
                </pre>
                <textarea
                    value={props.value.clone()}
                    oninput={on_input}
                    onscroll={on_scroll}
                    spellcheck="false"
                    autocomplete="off"
                    autocorrect="off"
                    autocapitalize="off"
                    style={textarea_style}
                />
            </div>
        </div>
    }
}
