use std::fs::File;
use std::collections::HashMap;

fn main() {
    let path = std::env::var("ROCKET_DBPATH").unwrap();

    let db = sled::Config::new()
        .path(&path)
        .open()
        .unwrap();

    let f = File::open("db.json").unwrap();
    let vals: HashMap<String,String> = serde_json::from_reader(&f).unwrap();
    for (k,v) in vals.into_iter() {
        db.insert(base64::decode(k).unwrap(), base64::decode(v).unwrap()).unwrap();
    }
}