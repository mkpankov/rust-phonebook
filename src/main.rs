extern crate postgres;
extern crate ini;

use postgres::{Connection, ConnectParams, ConnectTarget, SslMode, UserInfo};
use ini::Ini;

use std::str::FromStr;

struct Person {
    id: i32,
    name: String,
    data: Option<Vec<u8>>
}

fn params() -> (ConnectParams, SslMode) {
    let conf = Ini::load_from_file("conf.ini").unwrap();

    let host = conf.get("host").unwrap();
    let port = conf.get("port").unwrap();
    let sslmode = conf.get("sslmode").unwrap();
    let dbname = conf.get("dbname").unwrap();
    let user = conf.get("user").unwrap();
    let pass = conf.get("pass").unwrap();

    let s = match sslmode {
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
    let conn = Connection::connect("postgres://postgres:postgres@localhost", &SslMode::None)
            .unwrap();

    conn.execute("CREATE TABLE person (
                    id              SERIAL PRIMARY KEY,
                    name            VARCHAR NOT NULL,
                    data            BYTEA
                  )", &[]).unwrap();
    let me = Person {
        id: 0,
        name: "Steven".to_string(),
        data: None
    };
    conn.execute("INSERT INTO person (name, data) VALUES ($1, $2)",
                 &[&me.name, &me.data]).unwrap();

    let stmt = conn.prepare("SELECT id, name, data FROM person").unwrap();
    for row in stmt.query(&[]).unwrap() {
        let person = Person {
            id: row.get(0),
            name: row.get(1),
            data: row.get(2)
        };
        println!("Found person {}", person.name);
    }
}
