use iron::*;
use iron::mime::{Mime, TopLevel, SubLevel};
use postgres::Connection;
use rustc_serialize::json;

use std::io::Read;
use std::sync::{Arc, Mutex};

pub fn get_records(sdb: Arc<Mutex<Connection>>, req: &mut Request) -> IronResult<Response> {
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

    let json_records;
    if let Ok(recs) = ::db::read(sdb, name.as_ref().map(|s| &s[..])) {
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

pub fn get_record(sdb: Arc<Mutex<Connection>>, req: &mut Request) -> IronResult<Response> {
    let url = req.url.clone().into_generic_url();
    let path = url.path().unwrap();
    let sid: &str = &path.iter().last().unwrap();
    let id;
    if let Ok(r) = sid.parse() {
        id = r;
    } else {
        return Ok(Response::with((status::BadRequest, "bad id")));
    }

    let json_record;
    if let Ok(recs) = ::db::read_one(sdb, id) {
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

pub fn add_record(sdb: Arc<Mutex<Connection>>, req: &mut Request) -> IronResult<Response> {
    let mut body = String::new();
    req.body.read_to_string(&mut body).unwrap();
    let decoded: json::DecodeResult<::db::Record> = json::decode(&body);
    if let Ok(record) = decoded {
        if record.name == "" || record.phone == "" {
            return Ok(Response::with((status::BadRequest, "empty name or phone")))
        }
        if let Ok(_) = ::db::insert(&*sdb.lock().unwrap(), &record.name, &record.phone) {
            Ok(Response::with((status::Created)))
        } else {
            Ok(Response::with((status::InternalServerError, "couldn't insert record")))
        }
    } else {
        return Ok(Response::with((status::BadRequest, "couldn't decode JSON")));
    }
}

pub fn update_record(sdb: Arc<Mutex<Connection>>, req: &mut Request) -> IronResult<Response> {
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
    let decoded: json::DecodeResult<::db::Record> = json::decode(&body);
    if let Ok(record) = decoded {
        if record.name == "" || record.phone == "" {
            return Ok(Response::with((status::BadRequest, "empty name or phone")))
        }
        if let Ok(_) = ::db::update(&*sdb.lock().unwrap(), id, &record.name, &record.phone) {
            Ok(Response::with((status::NoContent)))
        } else {
            Ok(Response::with((status::NotFound, "couldn't update record")))
        }
    } else {
        return Ok(Response::with((status::BadRequest, "couldn't decode JSON")));
    }
}


pub fn delete_record(sdb: Arc<Mutex<Connection>>, req: &mut Request) -> IronResult<Response> {
    let url = req.url.clone().into_generic_url();
    let path = url.path().unwrap();
    let sid: &str = &path.iter().last().unwrap();
    let id;
    if let Ok(r) = sid.parse() {
        id = r;
    } else {
        return Ok(Response::with((status::BadRequest, "bad id")));
    }

    if let Ok(_) = ::db::remove(&*sdb.lock().unwrap(), &[id]) {
        Ok(Response::with((status::NoContent)))
    } else {
        Ok(Response::with((status::NotFound, "couldn't update record")))
    }
}
