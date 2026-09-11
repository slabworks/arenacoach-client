use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use crate::log_path::player_prev_log_path;

pub const HEADER: &str = "[UnityCrossThreadLogger]";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEntry {
    pub timestamp: Option<String>,
    pub body: String,
}

impl LogEntry {
    pub fn from_chunk(chunk: &str) -> Self {
        let rest = chunk.strip_prefix(HEADER).unwrap_or(chunk);
        let (timestamp, body) = split_header(rest);
        Self { timestamp, body }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailedLogs {
    On,
    Off,
    Unknown,
}

pub fn detect_detailed_logs(text: &str) -> DetailedLogs {
    if text.contains("greToClientEvent") || text.contains("matchGameRoomStateChangedEvent") {
        DetailedLogs::On
    } else if text.contains("DETAILED LOGS: DISABLED") {
        DetailedLogs::Off
    } else {
        DetailedLogs::Unknown
    }
}

pub struct LogFollower {
    path: PathBuf,
    offset: u64,
    incomplete: String,
    file_key: Option<(u64, u64)>,
}

impl LogFollower {
    pub fn tail(path: PathBuf) -> Self {
        let meta = fs::metadata(&path).ok();
        Self {
            path,
            offset: meta.as_ref().map(|item| item.len()).unwrap_or(0),
            incomplete: String::new(),
            file_key: meta.as_ref().map(file_key),
        }
    }

    pub fn from_start(path: PathBuf) -> Self {
        Self {
            path,
            offset: 0,
            incomplete: String::new(),
            file_key: None,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn poll(&mut self) -> std::io::Result<Vec<LogEntry>> {
        if !self.path.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Player.log is missing",
            ));
        }

        let meta = fs::metadata(&self.path)?;
        let key = file_key(&meta);
        let rotated =
            self.file_key.is_some() && (self.file_key != Some(key) || meta.len() < self.offset);
        if rotated {
            let mut unread = self.read_rotated_tail()?;
            self.offset = 0;
            self.incomplete.clear();
            self.file_key = Some(key);
            unread.extend(self.read_from_current()?);
            return Ok(unread);
        }
        self.file_key = Some(key);
        self.read_from_current()
    }

    fn read_rotated_tail(&self) -> std::io::Result<Vec<LogEntry>> {
        let prev = player_prev_log_path(&self.path);
        if !prev.exists() {
            return Ok(Vec::new());
        }
        let mut follower = LogFollower {
            path: prev,
            offset: self.offset,
            incomplete: self.incomplete.clone(),
            file_key: None,
        };
        follower.read_from_current()
    }

    fn read_from_current(&mut self) -> std::io::Result<Vec<LogEntry>> {
        let mut file = File::open(&self.path)?;
        file.seek(SeekFrom::Start(self.offset))?;
        let mut buf = String::new();
        file.read_to_string(&mut buf)?;
        self.offset += buf.len() as u64;
        if buf.is_empty() && self.incomplete.is_empty() {
            return Ok(Vec::new());
        }
        self.incomplete.push_str(&buf);
        Ok(self.drain_complete_entries())
    }

    fn drain_complete_entries(&mut self) -> Vec<LogEntry> {
        let (entries, remainder) = split_entries(&self.incomplete);
        self.incomplete = remainder;
        entries
    }
}

pub fn split_entries(buffer: &str) -> (Vec<LogEntry>, String) {
    let starts: Vec<usize> = buffer.match_indices(HEADER).map(|(idx, _)| idx).collect();
    if starts.is_empty() {
        let trimmed = if buffer.len() > 65_536 {
            {
                let mut start = buffer.len() - 65_536;
                while !buffer.is_char_boundary(start) {
                    start += 1;
                }
                buffer[start..].to_string()
            }
        } else {
            buffer.to_string()
        };
        return (Vec::new(), trimmed);
    }

    let prefix = &buffer[..starts[0]];
    let mut work = String::new();
    if !prefix.is_empty() && prefix.contains('{') {
        work.push_str(prefix);
    }

    let mut entries = Vec::new();
    for window in starts.windows(2) {
        let chunk = &buffer[window[0]..window[1]];
        work.push_str(chunk);
        entries.push(LogEntry::from_chunk(&work));
        work.clear();
    }

    let tail = &buffer[*starts.last().unwrap()..];
    work.push_str(tail);
    if entry_complete(&work) {
        entries.push(LogEntry::from_chunk(&work));
        (entries, String::new())
    } else {
        (entries, work)
    }
}

fn split_header(rest: &str) -> (Option<String>, String) {
    let line_end = rest.find('\n').unwrap_or(rest.len());
    let first_line = rest[..line_end].trim_end_matches('\r');
    if looks_like_timestamp(first_line) {
        let body = rest[line_end..]
            .trim_start_matches(['\r', '\n'])
            .to_string();
        (Some(first_line.trim().to_string()), body)
    } else {
        (None, rest.trim_start_matches(['\r', '\n']).to_string())
    }
}

fn looks_like_timestamp(line: &str) -> bool {
    if line.is_empty()
        || line.starts_with('{')
        || line.starts_with("==>")
        || line.starts_with("<==")
    {
        return false;
    }
    let mut digits = 0;
    let mut slashes = 0;
    for ch in line.chars().take(20) {
        if ch.is_ascii_digit() {
            digits += 1;
        }
        if ch == '/' {
            slashes += 1;
        }
    }
    digits >= 4 && slashes >= 2
}

fn entry_complete(chunk: &str) -> bool {
    let body = chunk.strip_prefix(HEADER).unwrap_or(chunk);
    if !body.contains('{') {
        return body.contains('\n');
    }
    json_objects_complete(body)
}

pub fn extract_json_values(text: &str) -> Vec<serde_json::Value> {
    let mut values = Vec::new();
    let mut idx = 0;
    let bytes = text.as_bytes();
    while idx < bytes.len() {
        if bytes[idx] == b'{' {
            match take_json_object(&text[idx..]) {
                Some((consumed, value)) => {
                    values.push(value);
                    idx += consumed;
                    continue;
                }
                None => break,
            }
        }
        idx += 1;
    }
    values
}

fn json_objects_complete(text: &str) -> bool {
    let mut saw_object = false;
    let mut idx = 0;
    let bytes = text.as_bytes();
    while idx < bytes.len() {
        if bytes[idx] == b'{' {
            saw_object = true;
            match take_json_object(&text[idx..]) {
                Some((consumed, _)) => idx += consumed,
                None => return false,
            }
            continue;
        }
        idx += 1;
    }
    saw_object
}

fn file_key(meta: &fs::Metadata) -> (u64, u64) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        (meta.dev(), meta.ino())
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        (meta.creation_time(), 0)
    }
    #[cfg(not(any(unix, windows)))]
    {
        (meta.len(), 0)
    }
}

