extern crate ini;
extern crate iron;
extern crate postgres;
extern crate router;
extern crate rustc_serialize;
extern crate url;

use ini::Ini;
use iron::*;
use iron::mime::{Mime, TopLevel, SubLevel};
use postgres::{Connection, ConnectParams, ConnectTarget, SslMode, UserInfo};
use rustc_serialize::json;

use std::io::Read;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

mod db;

const HELP: &'static str = "Usage: phonebook COMMAND [ARG]...
Commands:
	add NAME PHONE - create new record;
	del ID1 ID2... - delete record;
	edit ID        - edit record;
	show           - display all records;
	show STRING    - display records which contain a given substring in the name;
	help           - display this help.";

fn params() -> (ConnectParams, SslMode) {
    let conf = Ini::load_from_file(".phonebookrc").unwrap();
    let general = conf.general_section();

    let host = general.get("host").unwrap();
    let port = general.get("port").unwrap();
    let sslmode = general.get("sslmode").unwrap();
    let dbname = general.get("dbname").unwrap();
    let user = general.get("user").unwrap();
    let pass = general.get("pass").unwrap();

    let s = match sslmode.as_ref() {
        "disable" => SslMode::None,
        "enable" => unimplemented!(),
        _ => panic!("Wrong sslmode"),
    };

    (ConnectParams {
        target: ConnectTarget::Tcp(host.to_owned()),
        port: Some(FromStr::from_str(port).ok().unwrap()),
        user: Some(UserInfo {
            user: user.to_owned(),
            password: Some(pass.to_owned()),
        }),
        database: Some(dbname.to_owned()),
        options: vec![],
    }, s)
}

fn main() {
    let (params, sslmode) = params();
    let db = Connection::connect(params, &sslmode).unwrap();

    db.execute(
        concat!(r#"CREATE TABLE IF NOT EXISTS phonebook"#,
                r#"("id" SERIAL PRIMARY KEY, "name" varchar(50),"#,
                r#" "phone" varchar(100))"#,
                ),
        &[]
            ).unwrap();

    let args: Vec<String> = std::env::args().collect();
    match args.get(1) {
        Some(text) => {
            match text.as_ref() {
                "add" => {
                    if args.len() != 4 {
                        panic!("Usage: phonebook add NAME PHONE");
                    }
                    let r = db::insert(&db, &args[2], &args[3])
                        .unwrap();
                    println!("{} rows affected", r);
                },
                "del" => {
                    if args.len() < 3 {
                        panic!("Usage: phonebook del ID...");
                    }
                    let ids: Vec<i32> = args[2..].iter()
                        .map(|s| s.parse().unwrap())
                        .collect();

                    db::remove(&db, &ids)
                        .unwrap();
                },
                "edit" => {
                    if args.len() != 5 {
                        panic!("Usage: phonebook edit ID NAME PHONE");
                    }
                    let id = args[2].parse().unwrap();
                    db::update(&db, id, &args[3], &args[4])
                        .unwrap();
                },
                "show" => {
                    if args.len() > 3 {
                        panic!("Usage: phonebook show [SUBSTRING]");
                    }
                    let s;
                    if args.len() == 3 {
                        s = args.get(2);
                    } else {
                        s = None;
                    }
                    let r = db::show(&db, s.as_ref().map(|s| &s[..])).unwrap();
                    db::format(&r);
                },
                "help" => {
                    println!("{}", HELP);
                },
                "serve" => {
                    let sdb = Arc::new(Mutex::new(db));
                    let mut router = router::Router::new();
                    {
                        let sdb_ = sdb.clone();
                        router.get("/api/v1/records",
                                   move |req: &mut Request|
                                   get_records(sdb_.clone(), req));
                    }
                    {
                        let sdb_ = sdb.clone();
                        router.get("/api/v1/records/:id",
                                   move |req: &mut Request|
                                   get_record(sdb_.clone(), req));
                    }
                    {
                        let sdb_ = sdb.clone();
                        router.post("/api/v1/records",
                                move |req: &mut Request|
                                add_record(sdb_.clone(), req));
                    }
                    {
                        let sdb_ = sdb.clone();
                        router.put("/api/v1/records/:id",
                                   move |req: &mut Request|
                                   update_record(sdb_.clone(), req));
                    }
                    {
                        let sdb_ = sdb.clone();
                        router.delete("/api/v1/records/:id",
                                      move |req: &mut Request|
                                      delete_record(sdb_.clone(), req));

                    }

                    Iron::new(router).http("localhost:3000").unwrap();
                }
                command @ _  => panic!(
                    format!("Invalid command: {}", command))
            }
        }
        None => panic!("No command supplied"),
    }
}

