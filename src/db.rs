use postgres::{Connection};

pub fn insert(db: Connection, name: &str, phone: &str) -> ::postgres::Result<u64> {
    db.execute("INSERT INTO phonebook VALUES (default, $1, $2)", &[&name, &phone])
}

pub fn remove(db: Connection, ids: &[i32]) -> ::postgres::Result<u64> {
    let stmt = db.prepare("DELETE FROM phonebook WHERE id=$1").unwrap();
    for id in ids {
        try!(stmt.execute(&[id]));
    }
    Ok(0)
}

pub fn update(db: Connection, id: i32, name: &str, phone: &str)
          -> ::postgres::Result<()> {
    let tx: ::postgres::Transaction = db.transaction().unwrap();
    tx.execute(
        "UPDATE phonebook SET name = $1, phone = $2 WHERE id = $3",
        &[&name, &phone, &id]).unwrap();
    tx.set_commit();
    tx.finish()
}

pub fn show(db: Connection, arg: Option<&str>) -> ::postgres::Result<Vec<Record>> {
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
            id: row.get("id"),
            name: row.get("name"),
            phone: row.get("phone"),
        };
        results.push(record)
    }
    Ok(results)
}

pub struct Record {
    id: i32,
    name: String,
    phone: String,
}

pub fn format(rs: &[Record]) {
    let max = rs.iter().fold(
        0,
        |acc, ref item|
        if item.name.len() > acc { item.name.len() } else { acc });
    for v in rs {
        println!("{:3}   {:.*}   {}", v.id, max, v.name, v.phone);
    }
}