fn take_json_object(text: &str) -> Option<(usize, serde_json::Value)> {
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape = false;
    for (offset, ch) in text.char_indices() {
        if in_string {
            if escape {
                escape = false;
                continue;
            }
            match ch {
                '\\' => escape = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    let end = offset + ch.len_utf8();
                    let parsed = serde_json::from_str(&text[..end]).ok()?;
                    return Some((end, parsed));
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("arenacoach-log-{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn headerless_unicode_truncation_keeps_character_boundaries() {
        let input = "界".repeat(30_000);
        let (entries, remainder) = split_entries(&input);
        assert!(entries.is_empty());
        assert!(remainder.len() <= 65_536);
        assert!(input.ends_with(&remainder));
    }

    #[test]
    fn splits_multiline_json_entry() {
        let buffer = "\
[UnityCrossThreadLogger]9/8/2026 5:00:00 PM
{
  \"greToClientEvent\": {
    \"greToClientMessages\": [{\"type\":\"GREMessageType_GameStateMessage\"}]
  }
}
[UnityCrossThreadLogger]9/8/2026 5:00:01 PM
{\"ok\":true}
";
        let (entries, remainder) = split_entries(buffer);
        assert!(remainder.is_empty());
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].timestamp.as_deref(), Some("9/8/2026 5:00:00 PM"));
        assert!(entries[0].body.contains("greToClientEvent"));
        assert_eq!(extract_json_values(&entries[0].body).len(), 1);
    }

    #[test]
    fn keeps_partial_json_at_eof() {
        let buffer = "\
[UnityCrossThreadLogger]9/8/2026 5:00:00 PM
{\"greToClientEvent\":{\"greToClientMessages\":[";
        let (entries, remainder) = split_entries(buffer);
        assert!(entries.is_empty());
        assert!(remainder.contains("greToClientMessages"));
    }

    #[test]
    fn poll_emits_completed_partial_on_next_read() {
        let dir = temp_dir();
        let path = dir.join("Player.log");
        fs::write(
            &path,
            "[UnityCrossThreadLogger]9/8/2026 5:00:00 PM\n{\"a\":",
        )
        .unwrap();
        let mut follower = LogFollower::from_start(path.clone());
        assert!(follower.poll().unwrap().is_empty());

        let mut file = fs::OpenOptions::new().append(true).open(&path).unwrap();
        file.write_all(b"1}\n").unwrap();
        drop(file);

        let entries = follower.poll().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(extract_json_values(&entries[0].body)[0]["a"], 1);
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn rotation_reads_prev_unread_tail_then_new_file() {
        let dir = temp_dir();
        let path = dir.join("Player.log");
        let prev = dir.join("Player-prev.log");
        fs::write(
            &path,
            "[UnityCrossThreadLogger]9/8/2026 5:00:00 PM\n{\"n\":1}\n",
        )
        .unwrap();
        let mut follower = LogFollower::from_start(path.clone());
        assert_eq!(follower.poll().unwrap().len(), 1);

        fs::rename(&path, &prev).unwrap();
        fs::write(
            &prev,
            "[UnityCrossThreadLogger]9/8/2026 5:00:00 PM\n{\"n\":1}\n[UnityCrossThreadLogger]9/8/2026 5:00:01 PM\n{\"n\":2}\n",
        )
        .unwrap();
        fs::write(
            &path,
            "[UnityCrossThreadLogger]9/8/2026 5:00:02 PM\n{\"n\":3}\n",
        )
        .unwrap();

        let entries = follower.poll().unwrap();
        let numbers: Vec<i64> = entries
            .iter()
            .flat_map(|entry| extract_json_values(&entry.body))
            .filter_map(|value| value.get("n").and_then(|n| n.as_i64()))
            .collect();
        assert_eq!(numbers, vec![2, 3]);
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn detects_disabled_detailed_logs() {
        assert_eq!(
            detect_detailed_logs("DETAILED LOGS: DISABLED\nno gre here"),
            DetailedLogs::Off
        );
        assert_eq!(
            detect_detailed_logs("{\"greToClientEvent\":{}}"),
            DetailedLogs::On
        );
    }
}
