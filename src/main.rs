extern crate ini;
extern crate iron;
extern crate postgres;
extern crate router;
extern crate rustc_serialize;
extern crate url;

use ini::Ini;
use iron::*;
use postgres::{Connection, ConnectParams, ConnectTarget, SslMode, UserInfo};

use std::str::FromStr;
use std::sync::{Arc, Mutex};

mod db;
mod handlers;

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
        target: ConnectTarget::Tcp(host.clone()),
        port: Some(FromStr::from_str(port).ok().unwrap()),
        user: Some(UserInfo {
            user: user.clone(),
            password: Some(pass.clone()),
        }),
        database: Some(dbname.clone()),
        options: vec![],
    }, s)
}

fn init_db(db: &Connection) {
    db.execute(
        concat!(r#"CREATE TABLE IF NOT EXISTS phonebook"#,
                r#"("id" SERIAL PRIMARY KEY, "name" varchar(50),"#,
                r#" "phone" varchar(100))"#,
                ),
        &[])
        .unwrap();
}

fn add(db: &Connection, args: &Vec<String>) {
    if args.len() != 4 {
        panic!("Usage: phonebook add NAME PHONE");
    }
    let r = db::insert(&db, &args[2], &args[3])
        .unwrap();
    println!("{} rows affected", r);
}

fn del(db: &Connection, args: &Vec<String>) {
    if args.len() < 3 {
        panic!("Usage: phonebook del ID...");
    }
    let ids: Vec<i32> = args[2..].iter()
        .map(|s| s.parse().unwrap())
        .collect();

    db::remove(&db, &ids)
        .unwrap();
}

fn edit(db: &Connection, args: &Vec<String>) {
    if args.len() != 5 {
        panic!("Usage: phonebook edit ID NAME PHONE");
    }
    let id = args[2].parse().unwrap();
    db::update(&db, id, &args[3], &args[4])
        .unwrap();
}

fn show(db: &Connection, args: &Vec<String>) {
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
}

macro_rules! clone_pass_bound {
    ($arc:ident, $stmt:stmt) => {
        {
            let $arc = $arc.clone();
            $stmt;
        }
    }
}

macro_rules! define_handler {
    ($connection:ident, $router: ident.$method:ident, $route:expr,
     $handler:path) => {
        clone_pass_bound!(
            $connection,
            $router.$method(
                $route,
                move |req: &mut Request|
                $handler(&*$connection, req)));
    }
}

fn serve(db: Connection) {
    let sdb = Arc::new(Mutex::new(db));
    let mut router = router::Router::new();
    define_handler!(sdb, router.get, "/api/v1/records", handlers::get_records);

    define_handler!(sdb, router.get, "/api/v1/records/:id",
                    handlers::get_record);

    define_handler!(sdb, router.post, "/api/v1/records", handlers::add_record);

    define_handler!(sdb, router.put, "/api/v1/records/:id",
                    handlers::update_record);

    define_handler!(sdb, router.delete, "/api/v1/records/:id",
                    handlers::delete_record);

    Iron::new(router).http("localhost:3000").unwrap();
}

fn main() {
    let (params, sslmode) = params();
    let db = Connection::connect(params, &sslmode).unwrap();

    init_db(&db);

    let args: Vec<String> = std::env::args().collect();

    match args.get(1) {
        Some(text) => {
            match text.as_ref() {
                "add" => add(&db, &args),
                "del" => del(&db, &args),
                "edit" => edit(&db, &args),
                "show" => show(&db, &args),
                "help" => println!("{}", HELP),
                "serve" => serve(db),
                command => panic!(
                    format!("Invalid command: {}", command))
            }
        }
        None => panic!("No command supplied"),
    }
}
