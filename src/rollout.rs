use crate::non_empty_env_path;
use memchr::memrchr_iter;
use serde_json::Value;
use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

const READ_CHUNK_SIZE: usize = 64 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TokenUsage {
    pub used: u64,
    pub limit: u64,
}

enum RelevantLine<T> {
    Irrelevant,
    Relevant(Option<T>),
}

fn token_usage_from_line(line: &[u8]) -> RelevantLine<TokenUsage> {
    let Ok(record) = serde_json::from_slice::<Value>(line) else {
        return RelevantLine::Irrelevant;
    };

    if record.get("type").and_then(Value::as_str) != Some("event_msg")
        || record.pointer("/payload/type").and_then(Value::as_str) != Some("token_count")
    {
        return RelevantLine::Irrelevant;
    }

    let used = record
        .pointer("/payload/info/last_token_usage/total_tokens")
        .and_then(Value::as_u64);
    let limit = record
        .pointer("/payload/info/model_context_window")
        .and_then(Value::as_u64)
        .filter(|limit| *limit > 0);

    RelevantLine::Relevant(
        used.zip(limit)
            .map(|(used, limit)| TokenUsage { used, limit }),
    )
}

fn context_window_limit_from_line(line: &[u8]) -> RelevantLine<u64> {
    let Ok(record) = serde_json::from_slice::<Value>(line) else {
        return RelevantLine::Irrelevant;
    };

    if record.get("type").and_then(Value::as_str) != Some("event_msg")
        || record.pointer("/payload/type").and_then(Value::as_str) != Some("task_started")
    {
        return RelevantLine::Irrelevant;
    }

    let limit = record
        .pointer("/payload/model_context_window")
        .and_then(Value::as_u64)
        .filter(|limit| *limit > 0);

    RelevantLine::Relevant(limit)
}

fn read_last_relevant<T>(
    file_path: &Path,
    parse_line: impl Fn(&[u8]) -> RelevantLine<T>,
) -> io::Result<Option<T>> {
    let mut file = File::open(file_path)?;
    let mut position = file.metadata()?.len();
    let mut carry = Vec::new();

    while position > 0 {
        let length = usize::try_from(position.min(READ_CHUNK_SIZE as u64))
            .expect("chunk length always fits usize");
        position -= length as u64;

        file.seek(SeekFrom::Start(position))?;
        let mut data = vec![0; length];
        file.read_exact(&mut data)?;
        data.extend_from_slice(&carry);

        let mut line_end = data.len();
        for newline_index in memrchr_iter(b'\n', &data) {
            let line = &data[newline_index + 1..line_end];
            if !line.is_empty() {
                match parse_line(line) {
                    RelevantLine::Irrelevant => {}
                    RelevantLine::Relevant(value) => return Ok(value),
                }
            }
            line_end = newline_index;
        }

        carry = data[..line_end].to_vec();
    }

    if !carry.is_empty() {
        if let RelevantLine::Relevant(value) = parse_line(&carry) {
            return Ok(value);
        }
    }

    Ok(None)
}

pub fn read_last_token_usage(file_path: &Path) -> io::Result<Option<TokenUsage>> {
    read_last_relevant(file_path, token_usage_from_line)
}

pub fn read_context_window_limit(file_path: &Path) -> io::Result<Option<u64>> {
    read_last_relevant(file_path, context_window_limit_from_line)
}

fn matches_rollout_name(file_path: &Path, session_id: &str) -> bool {
    let Some(name) = file_path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };

    name.starts_with("rollout-") && name.ends_with(&format!("-{session_id}.jsonl"))
}

fn default_codex_home() -> Option<PathBuf> {
    if let Some(codex_home) = non_empty_env_path("CODEX_HOME") {
        return Some(codex_home);
    }

    #[cfg(windows)]
    let home = non_empty_env_path("USERPROFILE");

    #[cfg(not(windows))]
    let home = non_empty_env_path("HOME");

    home.map(|home| home.join(".codex"))
}

