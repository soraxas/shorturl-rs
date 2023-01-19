use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

use rusqlite::{Connection, params, Result, types::ToSqlOutput};

use log::{error, info, warn};
use crate::url_mapping::Meta;

type Items = HashMap<String, String>;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Id {
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Item {
    pub short: String,
    pub long_url: String,
}

pub struct Store {
    conn: Connection,
}

#[derive(Debug, Copy, Clone)]
enum MetaType {
    Create = 1,
    Access = 2,
}

impl rusqlite::ToSql for MetaType {
    fn to_sql(&self) -> Result<ToSqlOutput> {
        Ok(ToSqlOutput::from(*self as u8))
    }
}

impl fmt::Display for MetaType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
        // or, alternatively:
        // fmt::Debug::fmt(self, f)
    }
}

impl Store {
    pub fn new() -> Result<Self> {
        // initialise database
        let mut conn = Connection::open("urls.db")?;

        let tx = conn.transaction()?;

        tx.execute(
            "create table if not exists short_urls (
                short_code text not null unique primary key,
                long_url text not null,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            (),
        )?;

        tx.execute(
            "create table if not exists meta_type (
                id INTEGER PRIMARY KEY,
                description text not null
            )",
            (),
        )?;

        for meta_type in [MetaType::Create, MetaType::Access] {
            tx.execute(
                "INSERT OR IGNORE INTO meta_type(id, description) VALUES(?1, ?2)",
                params![meta_type, meta_type.to_string()],
            )?;
        }

        tx.execute(
            "create table if not exists access_meta (
                meta_type integer not null,
                short_code text not null,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                address text,
                header text,
                succeed boolean,
                FOREIGN KEY(short_code) REFERENCES short_urls(short_code)
                FOREIGN KEY(meta_type) REFERENCES meta_type(id)
            )",
            (),
        )?;

        tx.commit();

        Ok(Store { conn })
    }

    pub fn put(
        &mut self,
        short_code: String,
        long_url: String,
        meta: &Meta,
    ) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO short_urls (short_code, long_url) values (?1, ?2)",
            params![short_code, long_url],
        )?;

        Store::accessed(&tx, &short_code, meta, &MetaType::Create, true);
        //
        //
        // tx.execute("INSERT INTO access_meta (short_code, meta_type, address, header) \
        //         values (?1, ?2, ?3, ?4)",
        //            params![short_code, MetaType::Create, meta.address, meta.header])?;

        tx.commit()
    }

    pub fn get(&mut self, short_code: &str, meta: &Meta) -> Option<String> {
        let result: Option<String>;
        let mut succeed = false;
        {
            let mut stmt = self
                .conn
                .prepare(
                    "SELECT long_url FROM short_urls \
                    WHERE short_code = :short_code")
                .unwrap();
            let mut rows = stmt
                .query_map(&[(":short_code", &short_code)], |row| row.get(0))
                .unwrap();

            result = match rows.next() {
                Some(val) => {
                    succeed = true;
                    Some(val.unwrap())
                }
                None => None,
            };
        }

        Store::accessed(&self.conn, short_code, meta, &MetaType::Access, succeed);

        return result;
    }

    pub fn get_all(&mut self) -> Option<String> {
        // let mut conn = Connection::open("urls.db").unwrap();

        // let mut stmt = conn
        //     .prepare("SELECT * FROM short_urls WHERE short_code = :short_code")
        //     .unwrap();
        // let rows = stmt
        //     .query_map(&[(":short_code", &short_code)], |row| row.get(0))
        //     .unwrap();

        // for name_result in rows {
        //     return first result
        //     // long_urls.push(name_result?);
        // }

        // // if (long_urls.len() > 0) {
        // //     return Ok(Some(long_urls[0]))
        // // }

        return None;
    }


    fn accessed(conn: &Connection, short_code: &str, meta: &Meta, access_type: &MetaType,
                succeed: bool) {
        match conn.execute(
            "INSERT INTO access_meta (short_code, meta_type, address, header, succeed) \
                values (?1, ?2, ?3, ?4, ?5)",
            params![short_code, access_type, meta.address, meta.header, succeed],
        ) {
            Ok(_) => (),
            Err(e) => error!("{}", e),
        }
    }

    pub fn remove(&mut self, short_code: String) -> Result<()> {
        let tx = self.conn.transaction()?;

        tx.execute(
            "DELETE FROM artists_backup WHERE artistid = >1",
            &[&short_code.to_string()],
        )?;

        tx.commit()
    }
}
