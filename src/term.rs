//! Terminal colours, width-aware padding, and the static report tables.

use std::io::{self, IsTerminal};

#[derive(Clone, Copy)]
pub struct Style {
    pub on: bool,
}

impl Style {
    pub fn new(on: bool) -> Self {
        Style { on }
    }

    fn paint(&self, code: &str, text: &str) -> String {
        if self.on {
            format!("\x1b[{code}m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }

    pub fn dim(&self, text: &str) -> String {
        self.paint("2", text)
    }
    pub fn bold(&self, text: &str) -> String {
        self.paint("1", text)
    }
    pub fn green(&self, text: &str) -> String {
        self.paint("32", text)
    }
    pub fn yellow(&self, text: &str) -> String {
        self.paint("33", text)
    }
    pub fn red(&self, text: &str) -> String {
        self.paint("31", text)
    }
    pub fn cyan(&self, text: &str) -> String {
        self.paint("36", text)
    }
    pub fn magenta(&self, text: &str) -> String {
        self.paint("35", text)
    }
    pub fn blue(&self, text: &str) -> String {
        self.paint("34", text)
    }
}

/// `--no-color`, `NO_COLOR`, `TERM=dumb` and non-tty stdout all disable colour.
pub fn colour_allowed(no_color_flag: bool) -> bool {
    if no_color_flag {
        return false;
    }
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    if std::env::var("TERM").map(|t| t == "dumb").unwrap_or(false) {
        return false;
    }
    io::stdout().is_terminal()
}

/// Visual width, counting East Asian wide characters as two columns.
pub fn display_width(text: &str) -> usize {
    text.chars().map(char_width).sum()
}

fn char_width(c: char) -> usize {
    let cp = c as u32;
    if cp == 0 {
        return 0;
    }
    // Combining marks and zero-width joiners take no space.
    if (0x0300..=0x036F).contains(&cp)
        || cp == 0x200B
        || cp == 0x200D
        || (0xFE00..=0xFE0F).contains(&cp)
    {
        return 0;
    }
    let wide = (0x1100..=0x115F).contains(&cp)
        || (0x2E80..=0x303E).contains(&cp)
        || (0x3041..=0x33FF).contains(&cp)
        || (0x3400..=0x4DBF).contains(&cp)
        || (0x4E00..=0x9FFF).contains(&cp)
        || (0xA000..=0xA4CF).contains(&cp)
        || (0xAC00..=0xD7A3).contains(&cp)
        || (0xF900..=0xFAFF).contains(&cp)
        || (0xFE30..=0xFE4F).contains(&cp)
        || (0xFF00..=0xFF60).contains(&cp)
        || (0xFFE0..=0xFFE6).contains(&cp)
        || (0x1F300..=0x1F64F).contains(&cp)
        || (0x1F900..=0x1F9FF).contains(&cp)
        || (0x20000..=0x3FFFD).contains(&cp);
    if wide {
        2
    } else {
        1
    }
}

/// Pad on the right to `width` display columns (never truncates).
pub fn pad(text: &str, width: usize) -> String {
    let w = display_width(text);
    if w >= width {
        return text.to_string();
    }
    format!("{text}{}", " ".repeat(width - w))
}

/// Keep the *tail* of a long path, which is where the meaningful part lives:
/// `…/web-shop/node_modules` beats `~/code/…`.
pub fn ellipsis_left(text: &str, width: usize) -> String {
    if display_width(text) <= width {
        return text.to_string();
    }
    if width <= 1 {
        return "…".to_string();
    }
    let budget = width - 1;
    let mut used = 0;
    let mut start = text.len();
    for (index, c) in text.char_indices().rev() {
        let w = char_width(c);
        if used + w > budget {
            break;
        }
        used += w;
        start = index;
    }
    let mut tail = &text[start..];
    if let Some(cut) = tail.find(['/', '\\']) {
        if cut + 1 < tail.len() {
            tail = &tail[cut + 1..];
        }
    }
    format!("…/{tail}")
}

/// A path string as a human wants to read it: `~/` for home, no `./` noise.
pub fn tidy_text(text: &str) -> String {
    let text = tildify(text);
    match text.strip_prefix("./") {
        Some(rest) => rest.to_string(),
        None => text,
    }
}

pub fn tidy_path(path: &std::path::Path) -> String {
    tidy_text(&path.to_string_lossy())
}

/// Replace the home directory prefix with `~` so paths stay readable.
pub fn tildify(path: &str) -> String {
    if let Some(home) = home_dir() {
        let home = home.to_string_lossy().to_string();
        if let Some(rest) = path.strip_prefix(&home) {
            if rest.is_empty() {
                return "~".to_string();
            }
            let trimmed = rest.trim_start_matches(['/', '\\']);
            return format!("~/{trimmed}");
        }
    }
    path.to_string()
}

pub fn home_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widths_handle_cjk() {
        assert_eq!(display_width("abc"), 3);
        assert_eq!(display_width("项目"), 4);
        assert_eq!(pad("ab", 4), "ab  ");
    }

    #[test]
    fn tidy_path_drops_dot_slash() {
        assert_eq!(
            tidy_path(std::path::Path::new("./web/node_modules")),
            "web/node_modules"
        );
    }

    #[test]
    fn left_ellipsis_keeps_the_tail() {
        let long = "~/code/very/deep/tree/web-shop/node_modules";
        let short = ellipsis_left(long, 22);
        assert!(short.starts_with('…'));
        assert!(short.ends_with("node_modules"));
        assert!(display_width(&short) <= 22);
    }
}
