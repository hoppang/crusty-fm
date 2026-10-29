use chrono::{DateTime, Local};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::app::{App, Entry};

/// 한 열의 최소·최대 폭(구분선 포함). 가장 긴 이름이 들어가도록 이 범위 안에서 열 폭을 정하고,
/// 터미널 폭에 들어가는 만큼 열을 늘린다. 최대 폭이 최소 폭의 두 배 이상이어야 열이 최소 폭보다 좁아지지 않는다.
const COL_MIN_WIDTH: u16 = 40;
const COL_MAX_WIDTH: u16 = 80;
/// 이름 뒤에 붙는 크기와 날짜 부분의 폭: " " + 크기(8) + " " + "yy-mm-dd hh:mm"(14)
const META_WIDTH: usize = 24;

// Mdir III 풍 색상. Mdir 을 재현한 mdir.js 의 기본 색 설정을 참고했다.
const BG: Color = Color::Black;
const FG: Color = Color::Gray;
const LINE: Color = Color::Cyan;
const DIR: Color = Color::LightRed;
const SELECTED: Style = Style::new().fg(Color::Black).bg(Color::Cyan);
const BAR: Style = Style::new().fg(Color::Black).bg(Color::Gray);
const BAR_ACCENT: Style = Style::new().fg(Color::Blue).bg(Color::Gray);
const FUNC: Style = Style::new().fg(Color::White).bg(Color::Cyan);
const FUNC_KEY: Style = Style::new().fg(Color::Yellow).bg(Color::Cyan);

/// 확장자별 파일 색. 앞에서부터 처음 맞는 것을 쓴다.
const EXT_COLORS: &[(Color, &[&str])] = &[
    (Color::Magenta, &["a", "so", "la", "exe", "com", "bat", "dll"]),
    (Color::LightGreen, &["json", "sh", "toml", "yaml", "yml"]),
    (
        Color::Cyan,
        &[
            "txt", "md", "ini", "cfg", "log", "conf", "html", "htm", "xml", "css", "js", "ts",
            "jsx", "tsx", "php", "c", "cpp", "cc", "h", "hh", "rs", "pl", "java", "py", "go",
            "diff", "patch",
        ],
    ),
    (
        Color::LightBlue,
        &[
            "pdf", "hwp", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "odt", "ods", "odp",
            "rtf", "csv",
        ],
    ),
    (
        Color::Green,
        &[
            "bmp", "tga", "pcx", "gif", "jpg", "jpeg", "png", "webp", "ico", "svg", "xpm",
        ],
    ),
    (
        Color::LightMagenta,
        &[
            "tar", "gz", "tgz", "bz2", "xz", "zst", "zip", "rar", "7z", "arj", "lzh", "alz",
            "jar", "deb", "rpm", "iso",
        ],
    ),
    (
        Color::Yellow,
        &[
            "mp2", "mp3", "wav", "ogg", "flac", "voc", "avi", "mpg", "mpeg", "mov", "wmv",
            "mp4", "mkv",
        ],
    ),
];

pub fn draw(frame: &mut Frame, app: &mut App) {
    let [header, body, status, keys] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(3),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    draw_header(frame, header, app);
    draw_files(frame, body, app);
    draw_status(frame, status, app);
    draw_keys(frame, keys);
}

fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    let line = Line::from(vec![
        Span::styled(" crusty-fm ", BAR_ACCENT.add_modifier(Modifier::BOLD)),
        Span::raw(" "),
        Span::raw(app.cwd.display().to_string()),
    ]);
    frame.render_widget(Paragraph::new(line).style(BAR), area);
}

fn draw_files(frame: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::new().fg(LINE))
        .style(Style::new().fg(FG).bg(BG));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = inner.height as usize;
    let cols = column_count(inner.width, &app.entries);
    app.set_viewport(rows, cols);
    if rows == 0 {
        return;
    }

    let col_areas = Layout::horizontal(vec![Constraint::Ratio(1, cols as u32); cols]).split(inner);
    for (i, &col_area) in col_areas.iter().enumerate() {
        let is_last = i + 1 == cols;
        // 열 사이에 세로 구분선을 둔다.
        let col_block = if is_last {
            Block::new()
        } else {
            Block::new()
                .borders(Borders::RIGHT)
                .border_style(Style::new().fg(LINE))
        };
        let text_area = col_block.inner(col_area);
        frame.render_widget(col_block, col_area);

        let start = (app.offset + i) * rows;
        let width = text_area.width as usize;
        let lines: Vec<Line> = app
            .entries
            .iter()
            .enumerate()
            .skip(start)
            .take(rows)
            .map(|(idx, entry)| entry_line(entry, width, idx == app.selected))
            .collect();
        frame.render_widget(Paragraph::new(lines), text_area);
    }
}

