<p align="center"><img src="https://raw.githubusercontent.com/StrangeQuery/cargo-cardiac/HEAD/assets/logo.png" alt="cargo-cardiac: Ferris, the Rust crab, in front of a CARDIAC" width="320"></p>
<p align="center"><a href="https://lab.strangequery.com/cargo-cardiac/"><code>lab.strangequery.com/cargo-cardiac</code></a></p>
<p align="center"><a href="https://github.com/StrangeQuery/cargo-cardiac"><img src="https://img.shields.io/badge/GitHub-cargo--cardiac-181717?logo=github&amp;logoColor=white" alt="GitHub"></a> <a href="https://discord.gg/VsqKEdBYzJ"><img src="https://img.shields.io/badge/Discord-join-5865F2?logo=discord&amp;logoColor=white" alt="Discord"></a></p>

# cardiac

**You probably want [cargo-cardiac](https://crates.io/crates/cargo-cardiac).** This is its supporting library that a Rust
program uses for CARDIAC calls. You don't install it yourself. You add it to your program, and cargo-cardiac does the
rest:

    cargo install cargo-cardiac cardiaq
    cargo new hello && cd hello
    cargo add cardiac