fn get_records(sdb: Arc<Mutex<Connection>>, req: &mut Request) -> IronResult<Response> {
    let url = req.url.clone().into_generic_url();
    let mut name: Option<String> = None;
    if let Some(qp) = url.query_pairs() {
        for (key, value) in qp {
            match (&key[..], value) {
                ("name", n) => {
                    if let None = name {
                        name = Some(n);
                    } else {
                        return Ok(Response::with((status::BadRequest, "passed name in query more than once")));
                    }
                }
                _ => return Ok(Response::with((status::BadRequest, "unexpected query parameters"))),
            }
        }
    }

    let mut json_records;
    if let Ok(recs) = db::read(sdb, name.as_ref().map(|s| &s[..])) {
        use rustc_serialize::json;
        if let Ok(json) = json::encode(&recs) {
            json_records = Some(json);
        } else {
            return Ok(Response::with((status::InternalServerError, "couldn't convert records to JSON")));
        }
    } else {
        return Ok(Response::with((status::InternalServerError, "couldn't read records from database")));
    }
    let content_type = Mime(
        TopLevel::Application, SubLevel::Json, Vec::new());

    Ok(Response::with(
        (content_type, status::Ok, json_records.unwrap())))
}

fn get_record(sdb: Arc<Mutex<Connection>>, req: &mut Request) -> IronResult<Response> {
    let url = req.url.clone().into_generic_url();
    let path = url.path().unwrap();
    let sid: &str = &path.iter().last().unwrap();
    let id;
    if let Ok(r) = sid.parse() {
        id = r;
    } else {
        return Ok(Response::with((status::BadRequest, "bad id")));
    }

    let mut json_record;
    if let Ok(recs) = db::read_one(sdb, id) {
        if let Ok(json) = json::encode(&recs) {
            json_record = Some(json);
        } else {
            return Ok(Response::with((status::InternalServerError, "couldn't convert records to JSON")));
        }
    } else {
        return Ok(Response::with((status::InternalServerError, "couldn't read records from database")));
    }
    let content_type = Mime(
        TopLevel::Application, SubLevel::Json, Vec::new());

    Ok(Response::with(
        (content_type, status::Ok, json_record.unwrap())))
}

fn add_record(sdb: Arc<Mutex<Connection>>, req: &mut Request) -> IronResult<Response> {
    let mut body = String::new();
    req.body.read_to_string(&mut body).unwrap();
    let decoded: json::DecodeResult<db::Record> = json::decode(&body);
    if let Ok(record) = decoded {
        if record.name == "" || record.phone == "" {
            return Ok(Response::with((status::BadRequest, "empty name or phone")))
        }
        if let Ok(_) = db::insert(&*sdb.lock().unwrap(), &record.name, &record.phone) {
            Ok(Response::with((status::Created)))
        } else {
            Ok(Response::with((status::InternalServerError, "couldn't insert record")))
        }
    } else {
        return Ok(Response::with((status::BadRequest, "couldn't decode JSON")));
    }
}

fn update_record(sdb: Arc<Mutex<Connection>>, req: &mut Request) -> IronResult<Response> {
    let url = req.url.clone().into_generic_url();
    let path = url.path().unwrap();
    let sid: &str = &path.iter().last().unwrap();
    let id;
    if let Ok(r) = sid.parse() {
        id = r;
    } else {
        return Ok(Response::with((status::BadRequest, "bad id")));
    }

    let mut body = String::new();
    req.body.read_to_string(&mut body).unwrap();
    let decoded: json::DecodeResult<db::Record> = json::decode(&body);
    if let Ok(record) = decoded {
        if record.name == "" || record.phone == "" {
            return Ok(Response::with((status::BadRequest, "empty name or phone")))
        }
        if let Ok(_) = db::update(&*sdb.lock().unwrap(), id, &record.name, &record.phone) {
            Ok(Response::with((status::NoContent)))
        } else {
            Ok(Response::with((status::NotFound, "couldn't update record")))
        }
    } else {
        return Ok(Response::with((status::BadRequest, "couldn't decode JSON")));
    }
}


fn delete_record(sdb: Arc<Mutex<Connection>>, req: &mut Request) -> IronResult<Response> {
    let url = req.url.clone().into_generic_url();
    let path = url.path().unwrap();
    let sid: &str = &path.iter().last().unwrap();
    let id;
    if let Ok(r) = sid.parse() {
        id = r;
    } else {
        return Ok(Response::with((status::BadRequest, "bad id")));
    }

    if let Ok(_) = db::remove(&*sdb.lock().unwrap(), &[id]) {
        Ok(Response::with((status::NoContent)))
    } else {
        Ok(Response::with((status::NotFound, "couldn't update record")))
    }
}