/// 가장 긴 이름에 맞춘 열 폭으로 몇 열을 둘지 정한다. 남는 폭은 열들이 나눠 가지되,
/// 한 열이 `COL_MAX_WIDTH`를 넘을 만큼 남으면 열을 하나 더 둔다.
fn column_count(width: u16, entries: &[Entry]) -> usize {
    let longest = entries.iter().map(|e| e.name.width()).max().unwrap_or(0);
    let wanted = (longest + META_WIDTH + 1).min(COL_MAX_WIDTH as usize) as u16;
    let col_width = wanted.max(COL_MIN_WIDTH);
    (width / col_width).max(width.div_ceil(COL_MAX_WIDTH)).max(1) as usize
}

fn entry_line(entry: &Entry, width: usize, selected: bool) -> Line<'static> {
    let name_width = width.saturating_sub(META_WIDTH);
    let size = if entry.is_parent() {
        "<UP-DIR>".to_string()
    } else if entry.is_dir {
        " <DIR>  ".to_string()
    } else {
        format_size(entry.size)
    };
    let date = entry
        .modified
        .map(|t| {
            DateTime::<Local>::from(t)
                .format("%y-%m-%d %H:%M")
                .to_string()
        })
        .unwrap_or_default();
    let text = fit(
        &format!("{} {size:>8} {date}", fit(&entry.name, name_width)),
        width,
    );

    let style = if selected {
        SELECTED
    } else if entry.is_dir {
        Style::new().fg(DIR)
    } else {
        Style::new().fg(file_color(&entry.name))
    };
    Line::styled(text, style)
}

fn file_color(name: &str) -> Color {
    let Some((_, ext)) = name.rsplit_once('.') else {
        return FG;
    };
    let ext = ext.to_ascii_lowercase();
    EXT_COLORS
        .iter()
        .find(|(_, exts)| exts.contains(&ext.as_str()))
        .map_or(FG, |&(color, _)| color)
}

fn draw_status(frame: &mut Frame, area: Rect, app: &App) {
    let line = match &app.message {
        Some(msg) => Line::styled(format!(" {msg}"), Style::new().fg(Color::Red)),
        None => {
            let files = app.entries.iter().filter(|e| !e.is_dir);
            let file_count = files.clone().count();
            let total: u64 = files.map(|e| e.size).sum();
            let dir_count = app
                .entries
                .iter()
                .filter(|e| e.is_dir && !e.is_parent())
                .count();
            Line::from(vec![
                Span::styled(format!(" {file_count}"), BAR_ACCENT),
                Span::raw(" 파일  "),
                Span::styled(dir_count.to_string(), BAR_ACCENT),
                Span::raw(" 디렉토리  "),
                Span::styled(group_digits(total), BAR_ACCENT),
                Span::raw(" 바이트"),
            ])
        }
    };
    frame.render_widget(Paragraph::new(line).style(BAR), area);
}

fn draw_keys(frame: &mut Frame, area: Rect) {
    let mut spans = Vec::new();
    for (k, desc) in [("Enter", "이동"), ("Ctrl-Q", "종료")] {
        spans.push(Span::styled(format!(" {k} "), FUNC_KEY));
        spans.push(Span::styled(format!(" {desc}  "), FUNC));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)).style(FUNC), area);
}

/// 8칸에 맞도록 크기를 표시한다. 너무 크면 단위를 붙인다.
fn format_size(size: u64) -> String {
    if size < 100_000_000 {
        return size.to_string();
    }
    let mut value = size as f64;
    for unit in ["K", "M", "G", "T", "P"] {
        value /= 1024.0;
        if value < 10_000.0 {
            return format!("{value:.1}{unit}");
        }
    }
    format!("{value:.0}E")
}

fn group_digits(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// 문자열을 표시 폭 기준으로 정확히 `width` 칸에 맞춘다.
/// 한글처럼 두 칸을 차지하는 문자를 고려하고, 넘치면 `~`로 자른다.
fn fit(s: &str, width: usize) -> String {
    let chars: Vec<(char, usize)> = s
        .chars()
        .map(|c| if c.is_control() { '?' } else { c })
        .map(|c| (c, c.width().unwrap_or(0)))
        .collect();
    let total: usize = chars.iter().map(|&(_, w)| w).sum();

    let mut out = String::new();
    let mut used = 0;
    if total <= width {
        out.extend(chars.iter().map(|&(c, _)| c));
        used = total;
    } else if width > 0 {
        for &(c, w) in &chars {
            if used + w > width - 1 {
                break;
            }
            out.push(c);
            used += w;
        }
        out.push('~');
        used += 1;
    }
    out.extend(std::iter::repeat_n(' ', width - used));
    out
}