pub fn find_session_file(
    transcript_path: Option<&Path>,
    session_id: Option<&str>,
    codex_home: Option<&Path>,
) -> Option<PathBuf> {
    if let Some(transcript_path) = transcript_path {
        if transcript_path
            .metadata()
            .map(|metadata| metadata.is_file())
            .unwrap_or(false)
        {
            return Some(transcript_path.to_path_buf());
        }
    }

    let session_id = session_id?;
    let sessions_root = codex_home
        .map(Path::to_path_buf)
        .or_else(default_codex_home)?
        .join("sessions");

    let mut directories = vec![sessions_root];
    let mut matched_file = None;

    while let Some(directory) = directories.pop() {
        let Ok(entries) = fs::read_dir(directory) else {
            continue;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(file_type) = entry.file_type() else {
                continue;
            };

            if file_type.is_dir() {
                directories.push(path);
                continue;
            }

            if !file_type.is_file() || !matches_rollout_name(&path, session_id) {
                continue;
            }

            if matched_file.is_some() {
                return None;
            }
            matched_file = Some(path);
        }
    }

    matched_file
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{task_started, token_count, TempDirectory};
    use serde_json::json;
    use std::io::Write;

    #[test]
    fn reads_newest_token_count_from_end() {
        let temp = TempDirectory::new();
        let rollout = temp.path().join("rollout-test-thread.jsonl");
        let mut file = File::create(&rollout).expect("create rollout");

        writeln!(file, "{}", token_count(100, 1_000)).unwrap();
        writeln!(
            file,
            "{}",
            json!({"type": "event_msg", "payload": {"type": "other"}})
        )
        .unwrap();
        writeln!(file, "{{not-json}}").unwrap();
        writeln!(file, "{}", token_count(350, 1_000)).unwrap();
        writeln!(file, "{}", json!({"type": "response_item"})).unwrap();

        assert_eq!(
            read_last_token_usage(&rollout).unwrap(),
            Some(TokenUsage {
                used: 350,
                limit: 1_000,
            })
        );
    }

    #[test]
    fn reads_token_count_line_larger_than_one_chunk() {
        let temp = TempDirectory::new();
        let rollout = temp.path().join("rollout-test-thread.jsonl");
        let large_record = json!({
            "type": "event_msg",
            "payload": {
                "type": "token_count",
                "info": {
                    "last_token_usage": { "total_tokens": 700 },
                    "model_context_window": 1_000
                }
            },
            "padding": "x".repeat(READ_CHUNK_SIZE + 1_024)
        });

        fs::write(
            &rollout,
            format!(
                "{}\n{}\n{}\n",
                token_count(100, 1_000),
                large_record,
                json!({"type": "response_item"})
            ),
        )
        .unwrap();

        assert_eq!(
            read_last_token_usage(&rollout).unwrap(),
            Some(TokenUsage {
                used: 700,
                limit: 1_000,
            })
        );
    }

    #[test]
    fn does_not_use_stale_usage_when_newest_event_lacks_info() {
        let temp = TempDirectory::new();
        let rollout = temp.path().join("rollout-test-thread.jsonl");
        fs::write(
            &rollout,
            format!(
                "{}\n{}\n",
                token_count(100, 1_000),
                json!({
                    "type": "event_msg",
                    "payload": {"type": "token_count", "info": null}
                })
            ),
        )
        .unwrap();

        assert_eq!(read_last_token_usage(&rollout).unwrap(), None);
    }

    #[test]
    fn reads_context_window_limit_from_task_started() {
        let temp = TempDirectory::new();
        let rollout = temp.path().join("rollout-test-thread.jsonl");
        fs::write(
            &rollout,
            format!(
                "{}\n{}\n",
                task_started(258_400),
                json!({"type": "turn_context"})
            ),
        )
        .unwrap();

        assert_eq!(read_context_window_limit(&rollout).unwrap(), Some(258_400));
    }

    #[test]
    fn does_not_use_stale_limit_when_newest_task_start_lacks_it() {
        let temp = TempDirectory::new();
        let rollout = temp.path().join("rollout-test-thread.jsonl");
        fs::write(
            &rollout,
            format!(
                "{}\n{}\n",
                task_started(258_400),
                json!({
                    "type": "event_msg",
                    "payload": {"type": "task_started"}
                })
            ),
        )
        .unwrap();

        assert_eq!(read_context_window_limit(&rollout).unwrap(), None);
    }

    #[test]
    fn finds_unique_rollout_by_session_id() {
        let temp = TempDirectory::new();
        let session_id = "session-123";
        let session_directory = temp.path().join("sessions/2026/07/27");
        fs::create_dir_all(&session_directory).unwrap();

        let rollout = session_directory.join(format!("rollout-current-{session_id}.jsonl"));
        fs::write(&rollout, "").unwrap();

        assert_eq!(
            find_session_file(None, Some(session_id), Some(temp.path())),
            Some(rollout)
        );
    }

    #[test]
    fn rejects_ambiguous_rollout_matches() {
        let temp = TempDirectory::new();
        let session_id = "session-123";
        let older_directory = temp.path().join("sessions/2026/07/26");
        let newer_directory = temp.path().join("sessions/2026/07/27");
        fs::create_dir_all(&older_directory).unwrap();
        fs::create_dir_all(&newer_directory).unwrap();

        let older = older_directory.join(format!("rollout-old-{session_id}.jsonl"));
        let newer = newer_directory.join(format!("rollout-new-{session_id}.jsonl"));
        fs::write(&older, "").unwrap();
        fs::write(&newer, "").unwrap();

        assert_eq!(
            find_session_file(None, Some(session_id), Some(temp.path())),
            None
        );
    }

    #[test]
    fn trusts_hook_transcript_path_without_matching_its_name() {
        let temp = TempDirectory::new();
        let transcript = temp.path().join("transcript.jsonl");
        fs::write(&transcript, "").unwrap();

        assert_eq!(
            find_session_file(
                Some(&transcript),
                Some("different-session-id"),
                Some(&temp.path().join("missing"))
            ),
            Some(transcript)
        );
    }
}
