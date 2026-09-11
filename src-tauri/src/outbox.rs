use crate::match_payload::Match;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const MAX_ITEMS: usize = 100;
const MAX_BYTES: u64 = 20 * 1024 * 1024;

#[derive(Clone, Serialize, Deserialize)]
struct Item {
    payload: Match,
    error: Option<String>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct Outbox {
    items: VecDeque<Item>,
}

impl Outbox {
    pub fn path(directory: &Path, developer_mode: bool, user_id: Option<i64>) -> Option<PathBuf> {
        user_id.filter(|id| *id > 0).map(|id| {
            directory.join(format!(
                "outbox-{}-{id}.json",
                if developer_mode { "dev" } else { "prod" }
            ))
        })
    }

    pub fn load(path: &Path) -> std::io::Result<Self> {
        let file = match std::fs::File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default())
            }
            Err(error) => return Err(error),
        };
        let mut bytes = Vec::new();
        file.take(MAX_BYTES + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err(std::io::Error::other("Upload outbox is too large"));
        }
        let outbox: Self = serde_json::from_slice(&bytes).map_err(std::io::Error::other)?;
        if outbox.items.len() > MAX_ITEMS {
            return Err(std::io::Error::other("Upload outbox has too many matches"));
        }
        Ok(outbox)
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        atomic_write(
            path,
            &serde_json::to_vec(self).map_err(std::io::Error::other)?,
        )
    }

    pub fn enqueue(&mut self, payload: Match) -> std::io::Result<()> {
        payload.to_json_bytes().map_err(std::io::Error::other)?;
        let mut next = self.items.clone();
        if let Some(item) = next
            .iter_mut()
            .find(|item| item.payload.client_match_id == payload.client_match_id)
        {
            if item.payload == payload {
                return Ok(());
            }
            *item = Item {
                payload,
                error: None,
            };
        } else {
            if next.len() >= MAX_ITEMS {
                return Err(std::io::Error::other("Upload outbox is full. Sync or remove failed uploads before playing more games."));
            }
            next.push_back(Item {
                payload,
                error: None,
            });
        }
        if serde_json::to_vec(&next)
            .map_err(std::io::Error::other)?
            .len() as u64
            > MAX_BYTES - 1024
        {
            return Err(std::io::Error::other("Upload outbox is full"));
        }
        self.items = next;
        Ok(())
    }

    pub fn next(&self) -> Option<Match> {
        self.items
            .iter()
            .find(|item| item.error.is_none())
            .map(|item| item.payload.clone())
    }

    pub fn finish(&mut self, payload: &Match, error: Option<String>) {
        if let Some(index) = self.items.iter().position(|item| item.payload == *payload) {
            if let Some(error) = error {
                self.items[index].error = Some(error);
            } else {
                self.items.remove(index);
            }
        }
    }

    pub fn retry_failed(&mut self) {
        for item in &mut self.items {
            item.error = None;
        }
    }
    pub fn discard_failed(&mut self) {
        self.items.retain(|item| item.error.is_none());
    }

    pub fn pending(&self) -> usize {
        self.items
            .iter()
            .filter(|item| item.error.is_none())
            .count()
    }
    pub fn failed(&self) -> usize {
        self.items
            .iter()
            .filter(|item| item.error.is_some())
            .count()
    }
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("tmp");
    let mut options = std::fs::OpenOptions::new();
    options.create(true).truncate(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    std::fs::rename(temporary, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn game(id: &str) -> Match {
        serde_json::from_value(serde_json::json!({"client_match_id":id,"event_id":"test","format":"unknown","result":"win","player_seat":1,"deck_grp_ids":[],"timeline":[]})).unwrap()
    }
    #[test]
    fn owner_paths_never_overlap() {
        let dir = Path::new("test");
        assert_ne!(
            Outbox::path(dir, false, Some(1)),
            Outbox::path(dir, false, Some(2))
        );
        assert_ne!(
            Outbox::path(dir, false, Some(1)),
            Outbox::path(dir, true, Some(1))
        );
        assert_eq!(Outbox::path(dir, false, None), None);
    }
    #[test]
    fn permanent_failures_do_not_block_later_uploads() {
        let mut queue = Outbox::default();
        queue.enqueue(game("a")).unwrap();
        queue.enqueue(game("b")).unwrap();
        queue.finish(&game("a"), Some("invalid".into()));
        assert_eq!(queue.next().unwrap().client_match_id, "b");
        assert_eq!(queue.failed(), 1);
    }
    #[test]
    fn completion_does_not_remove_a_newer_version() {
        let mut queue = Outbox::default();
        let old = game("a");
        queue.enqueue(old.clone()).unwrap();
        let mut new = old.clone();
        new.event_id = "updated".into();
        queue.enqueue(new.clone()).unwrap();
        queue.finish(&old, None);
        assert_eq!(queue.next(), Some(new));
    }
    #[test]
    fn pending_matches_survive_restart() {
        let path = std::env::temp_dir().join(format!(
            "arenacoach-outbox-test-{}.json",
            std::process::id()
        ));
        let mut queue = Outbox::default();
        queue.enqueue(game("a")).unwrap();
        queue.save(&path).unwrap();
        assert_eq!(Outbox::load(&path).unwrap().next(), Some(game("a")));
        std::fs::remove_file(path).unwrap();
    }
}
