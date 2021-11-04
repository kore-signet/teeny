use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;

#[derive(Serialize, Deserialize, Debug)]
struct Import {
    urls: HashMap<String, String>,
    hashes: HashMap<String, String>,
}

fn main() {
    let path = std::env::var("ROCKET_DBPATH").unwrap();

    let db = sled::Config::new().path(&path).open().unwrap();

    let f = File::open("db.json").unwrap();
    let vals: Import = serde_json::from_reader(&f).unwrap();

    for (k, v) in vals.hashes.into_iter() {
        db.insert(base64::decode(k).unwrap(), v.as_bytes()).unwrap();
    }

    for (k, v) in vals.urls.into_iter() {
        db.insert(base64::decode(k).unwrap(), base64::decode(v).unwrap())
            .unwrap();
    }
}
