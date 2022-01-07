use std::env;
use std::process::exit;
use std::io;

use chrono::prelude::*;
use serde_json::Value;

//TODO: Add a settings file with options for:
//    - Client ID and OAuth token
//    - Media player (to use with Streamlink)
//    - Media player arguments
//TODO: Add support for writing settings to file with runtime args
//TODO: Add option to select a stream and open with Streamlink
//TODO: Refactor game and stream fetching to use a single function


macro_rules! to_str {
    ($val: expr, $key: expr) => {
        $val.get($key).unwrap().as_str().unwrap().to_string()
    };
}

macro_rules! to_num {
    ($val: expr, $key: expr) => {
        $val.get($key).unwrap().as_i64().unwrap()
    };
}

fn to_instant(ds: &str) -> String {
    match ds.parse::<DateTime<Utc>>() {
        Ok(val) => {
            let dur = Utc::now() - val;
            format!("{:02}:{:02}", dur.num_hours(), dur.num_minutes() % 60)
        }
        Err(_e) => "".to_string(),
    }
}

#[derive(Debug)]
struct Games {
    name: String,
    id: String,
}

#[derive(Debug)]
struct Entry {
    lang: String,
    display_name: String,
    title: String,
    game_id: String,
    viewer_count: i64,
    live_duration: String,
}

fn filter(entry: &Entry, term: &str, ignored_names: &[&str]) -> bool {
    let display_name: &str = &entry.display_name.to_lowercase();
    // 
    if ignored_names.contains(&display_name) {
        return false;
    }

    if entry.title.to_lowercase().contains(term) {
        true
    } else {
        false
    }
}

fn print(entry: Entry) {
    print!("{} | ", entry.lang);
    print!("https://twitch.tv/{:<14} | ", entry.display_name);
    print!("{:>4} viewers | ", entry.viewer_count);
    print!("{} | ", entry.live_duration);
    print!("{}\n", entry.title);
}

fn to_entry(value: &mut Value) -> Entry {
    let value = value.take();

    Entry {
        lang: to_str!(value, "language"),
        display_name: to_str!(value, "user_name"),
        title: to_str!(value, "title"),
        game_id: to_str!(value, "game_id"),
        viewer_count: to_num!(value, "viewer_count"),
        live_duration: to_instant(&to_str!(value, "started_at")),
    }
}

fn fetch(after: Option<String>, id: String) -> (Vec<Entry>, Option<String>) {
    let root_url = "https://api.twitch.tv/helix/streams?first=100;game_id=".to_owned() + &id;
    let url = match after {
        Some(after) => format!("{}&after={}", (root_url.to_string() + &id), after),
        None => root_url.to_string(),
    };

    let client_id = match env::var("TWITCH_CLIENT_ID") {
        Ok(cid) => cid,
        Err(_e) => {
            eprintln!("Client id missing");
            exit(1);
        }
    };

    let token = match env::var("TWITCH_TOKEN") {
        Ok(t) => t,
        Err(_e) => {
            eprintln!("OAuth token missing");
            exit(1);
        }
    };

    let resp = ureq::get(&url)
        .set("Authorization", &format!("Bearer {}", token))
        .set("Client-Id", &client_id)
        .call();

    let mut json: Value = match resp.unwrap().into_json() {
        Ok(j) => j,
        Err(e) => {
            eprintln!("failed to serialize json: {:?}", e);
            exit(1);
        }
    };

    let pagination = json
        .get_mut("pagination")
        .take()
        .and_then(|v| v.get("cursor").take())
        .and_then(|v| v.as_str())
        .map(|v| v.to_string());

    let data = match json.get_mut("data") {
        Some(Value::Array(a)) => a.into_iter().map(to_entry).collect::<Vec<_>>(),
        _ => {
            exit(0);
        }
    };

    (data, pagination)
}

fn fetch_categories(id: String) -> Vec<Games> {
    let url = "https://api.twitch.tv/helix/search/categories?query=".to_owned() + &id;
    let client_id = match env::var("TWITCH_CLIENT_ID") {
        Ok(cid) => cid,
        Err(_e) => {
            eprintln!("Client id missing");
            exit(1);
        }
    };
    let token = match env::var("TWITCH_TOKEN") {
        Ok(t) => t,
        Err(_e) => {
            eprintln!("OAuth token missing");
            exit(1);
        }
    };
    let resp = ureq::get(&url)
        .set("Authorization", &format!("Bearer {}", token))
        .set("Client-Id", &client_id)
        .call();
    let mut json: Value = match resp.unwrap().into_json() {
        Ok(j) => j,
        Err(e) => {
            eprintln!("failed to serialize json: {:?}", e);
            exit(1);
        }
    };
    let games = match json.get_mut("data") {
        Some(Value::Array(a)) => a.into_iter().map(|v| {
            let v = v.take();
            Games {
                name: to_str!(v, "name"),
                id: to_str!(v, "id"),
            }
        }).collect::<Vec<_>>(),
        _ => {
            exit(0);
        }
    };
    games
}

//Let the user choose a game
fn choose_game(games: Vec<Games>) -> String {
    let mut i = 0;
    for game in &games {
        println!("{}: {}", i, game.name);
        i += 1;
    }
    let mut choice = String::new();
    io::stdin().read_line(&mut choice).unwrap();
    let choice = choice.trim().parse::<usize>().unwrap();
    println!("Category: {}", games[choice].name);
    let search_id = &games[choice].id;
    search_id.to_string()
}

//Let the user enter a search term
fn choose_term() -> String {
    let mut term = String::new();
    io::stdin().read_line(&mut term).unwrap();
    term.trim().to_string()
}


fn main() {
    println!("Enter a category name to search for");
    let category_term = choose_term();
    if category_term.is_empty() {
        println!("A category name is required");
        exit(1);
    }
    let game_choice = &choose_game(fetch_categories(category_term));
    println!("Enter a search term");
    let search_term = choose_term();

    println!("Searching for \"{}\" in chosen category", search_term);

    let mut total = 0;
    let mut found = 0;

    let mut page = None;
    loop {
        let (entries, p) = fetch(page, game_choice.to_string());
        total += entries.len();
        page = p;
        for entry in entries
            .into_iter()
            .filter(|e| filter(e, &search_term, &[""]))
            .collect::<Vec<_>>()
        {
            print(entry);
            found += 1;
        }

        if page.is_none() {
            break;
        }
    }
    println!("Done ({}/{})", found, total);
}
