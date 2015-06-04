extern crate ini;
extern crate postgres;

use ini::Ini;
use postgres::{Connection, ConnectParams, ConnectTarget, SslMode, UserInfo};

use std::str::FromStr;

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
                    let r = db::insert(db, &args[2], &args[3])
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

                    db::remove(db, &ids)
                        .unwrap();
                },
                "edit" => {
                    if args.len() != 5 {
                        panic!("Usage: phonebook edit ID NAME PHONE");
                    }
                    let id = args[2].parse().unwrap();
                    db::update(db, id, &args[3], &args[4])
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
                    let r = db::show(db, s.as_ref().map(|s| &s[..])).unwrap();
                    db::format(&r);
                },
                "help" => {
                    println!("{}", HELP);
                },
                command @ _  => panic!(
                    format!("Invalid command: {}", command))
            }
        }
        None => panic!("No command supplied"),
    }
}
