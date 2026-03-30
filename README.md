> **Stabilized.** Active development continues at [sw-embed/web-sw-cor24-tinyc](https://github.com/sw-embed/web-sw-cor24-tinyc). Bug fixes here may not yet be reflected in the fork — a commit comparison is pending.

# web-tc24r

Web UI for the [Tiny COR24 in Rust](https://github.com/sw-vibe-coding/tc24r) compiler. Live browser demos using Rust, Yew, and WebAssembly.

Compile, assemble, and run COR24 programs entirely in the browser.

**[Live Demo](https://sw-vibe-coding.github.io/web-tc24r/)**

![web-tc24r screenshot](images/screenshot.png?ts=1774325700787)

## Related

- [tc24r](https://github.com/sw-vibe-coding/tc24r) -- The compiler
- [cor24-rs](https://github.com/sw-embed/cor24-rs) -- COR24 assembler and emulator
- [tml24c](https://github.com/sw-vibe-coding/tml24c) -- Tiny Macro Lisp for COR24

## Development

```bash
./scripts/serve.sh                                    # dev server with hot reload on port 9101
trunk build --release --public-url /web-tc24r/ -d pages  # production build to pages/
```

## Status

Project scaffolding. Development managed by [AgentRail](https://github.com/sw-vibe-coding/agentrail-rs).

## License

MIT
