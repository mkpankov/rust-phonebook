extern crate ini;
extern crate postgres;

use ini::Ini;
use postgres::{Connection, ConnectParams, ConnectTarget, SslMode, UserInfo};
use postgres::types::FromSql;
use postgres::rows::Row;

use std::str::FromStr;

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

    let mut args = std::env::args();
    match args.nth(1) {
        Some(text) => {
            match text.as_ref() {
                "add" => {
                    if args.len() != 4 {
                        panic!("Usage: phonebook add NAME PHONE");
                    }
                    let r = insert(
                        db,
                        &args.nth(2).unwrap(),
                        &args.nth(3).unwrap()
                            ).unwrap();
                    println!("{} rows affected", r);
                },
                "del" => {
                    if args.len() < 3 {
                        panic!("Usage: phonebook del ID...");
                    }
                    remove(
                        db,
                        args.skip(2).collect()
                            ).unwrap();
                },
                "edit" => {
                    if args.len() != 5 {
                        panic!("Usage: phonebook edit ID NAME PHONE");
                    }
                    update(
                        db,
                        &args.nth(2).unwrap(),
                        &args.nth(3).unwrap(),
                        &args.nth(4).unwrap()
                            ).unwrap();
                },
                "show" => {
                    if args.len() > 3 {
                        panic!("Usage: phonebook show [SUBSTRING]");
                    }
                    let s;
                    if args.len() == 3 {
                        s = args.nth(2);
                    } else {
                        s = None;
                    }
                    let r = show(db, s.as_ref().map(|s| &s[..])).unwrap();
                    format(&r);
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

fn insert(db: Connection, name: &str, phone: &str) -> postgres::Result<u64> {
    db.execute("INSERT INTO phonebook VALUES (default, $1, $2)", &[&name, &phone])
}

fn remove(db: Connection, ids: Vec<String>) -> postgres::Result<u64> {
    let stmt = db.prepare("DELETE FROM phonebook WHERE id=%1").unwrap();
    for id in ids {
        try!(stmt.execute(&[&id]));
    }
    Ok(0)
}

fn update(db: Connection, id: &str, name: &str, phone: &str)
          -> postgres::Result<()> {
    let tx: postgres::Transaction = db.transaction().unwrap();
    let _ = tx.execute(
        "UPDATE phonebook SET name = $1, phone = $2 WHERE id = $3",
        &[&name, &phone, &id]);
    tx.finish()
}

trait NamedRow<'a> {
    fn get_named<T>(&self, name: &str) -> T
        where T: FromSql;
}

impl<'a> NamedRow<'a> for Row<'a> {
    fn get_named<T>(&self, name: &str) -> T
        where T: FromSql
    {
        use postgres::Column;
        let columns = self.columns();
        for (i, n) in columns.iter().map(Column::name).enumerate() {
            if n == name {
                return self.get(i);
            }
        }
        panic!("Couldn't find column with given name");
    }
}

fn show(db: Connection, arg: Option<&str>) -> postgres::Result<Vec<Record>> {
    let s = match arg {
        Some(s) => format!("WHERE name LIKE '%{}'", s),
        None => "".to_owned(),
    };
    let stmt = db.prepare(
        &format!("SELECT * FROM phonebook {} ORDER BY id", s)
            ).unwrap();
    let rows = stmt.query(&[]).unwrap();
    let size = rows.iter().count();
    let mut results = Vec::with_capacity(size);
    for row in rows {
        let record = Record {
            id: row.get_named("id"),
            name: row.get_named("name"),
            phone: row.get_named("phone"),
        };
        results.push(record)
    }
    Ok(results)
}

struct Record {
    id: i64,
    name: String,
    phone: String,
}

fn format(rs: &[Record]) {
    let max = rs.iter().fold(
        0,
        |acc, ref item|
        if item.name.len() > acc { item.name.len() } else { acc });
    for v in rs {
        println!("{:3}   {:.*}   {}", v.id, max, v.name, v.phone);
    }
}
