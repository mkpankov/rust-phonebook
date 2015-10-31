use postgres::Connection;

use std::sync::Mutex;

pub fn insert(db: &Connection, name: &str, phone: &str) -> ::postgres::Result<u64> {
    db.execute("INSERT INTO phonebook VALUES (default, $1, $2)",
               &[&name, &phone])
}

pub fn remove(db: &Connection, ids: &[i32]) -> ::postgres::Result<u64> {
    let stmt = db.prepare("DELETE FROM phonebook WHERE id=$1").unwrap();
    for id in ids {
        try!(stmt.execute(&[id]));
    }
    Ok(0)
}

pub fn update(db: &Connection, id: i32, name: &str, phone: &str) -> ::postgres::Result<()> {
    let tx: ::postgres::Transaction = db.transaction().unwrap();
    tx.execute("UPDATE phonebook SET name = $1, phone = $2 WHERE id = $3",
               &[&name, &phone, &id])
      .unwrap();
    tx.set_commit();
    tx.finish()
}

pub fn show(db: &Connection, arg: Option<&str>) -> ::postgres::Result<Vec<Record>> {
    let s = match arg {
        Some(s) => format!("WHERE name LIKE '%{}%'", s),
        None => "".to_owned(),
    };
    let stmt = db.prepare(&format!("SELECT * FROM phonebook {} ORDER BY id", s))
                 .unwrap();
    let rows = stmt.query(&[]).unwrap();
    let size = rows.iter().count();
    let mut results = Vec::with_capacity(size);
    for row in rows {
        let record = Record {
            id: row.get("id"),
            name: row.get("name"),
            phone: row.get("phone"),
        };
        results.push(record)
    }
    Ok(results)
}

#[derive(RustcEncodable, RustcDecodable)]
pub struct Record {
    id: Option<i32>,
    pub name: String,
    pub phone: String,
}

pub fn format(rs: &[Record]) {
    let max = rs.iter().fold(0, |acc, ref item| {
        if item.name.chars().count() > acc {
            item.name.chars().count()
        } else {
            acc
        }
    });
    for v in rs {
        println!("{0:3?}   {1:2$}   {3}", v.id.unwrap(), v.name, max, v.phone);
    }
}

pub fn read(sdb: &Mutex<Connection>, name: Option<&str>) -> Result<Vec<Record>, ()> {
    if let Ok(rs) = show(&*sdb.lock().unwrap(), name) {
        Ok(rs)
    } else {
        Err(())
    }
}

pub fn read_one(sdb: &Mutex<Connection>, id: i32) -> Result<Record, ()> {
    let db = &*sdb.lock().unwrap();
    let stmt = db.prepare("SELECT * FROM phonebook WHERE id = $1")
                 .unwrap();
    if let Ok(rows) = stmt.query(&[&id]) {
        let mut iter = rows.iter();
        if iter.len() != 1 {
            return Err(());
        }
        let row = iter.next().unwrap();
        let record = Record {
            id: row.get("id"),
            name: row.get("name"),
            phone: row.get("phone"),
        };

        Ok(record)
    } else {
        Err(())
    }
}
