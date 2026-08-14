//! 全局 flag 的输出（help / version）与消息行样式（仿 progress.rs：
//! symbol + console::style 着色，非 TTY 自动降级纯文本）。

use std::io::Write;

use dialoguer::console::style;

pub(super) fn print_version(out: &mut impl Write) -> i32 {
  let _ = writeln!(out, "vbumpp {}", env!("CARGO_PKG_VERSION"));
  0
}

pub(super) fn print_help(out: &mut impl Write) -> i32 {
  let _ = writeln!(
    out,
    "usage:\n  \
     vbumpp [...files]                bump version and generate changelog\n  \
     vbumpp release <version>         retry platform release from a changelog section\n  \
     vbumpp token <action> [name]     manage tokens (action: set / list / remove), stored encrypted\n  \
     (set/list/remove accept --host <url> for gitlab; remove also accepts --all, --yes, --dry-run)\n\
     \noptions:\n  \
     -o, --output [output]       where CHANGELOG.md is generated / read (default CHANGELOG.md)\n  \
     -r, --recursive             recursively\n  \
     --provider <provider>       release provider (github / gitlab / gitee / gitcode)\n  \
     --dry-run                   preview the bump/release plan without side effects\n  \
     -h, --help                  show help\n  \
     -v, --version               show version"
  );
  0
}

pub(super) fn success_line(out: &mut impl Write, msg: &str) {
  let _ = writeln!(out, "{} {msg}", style("✔").green());
}

pub(super) fn info_line(out: &mut impl Write, msg: &str) {
  let _ = writeln!(out, "{} {msg}", style("ℹ").blue());
}

pub(super) fn warn_line(out: &mut impl Write, msg: &str) {
  let _ = writeln!(out, "{} {msg}", style("⚠").yellow());
}

pub(super) fn error_line(err: &mut impl Write, msg: &str) {
  let _ = writeln!(err, "{} {msg}", style("✖").red());
}
