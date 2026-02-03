use git2::Repository;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

const CONFIG_DIR: &str = "git-igitt";
const CONFIG_FILE: &str = "gitlab.toml";

#[derive(Debug, Clone, Default)]
pub struct RemoteInfo {
    pub host: Option<String>,
    pub url: Option<String>,
    pub project_id: Option<String>,
}

impl RemoteInfo {
    pub fn from_repository(repo: &Repository) -> Self {
        for remote_name in ["gitlab", "origin"] {
            if let Ok(remote) = repo.find_remote(remote_name) {
                if let Some(url_str) = remote.url() {
                    let info = Self::parse_remote_url(url_str);
                    if info.host.is_some() {
                        return info;
                    }
                }
            }
        }
        Self::default()
    }

    fn parse_remote_url(url: &str) -> Self {
        if let Some(rest) = url.strip_prefix("git@") {
            if let Some(colon_pos) = rest.find(':') {
                let host = &rest[..colon_pos];
                let path = rest[colon_pos + 1..].trim_end_matches(".git");
                return Self {
                    host: Some(host.to_string()),
                    url: Some(format!("https://{}", host)),
                    project_id: Some(path.to_string()),
                };
            }
        }

        if url.starts_with("https://") || url.starts_with("http://") {
            if let Ok(parsed) = url::Url::parse(url) {
                let host = parsed.host_str().unwrap_or("");
                let scheme = parsed.scheme();
                let path = parsed
                    .path()
                    .trim_start_matches('/')
                    .trim_end_matches(".git");
                if !host.is_empty() && !path.is_empty() {
                    return Self {
                        host: Some(host.to_string()),
                        url: Some(format!("{}://{}", scheme, host)),
                        project_id: Some(path.to_string()),
                    };
                }
            }
        }

        Self::default()
    }

    pub fn is_valid(&self) -> bool {
        self.host.is_some() && self.url.is_some() && self.project_id.is_some()
    }
}

#[derive(Debug, Clone, Default)]
pub struct GitLabConfigDialog {
    pub host: String,
    pub token: String,
    pub cursor_pos: usize,
}

impl GitLabConfigDialog {
    pub fn new(host: &str, existing_token: Option<&str>) -> Self {
        let token = existing_token.unwrap_or_default().to_string();
        let cursor_pos = token.len();
        Self {
            host: host.to_string(),
            token,
            cursor_pos,
        }
    }

    pub fn insert_char(&mut self, c: char) {
        if self.cursor_pos >= self.token.len() {
            self.token.push(c);
        } else {
            self.token.insert(self.cursor_pos, c);
        }
        self.cursor_pos += 1;
    }

    pub fn delete_char(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            if self.cursor_pos < self.token.len() {
                self.token.remove(self.cursor_pos);
            }
        }
    }

    pub fn delete_forward(&mut self) {
        if self.cursor_pos < self.token.len() {
            self.token.remove(self.cursor_pos);
        }
    }

    pub fn move_cursor_left(&mut self) {
        self.cursor_pos = self.cursor_pos.saturating_sub(1);
    }

    pub fn move_cursor_right(&mut self) {
        self.cursor_pos = (self.cursor_pos + 1).min(self.token.len());
    }

    pub fn move_cursor_home(&mut self) {
        self.cursor_pos = 0;
    }

    pub fn move_cursor_end(&mut self) {
        self.cursor_pos = self.token.len();
    }

    pub fn is_valid(&self) -> bool {
        !self.token.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GitLabConfig {
    #[serde(default)]
    pub tokens: HashMap<String, String>,
}

impl GitLabConfig {
    pub fn load() -> Result<Self, String> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read GitLab config: {}", e))?;

        toml::from_str(&content).map_err(|e| format!("Failed to parse GitLab config: {}", e))
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path()?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
        }

        let content = toml::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize GitLab config: {}", e))?;

        fs::write(&path, content).map_err(|e| format!("Failed to write GitLab config: {}", e))
    }

    pub fn config_path() -> Result<PathBuf, String> {
        let config_dir =
            dirs::config_dir().ok_or_else(|| "Could not determine config directory".to_string())?;
        Ok(config_dir.join(CONFIG_DIR).join(CONFIG_FILE))
    }

    pub fn get_token(&self, host: &str) -> Option<&str> {
        self.tokens.get(host).map(|s| s.as_str())
    }

    pub fn set_token(&mut self, host: &str, token: &str) {
        self.tokens.insert(host.to_string(), token.to_string());
    }

    pub fn has_token_for(&self, host: &str) -> bool {
        self.tokens.contains_key(host)
    }
}
