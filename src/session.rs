use crate::message_history::Message;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Session {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub messages: Vec<Message>,
}

impl Session {
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        Session {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            created_at: now,
            updated_at: now,
            messages: Vec::new(),
        }
    }
}

pub struct SessionManager {
    sessions_dir: PathBuf,
    current_session_id: Option<String>,
}

impl SessionManager {
    pub fn new(sessions_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(sessions_dir)
            .with_context(|| format!("Failed to create sessions directory {:?}", sessions_dir))?;
        Ok(SessionManager {
            sessions_dir: sessions_dir.to_path_buf(),
            current_session_id: None,
        })
    }

    pub fn create(&mut self, name: impl Into<String>) -> Result<Session> {
        let session = Session::new(name);
        self.save(&session)?;
        self.current_session_id = Some(session.id.clone());
        Ok(session)
    }

    pub fn save(&self, session: &Session) -> Result<()> {
        let path = self.sessions_dir.join(format!("{}.json", session.id));
        let mut session_to_save = session.clone();
        session_to_save.updated_at = Utc::now();
        let content = serde_json::to_string_pretty(&session_to_save)
            .with_context(|| "Failed to serialize session")?;
        std::fs::write(&path, content)
            .with_context(|| format!("Failed to write session to {:?}", path))?;
        Ok(())
    }

    pub fn load(&self, id: &str) -> Result<Session> {
        let path = self.sessions_dir.join(format!("{}.json", id));
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read session {:?}", path))?;
        let session: Session = serde_json::from_str(&content)
            .with_context(|| "Failed to parse session JSON")?;
        Ok(session)
    }

    pub fn list(&self) -> Result<Vec<Session>> {
        let mut sessions = Vec::new();
        for entry in std::fs::read_dir(&self.sessions_dir)
            .with_context(|| "Failed to read sessions directory")?
        {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let content = std::fs::read_to_string(&path)?;
                if let Ok(session) = serde_json::from_str::<Session>(&content) {
                    sessions.push(session);
                }
            }
        }
        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(sessions)
    }

    pub fn switch(&mut self, id: &str) -> Result<()> {
        let _ = self.load(id)?;
        self.current_session_id = Some(id.to_string());
        Ok(())
    }

    pub fn delete(&mut self, id: &str) -> Result<()> {
        let path = self.sessions_dir.join(format!("{}.json", id));
        if path.exists() {
            std::fs::remove_file(&path)
                .with_context(|| format!("Failed to delete session {:?}", path))?;
        }
        if self.current_session_id.as_deref() == Some(id) {
            self.current_session_id = None;
        }
        Ok(())
    }

    pub fn current(&self) -> Option<Session> {
        let id = self.current_session_id.as_ref()?;
        self.load(id).ok()
    }

    pub fn current_id(&self) -> Option<&String> {
        self.current_session_id.as_ref()
    }

    pub fn update_current(&self, session: &Session) -> Result<()> {
        if self.current_session_id.as_ref() == Some(&session.id) {
            self.save(session)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_session_manager_create_and_list() {
        let dir = TempDir::new().unwrap();
        let mut manager = SessionManager::new(dir.path()).unwrap();

        let session = manager.create("test-session").unwrap();
        assert_eq!(session.name, "test-session");
        assert!(session.messages.is_empty());

        let sessions = manager.list().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].name, "test-session");
    }

    #[test]
    fn test_session_save_and_load() {
        let dir = TempDir::new().unwrap();
        let mut manager = SessionManager::new(dir.path()).unwrap();

        let mut session = manager.create("my-session").unwrap();
        session.messages.push(Message::user("Hello"));
        manager.save(&session).unwrap();

        let loaded = manager.load(&session.id).unwrap();
        assert_eq!(loaded.messages.len(), 1);
        assert_eq!(loaded.name, "my-session");
    }

    #[test]
    fn test_session_switch_and_delete() {
        let dir = TempDir::new().unwrap();
        let mut manager = SessionManager::new(dir.path()).unwrap();

        let s1 = manager.create("first").unwrap();
        let s2 = manager.create("second").unwrap();

        manager.switch(&s2.id).unwrap();
        assert_eq!(manager.current().unwrap().id, s2.id);

        manager.delete(&s1.id).unwrap();
        assert_eq!(manager.list().unwrap().len(), 1);
    }
}
