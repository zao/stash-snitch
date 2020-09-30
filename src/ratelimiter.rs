use std::collections::VecDeque;

struct RateLimit {
    count: u32,
    time: u32,
    penalty: u32,
}

impl std::fmt::Debug for RateLimit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}:{}:{}", self.count, self.time, self.penalty))
    }
}

impl RateLimit {
    fn parse(s: &str) -> Option<RateLimit> {
        let mut parts = s.split(':');
        Some(Self {
            count: parts.next()?.parse().ok()?,
            time: parts.next()?.parse().ok()?,
            penalty: parts.next()?.parse().ok()?,
        })
    }
}

#[derive(Debug)]
pub struct RateLimiter {
    limits: Vec<RateLimit>,
    states: Vec<RateLimit>,
    request_history: VecDeque<std::time::Instant>,
}

impl RateLimiter {
    pub fn new() -> RateLimiter {
        Self {
            limits: vec![],
            states: vec![],
            request_history: VecDeque::new(),
        }
    }

    fn backoff(&self) -> Option<chrono::Duration> {
        let mut backoff = None;
        if !self.request_history.is_empty() {
            let now = std::time::Instant::now();
            for (limit, state) in self.limits.iter().zip(self.states.iter()) {
                if state.count >= limit.count {
                    let penalty = chrono::Duration::milliseconds(state.penalty as i64 * 1000);
                    let oldest = self.request_history.get(limit.count as usize);
                    let time_since_oldest = oldest.map(|t| chrono::Duration::from_std(now - *t).unwrap() + chrono::Duration::milliseconds(500));
                    let this_backoff = Some(penalty).max(time_since_oldest);
                    backoff = backoff.max(this_backoff);
                }
            }
        }
        eprintln!("Backoff is now {:?}", backoff);
        backoff
    }

    fn update_from_headers(&mut self, headers: &reqwest::header::HeaderMap) {
        if let Some(rules) = headers.get("X-Rate-Limit-Rules") {
            for method in rules.to_str().unwrap().split(',') {
                if let Some(limit) = headers.get(format!("X-Rate-Limit-{}", method)) {
                    let limit_triples = limit.to_str().unwrap().split(',').collect::<Vec<_>>();
                    if let Some(state) = headers.get(format!("X-Rate-Limit-{}-State", method)) {
                        let state_triples = state.to_str().unwrap().split(',').collect::<Vec<_>>();
                        if limit_triples.len() == state_triples.len() {
                            self.limits.clear();
                            self.states.clear();
                            for (limit, state) in
                                limit_triples.into_iter().zip(state_triples.into_iter())
                            {
                                if let (Some(limit), Some(state)) =
                                    (RateLimit::parse(limit), RateLimit::parse(state))
                                {
                                    self.limits.push(limit);
                                    self.states.push(state);
                                }
                            }
                        }
                    }
                }
            }
        }
        let history_size = self.limits.iter().map(|lim| lim.count).max().unwrap() as usize;
        self.request_history.truncate(history_size);
    }

    pub fn send(
        &mut self,
        client: &reqwest::blocking::Client,
        req: reqwest::blocking::Request,
    ) -> reqwest::Result<reqwest::blocking::Response> {
        loop {
            if let Some(backoff) = self.backoff() {
                std::thread::sleep(backoff.to_std().unwrap());
            }
            let resp = client.execute(req.try_clone().unwrap());
            self.request_history.push_front(std::time::Instant::now());
            match resp {
                Ok(resp) => {
                    self.update_from_headers(resp.headers());
                    if resp.status() != reqwest::StatusCode::TOO_MANY_REQUESTS {
                        break Ok(resp);
                    }
                }
                e => break e,
            }
        }
    }
}
