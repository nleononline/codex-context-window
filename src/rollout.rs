use memchr::memrchr_iter;
use serde_json::Value;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{task_started, token_count, TempDirectory};
    use serde_json::json;
    use std::{fs, io::Write};

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
}
