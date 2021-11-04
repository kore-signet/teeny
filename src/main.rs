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

type Result<T> = std::result::Result<T, rocket::response::Debug<anyhow::Error>>;

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
fn lookup(token: &str, db: &State<sled::Db>) -> Result<Either<String, NotFound<&'static str>>> {
    if let Some(url) = db
        .get(
            &base64::decode_config(
                &token,
                base64::Config::new(base64::CharacterSet::UrlSafe, false),
            )
            .unwrap(),
        )
        .map_err(anyhow::Error::from)?
    {
        Ok(Either::Left(String::from_utf8(url.to_vec()).unwrap()))
    } else {
        Ok(Either::Right(NotFound("not found")))
    }
}

#[get("/<token>")]
fn redirect(token: &str, db: &State<sled::Db>) -> Result<Either<Redirect, NotFound<&'static str>>> {
    if let Some(url) = db
        .get(
            &base64::decode_config(
                &token,
                base64::Config::new(base64::CharacterSet::UrlSafe, false),
            )
            .unwrap(),
        )
        .map_err(anyhow::Error::from)?
    {
        Ok(Either::Left(Redirect::found(
            String::from_utf8(url.to_vec()).unwrap(),
        )))
    } else {
        Ok(Either::Right(NotFound("not found")))
    }
}

#[post("/submit", data = "<sub>")]
fn submit(
    sub: Form<UrlSubmission>,
    allowed: &State<Vec<String>>,
    db: &State<sled::Db>,
) -> Result<(Status, String)> {
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
        Ok((Status::Ok, String::from_utf8(token.to_vec()).unwrap()))
    } else {
        let mut rng = rand::thread_rng();
        loop {
            let id = rng.gen::<[u8; 8]>();
            if !db.contains_key(&id).map_err(anyhow::Error::from)? {
                let token = base64::encode_config(
                    &id,
                    base64::Config::new(base64::CharacterSet::UrlSafe, false),
                );
                db.insert(&id, sub.url.as_bytes())
                    .map_err(anyhow::Error::from)?;
                db.insert(&hash, token.as_bytes())
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

    let db = sled::Config::new()
        .path(&db_path)
        .open()
        .unwrap();

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
