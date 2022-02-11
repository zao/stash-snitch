mod api;
mod ratelimiter;
mod storage;

use chrono::{DateTime, Local, TimeZone, Utc};
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
    sessid: Option<String>,

    /// Guild ID, last number in guild URL from website, like https://www.pathofexile.com/guild/profile/12345
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

    let mut storage = storage::Storage::new(project_dirs.data_dir().join("guild-stash.db"))?;
    // .unwrap_or("Could not create/open database")?;

    // First refresh the data from the server, prepending new items to the dataset.
    // Then export to the desired file type.

    if !opt.skip_refresh {
        if opt.sessid.is_none() {
            eprintln!("Session ID must be given unless refresh is skipped");
            Err("Session ID must be given unless refresh is skipped")?;
        }
        let mut stash_api = api::GuildStashAPI::new(opt.guildid, opt.sessid.as_ref().unwrap());
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

    match opt.output.extension().and_then(std::ffi::OsStr::to_str) {
        Some("json") => {
            let writer = std::fs::File::create(opt.output)?;
            serde_json::to_writer(writer, &entries)?;
            eprintln!("{} entries exported.", entries.entries.len());
        }
        Some("csv") => {
            let mut writer = csv::Writer::from_path(opt.output)?;
            writer.write_record(&[
                "id",
                "time",
                "league",
                "item",
                "action",
                "account_name",
                "account_realm",
                "stash",
            ])?;
            for e in &entries.entries {
                let t = Utc.timestamp(e.time as i64, 0);
                let localtime: DateTime<Local> = t.into();
                writer.write_record(&[
                    &e.id,
                    &localtime.to_rfc3339(),
                    &e.league,
                    &e.item,
                    &e.action,
                    &e.account.name,
                    &e.account.realm,
                    &e.stash.clone().unwrap_or_else(|| "".to_string()),
                ])?;
            }
            eprintln!("{} entries exported.", entries.entries.len());
        }
        _ => {
            eprintln!("Unknown output extension for output {:?}", opt.output);
        }
    }

    Ok(())
}
