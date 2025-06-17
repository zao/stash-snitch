use super::ratelimiter;
use serde::{Deserialize, Serialize};

static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StashEntries {
    pub entries: Vec<StashEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StashEntry {
    pub id: String,
    pub time: u64,
    pub league: String,
    pub stash: Option<String>,
    pub item: String,
    pub action: String,
    pub account: StashAccount,
    pub x: Option<i32>,
    pub y: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StashAccount {
    pub name: String,
    pub realm: Option<String>,
}

pub struct GuildStashAPI {
    limiter: ratelimiter::RateLimiter,
    client: reqwest::blocking::Client,
    log_endpoint: String,
    poesessid_cookie: String,
}

impl GuildStashAPI {
    pub fn new(guildid: i64, sessid: &str) -> Self {
        let log_endpoint = format!(
            "https://www.pathofexile.com/api/guild/{}/stash/history",
            guildid
        );
        let poesessid_cookie = format!("POESESSID={}", sessid);

        let client = reqwest::blocking::Client::builder()
            .user_agent(APP_USER_AGENT)
            .cookie_store(true)
            .build()
            .expect("Could not construct HTTP client");

        Self {
            limiter: ratelimiter::RateLimiter::new(),
            client,
            log_endpoint,
            poesessid_cookie,
        }
    }

    pub fn fetch(&mut self, anchor: Option<&(String, u64)>) -> Option<StashEntries> {
        let mut params = vec![];
        if let Some((ref id, ts)) = anchor {
            params.extend_from_slice(&[("from", ts.to_string()), ("fromid", id.clone())]);
        }

        let url = url::Url::parse_with_params(&self.log_endpoint, params.iter()).unwrap();
        eprintln!("Fetching from {}", url);

        match self.limiter.send(
            &self.client,
            self.client
                .get(url)
                .header("Cookie", &self.poesessid_cookie)
                .build()
                .ok()?,
        ) {
            Ok(resp) => {
                if resp.status().is_success() {
                    let contents: serde_json::Result<StashEntries> =
                        serde_json::from_str(&resp.text().ok()?);
                    if let Ok(contents) = contents {
                        return Some(contents);
                    } else {
                        eprintln!("{:?}", contents.unwrap_err());
                        return None;
                    }
                }
            }
            Err(e) => {
                eprintln!("Error: {:?}", e);
                return None;
            }
        }

        return None;
    }
}
