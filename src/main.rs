mod api;
mod ratelimiter;
mod storage;

use chrono::{TimeZone, Utc};
use std::path::PathBuf;
use structopt::StructOpt;

fn parse_duration(s: &str) -> Result<chrono::Duration, Box<dyn std::error::Error>> {
    let d = humantime::parse_duration(s)?;
    let d = chrono::Duration::from_std(d)?;
    Ok(d)
}

#[derive(Debug, StructOpt)]
#[structopt(
    name = "stash-snitch",
    about = "A tool to compile a summary of recent PoE guild stash changes."
)]
struct Opt {
    /// POESESSID from browser for authentication to API
    #[structopt(short, long)]
    sessid: String,

    /// Guild ID, can be found in guild URL on website, like https://www.pathofexile.com/guild/profile/12345
    #[structopt(short, long)]
    guildid: i64,

    /// Name of output file (CSV or JSON)
    #[structopt(short, long)]
    output: PathBuf,

    /// Amount of time to look back, like "12 days 5 hours"
    #[structopt(long, parse(try_from_str = parse_duration))]
    age_limit: Option<chrono::Duration>,

    /// Maximum number of items to return
    #[structopt(long)]
    count_limit: Option<i64>,

    /// Skip refresh step and use only cached data
    #[structopt(long)]
    skip_refresh: bool,
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let opt = Opt::from_args();
    let now = Utc::now().naive_utc();

    let project_dirs = directories::ProjectDirs::from("se", "zao", "stash-snitch")
        .ok_or("Could not find user profile")?;
    std::fs::create_dir_all(project_dirs.data_dir())?;

    let mut storage = storage::Storage::new(project_dirs.data_dir().join("guild-stash.db"))
        .ok_or("Could not create/open database")?;
    let mut stash_api = api::GuildStashAPI::new(opt.guildid, &opt.sessid);

    let mut results = api::StashEntries { entries: vec![] };

    // First refresh the data from the server, prepending new items to the dataset.
    // Then export to the desired file type.

    if !opt.skip_refresh {
        let mut added_count = 0;
        let mut last_item_key: Option<(String, u64)> = None;

        let mut txn = storage
            .begin_insert()
            .ok_or("Could not being database transaction")?;
        'fetch: loop {
            let chunk = stash_api
                .fetch(last_item_key.as_ref())
                .ok_or("Could not fetch from API")?;
            if chunk.entries.is_empty() {
                break 'fetch;
            }
            let added = txn.insert(opt.guildid, &chunk.entries)?;
            added_count += added;
            if added != chunk.entries.len() {
                break 'fetch;
            }
            let last = chunk.entries.last().unwrap();
            last_item_key = Some((last.id.clone(), last.time));
        }
        txn.commit().ok_or("Could not commit data to database")?;
        eprintln!("{} new entries found.", added_count);
    }

    let start_time = opt.age_limit.map(|age| now - age);
    let entries = storage.fetch(opt.guildid, start_time, opt.count_limit)?;

    let writer = std::fs::File::create(opt.output)?;
    serde_json::to_writer(writer, &entries)?;
    eprintln!("{} entries exported.", entries.entries.len());

    Ok(())
}
