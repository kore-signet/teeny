use std::{fs::File, io::{BufRead, BufReader, BufWriter, Write}};

use either::Either;
use rand::Rng;
use rocket::{
    form::Form,
    get,
    http::Header,
    http::Status,
    launch, options, post,
    response::{status::NotFound, Redirect},
    routes, FromForm, State,
};
use sha2::{Digest, Sha256};
use url::Url;
use rocksdb::{DB, Options};

type Result<T> = std::result::Result<T, rocket::response::Debug<anyhow::Error>>;

const URL_LIMIT: usize = 256_000;

pub struct CORS;
#[rocket::async_trait]
impl rocket::fairing::Fairing for CORS {
    fn info(&self) -> rocket::fairing::Info {
        rocket::fairing::Info {
            name: "CORS headers",
            kind: rocket::fairing::Kind::Response,
        }
    }

    async fn on_response<'r>(
        &self,
        _: &'r rocket::Request<'_>,
        response: &mut rocket::Response<'r>,
    ) {
        response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(Header::new("Access-Control-Allow-Methods", "GET"));
        response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
    }
}

#[rocket::async_trait]
impl<'r> rocket::response::Responder<'r, 'static> for CORS {
    fn respond_to(self, _: &'r rocket::Request<'_>) -> rocket::response::Result<'static> {
        rocket::Response::build()
            .header(Header::new("Access-Control-Allow-Origin", "*"))
            .header(Header::new("Access-Control-Allow-Methods", "GET"))
            .header(Header::new("Access-Control-Allow-Headers", "*"))
            .header(Header::new("Access-Control-Max-Age", "86400"))
            .header(Header::new("Allow", "OPTIONS, GET"))
            .status(Status::NoContent)
            .ok()
    }
}

#[options("/<_..>")]
pub async fn cors_preflight() -> CORS {
    CORS
}

#[derive(FromForm)]
struct UrlSubmission {
    url: String,
}

#[get("/lookup/<token>")]
fn lookup(token: &str, db: &State<DB>) -> Result<Either<String, NotFound<&'static str>>> {
    if let Some(url) = db.get(&token.as_bytes()).map_err(anyhow::Error::from)? {
        Ok(Either::Left(String::from_utf8(url.to_vec()).unwrap()))
    } else {
        Ok(Either::Right(NotFound("not found")))
    }
}

#[get("/<token>")]
fn redirect(token: &str, db: &State<DB>) -> Result<Either<Redirect, NotFound<&'static str>>> {
    if let Some(url) = db.get(&token.as_bytes()).map_err(anyhow::Error::from)? {
        Ok(Either::Left(Redirect::found(
            String::from_utf8(url).unwrap(),
        )))
    } else {
        Ok(Either::Right(NotFound("not found")))
    }
}

#[post("/submit", data = "<sub>")]
fn submit(
    sub: Form<UrlSubmission>,
    allowed: &State<Vec<String>>,
    db: &State<DB>,
) -> Result<(Status, String)> {

    if sub.url.len() > URL_LIMIT {
        return Ok((Status::UnprocessableEntity, "url too long".to_string()));
    }

    let url = Url::parse(&sub.url).unwrap();
    let domain = addr::parse_domain_name(url.host_str().unwrap()).unwrap();
    if !allowed
        .iter()
        .any(|v| domain.as_str() == v || (domain.root().is_some() && domain.root().unwrap() == v))
    {
        return Ok((Status::Forbidden, "url not allowed".to_string()));
    }

    let hash = Sha256::digest(sub.url.as_bytes());

    if let Some(token) = db.get(hash).map_err(anyhow::Error::from)? {
        Ok((Status::Ok, String::from_utf8(token).unwrap()))
    } else {
        let mut rng = rand::thread_rng();
        loop {
            let id = rng.gen::<[u8; 8]>();
            let token = base64::encode_config(
                &id,
                base64::Config::new(base64::CharacterSet::UrlSafe, false),
            );

            if !db.key_may_exist(&token) {
                db.put(&token, sub.url.as_bytes())
                    .map_err(anyhow::Error::from)?;
                db.put(&hash, token.as_bytes())
                    .map_err(anyhow::Error::from)?;
                return Ok((Status::Ok, token));
            }
        }
    }
}

#[launch]
fn rocket() -> _ {
    let rocket = rocket::build();
    let figment = rocket.figment();
    let db_path: String = figment.extract_inner("dbpath").expect("missing db path");

    let mut max_len = 0;

    let db = DB::open_default(db_path).unwrap();
    if let Ok(import_path) = figment.extract_inner::<String>("csvimport") {
        let csv = BufReader::new(File::open(import_path).unwrap());
        for line in csv.lines() {
            let line = line.unwrap();
            let (key, value) = line.split_once(',').unwrap();
            let (key, value) = (base64::decode(key).unwrap(), base64::decode(value).unwrap());
            println!("importing {}={}", String::from_utf8_lossy(&key), String::from_utf8_lossy(&value));
            max_len = std::cmp::max(max_len, key.len());
            max_len = std::cmp::max(max_len, value.len());
            db.put(
                key
                ,value).unwrap();

     
        }
    }

    println!("max len = {max_len}");

    let allowed: Vec<String> = figment
        .extract_inner::<String>("allowlist")
        .expect("missing allowlist")
        .split_terminator(";")
        .map(|v| v.to_owned())
        .collect();

    rocket
        .manage(db)
        .manage(allowed)
        .attach(CORS)
        .mount("/", routes![cors_preflight, lookup, redirect, submit])
}

