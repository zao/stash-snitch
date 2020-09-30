# Stash Snitch

## Usage

The tool keeps a local database in the user's profile directories which it populates with new log entries from the API.
It always fetches the full set of changes and as such may take some time on first run as there is enough data for active guilds to be rate-limited.
```
stash-snitch 0.2.0
A tool to compile a summary of recent PoE guild stash changes.

USAGE:
    stash-snitch.exe [FLAGS] [OPTIONS] --guildid <guildid> --output <output> --sessid <sessid>

FLAGS:
    -h, --help            Prints help information
        --skip-refresh    Skip refresh step and use only cached data
    -V, --version         Prints version information

OPTIONS:
        --age-limit <age-limit>        Amount of time to look back, like "12 days 5 hours"
        --count-limit <count-limit>    Maximum number of items to return
    -g, --guildid <guildid>            Guild ID, can be found in guild URL on website, like
                                       https://www.pathofexile.com/guild/profile/12345
    -o, --output <output>              Name of output file (CSV or JSON)
    -s, --sessid <sessid>              POESESSID from browser for authentication to API
```

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.