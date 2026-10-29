use std::{
    fs, io,
    path::{Path, PathBuf},
    time::SystemTime,
};

pub struct Entry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<SystemTime>,
}

impl Entry {
    pub fn is_parent(&self) -> bool {
        self.name == ".."
    }
}

pub struct App {
    pub cwd: PathBuf,
    pub entries: Vec<Entry>,
    pub selected: usize,
    /// 화면에 보이는 첫 번째 열의 인덱스
    pub offset: usize,
    /// 마지막으로 그린 화면의 열당 행 수와 열 수
    pub view_rows: usize,
    pub view_cols: usize,
    pub message: Option<String>,
    pub should_quit: bool,
}

impl App {
    pub fn new() -> io::Result<Self> {
        let cwd = std::env::current_dir()?;
        let entries = read_entries(&cwd)?;
        Ok(Self {
            cwd,
            entries,
            selected: 0,
            offset: 0,
            view_rows: 1,
            view_cols: 1,
            message: None,
            should_quit: false,
        })
    }

    /// 선택한 항목이 디렉토리면 그 안으로 이동한다.
    pub fn enter(&mut self) {
        let Some(entry) = self.entries.get(self.selected) else {
            return;
        };
        if !entry.is_dir {
            return;
        }

        let (target, came_from) = if entry.is_parent() {
            let Some(parent) = self.cwd.parent() else {
                return;
            };
            let came_from = self
                .cwd
                .file_name()
                .map(|n| n.to_string_lossy().into_owned());
            (parent.to_path_buf(), came_from)
        } else {
            (self.cwd.join(&entry.name), None)
        };

        let result = read_entries(&target).and_then(|entries| {
            std::env::set_current_dir(&target)?;
            Ok(entries)
        });
        match result {
            Ok(entries) => {
                self.cwd = target;
                self.entries = entries;
                // 상위로 올라갔을 때는 방금 나온 디렉토리에 커서를 둔다.
                self.selected = came_from
                    .and_then(|name| self.entries.iter().position(|e| e.name == name))
                    .unwrap_or(0);
                self.offset = 0;
                self.message = None;
            }
            Err(e) => self.message = Some(format!("{}: {e}", target.display())),
        }
    }

    pub fn move_by(&mut self, delta: isize) {
        if self.entries.is_empty() {
            return;
        }
        let last = self.entries.len() as isize - 1;
        self.selected = (self.selected as isize + delta).clamp(0, last) as usize;
    }

    pub fn move_to_end(&mut self) {
        self.selected = self.entries.len().saturating_sub(1);
    }

    /// 화면 크기를 기록하고, 커서가 보이도록 스크롤 위치를 맞춘다.
    pub fn set_viewport(&mut self, rows: usize, cols: usize) {
        self.view_rows = rows.max(1);
        self.view_cols = cols.max(1);
        let col = self.selected / self.view_rows;
        if col < self.offset {
            self.offset = col;
        } else if col >= self.offset + self.view_cols {
            self.offset = col + 1 - self.view_cols;
        }
    }
}

fn read_entries(dir: &Path) -> io::Result<Vec<Entry>> {
    let mut entries = Vec::new();
    for item in fs::read_dir(dir)?.flatten() {
        let name = item.file_name().to_string_lossy().into_owned();
        // 심볼릭 링크는 가리키는 대상 기준으로 판단하고, 깨진 링크는 링크 자체 정보를 쓴다.
        let meta = fs::metadata(item.path()).or_else(|_| item.metadata());
        let (is_dir, size, modified) = match meta {
            Ok(m) => (m.is_dir(), m.len(), m.modified().ok()),
            Err(_) => (false, 0, None),
        };
        entries.push(Entry {
            name,
            is_dir,
            size,
            modified,
        });
    }

    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    if dir.parent().is_some() {
        entries.insert(
            0,
            Entry {
                name: "..".into(),
                is_dir: true,
                size: 0,
                modified: None,
            },
        );
    }
    Ok(entries)
}
