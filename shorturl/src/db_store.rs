use std::fmt;

use rusqlite::{params, types::ToSqlOutput, Connection, Result};

use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

use log::error;

use crate::types::{AccessLog, Meta, MetaType, ShortUrlMapping};

pub struct Store {
    conn: Connection,
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
        // main table
        tx.execute(
            "
            CREATE TABLE IF NOT EXISTS
                short_urls (
                    id INTEGER primary key,
                    short_code text NOT NULL,
                    long_url text NOT NULL,
                    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    active BOOLEAN DEFAULT true
                )
            ",
            (),
        )?;
        // meta type definition
        tx.execute(
            "
            CREATE TABLE IF NOT EXISTS
                meta_type (
                    id INTEGER PRIMARY KEY,
                    description text NOT NULL
                )",
            (),
        )?;
        for meta_type in [MetaType::Create, MetaType::Access] {
            tx.execute(
                "INSERT OR IGNORE INTO meta_type(id, description) VALUES(?1, ?2)",
                params![meta_type, meta_type.to_string()],
            )?;
        }
        // store meta data
        tx.execute(
            "
            CREATE TABLE IF NOT EXISTS
                access_meta (
                    meta_type integer NOT NULL,
                    short_code text NOT NULL,
                    short_code_id INTEGER NULL,
                    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    address text,
                    header text,
                    FOREIGN KEY(short_code_id) REFERENCES short_urls(id)
                    FOREIGN KEY(meta_type) REFERENCES meta_type(id)
                )
            ",
            (),
        )?;
        // store api key
        tx.execute(
            "
            CREATE TABLE IF NOT EXISTS
                api_keys (
                    uid INTEGER NOT NULL,
                    api_key text NOT NULL,
                    PRIMARY KEY (uid, api_key)
                )
            ",
            (),
        )?;

        tx.commit()?;

        Ok(Store { conn })
    }

    pub fn insert(&mut self, short_code: &str, long_url: &str, meta: &Meta) -> Result<()> {
        let tx = self.conn.transaction()?;
        match Store::_get(&tx, short_code, meta, false) {
            Some(_) => {
                return Err(rusqlite::Error::InvalidParameterName(
                    "short code exists".to_string(),
                ))
            }
            None => (),
        };

        tx.execute(
            "INSERT INTO
                short_urls (short_code, long_url)
             VALUES
                (?1, ?2)",
            params![short_code, long_url],
        )?;
        // store meta data
        Store::accessed(&tx, &short_code, meta, &MetaType::Create, true);

        tx.commit()
    }

    pub fn get(&mut self, short_code: &str, meta: &Meta) -> Option<String> {
        Store::_get(&self.conn, short_code, meta, true)
    }

    fn _get(conn: &Connection, short_code: &str, meta: &Meta, log: bool) -> Option<String> {
        let result: Option<String>;
        let mut succeed = false;
        {
            let mut stmt = conn
                .prepare(
                    "SELECT
                        long_url
                    FROM
                        short_urls
                    WHERE
                        active = true
                    AND
                        short_code = :short_code",
                )
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

        if log {
            Store::accessed(&conn, short_code, meta, &MetaType::Access, succeed);
        }

        return result;
    }

    pub fn get_all(&mut self) -> Result<Vec<ShortUrlMapping>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT
                    short_code, long_url
                FROM
                    short_urls
                WHERE
                active = true
                    ",
            )
            .unwrap();

        Ok(stmt
            .query_map((), |row| {
                Ok(ShortUrlMapping {
                    short_code: row.get(0).unwrap(),
                    url: row.get(1).unwrap(),
                })
            })
            .unwrap()
            .map(|x| x.unwrap())
            .collect())
    }

    pub fn get_summarised_access_logs(&mut self) -> Result<Vec<AccessLog>> {
        let mut stmt = self
            .conn
            .prepare(
                "
            SELECT
                am.short_code,
                su.long_url,
                COUNT(case am.meta_type when :accessed_meta_type then 1 else null end)
                    as count,
                max(case am.meta_type when :accessed_meta_type then am.created_at else null end)
                    as last_access
            FROM
                access_meta AS am
            LEFT JOIN
                short_urls AS su
            ON
                am.short_code = su.short_code
            GROUP BY
                am.short_code
            ",
            )
            // TODO: this query is not exactly correct when there are historical repeating non-active short-url
            .unwrap();
        let meta_list: Vec<_> = stmt
            .query_map(
                &[(":accessed_meta_type", &(MetaType::Access as u8).to_string())],
                |row| {
                    Ok(AccessLog {
                        code: row.get(0).unwrap(),
                        url: row.get(1).unwrap(),
                        access_count: row.get(2).unwrap(),
                        last_access: row.get(3).unwrap(),
                    })
                },
            )
            .unwrap()
            .map(|x| x.unwrap())
            .collect();
        return Ok(meta_list);
    }

    fn accessed(
        conn: &Connection,
        short_code: &str,
        meta: &Meta,
        access_type: &MetaType,
        succeed: bool,
    ) {
        let mut short_code_id: Option<i32> = None;
        if succeed {
            short_code_id = match conn.query_row(
                "
            SELECT
                id
            FROM
                short_urls
            WHERE
                short_code = ?1
            AND
                active = true
                ",
                [short_code],
                |row| row.get(0),
            ) {
                Ok(val) => Some(val),
                Err(e) => {
                    error!("{}", e);
                    None
                }
            };
        }
        match conn.execute(
            "INSERT INTO
                access_meta (short_code, short_code_id, meta_type, address, header)
            VALUES
                (?1, ?2, ?3, ?4, ?5)",
            params![
                short_code,
                short_code_id,
                access_type,
                meta.address,
                meta.header
            ],
        ) {
            Ok(_) => (),
            Err(e) => error!("{}", e),
        }
    }

    pub fn remove(&mut self, short_code: &str) -> Result<i32> {
        self.conn
            .execute(
                "
            UPDATE
                short_urls
            SET
                active = false
            WHERE
                short_code = ?1",
                params![short_code],
            )
            .unwrap();
        match self
            .conn
            .query_row("SELECT changes()", (), |row| row.get(0))
        {
            Ok(val) => Ok(val),
            Err(e) => Err(e),
        }
    }

    pub fn create_api_key(&mut self, uid: i32) -> Result<String> {
        let rand_api_key: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(30)
            .map(char::from)
            .collect();

        match self.conn.execute(
            "INSERT INTO
                api_keys (uid, api_key)
            VALUES
                (?1, ?2)",
            params![uid, rand_api_key],
        ) {
            Ok(_) => Ok(rand_api_key),
            Err(e) => Err(e),
        }
    }

    pub fn list_api_key(&mut self, uid: i32) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare(
                "
            SELECT
                api_key
            FROM
                api_keys
            WHERE
                uid = :uid",
            )
            .unwrap();
        return Ok(stmt
            .query_map(&[(":uid", &uid)], |row| Ok(row.get(0).unwrap()))
            .unwrap()
            .map(|x| x.unwrap())
            .collect());
    }

    pub fn check_api_key(&mut self, uid: i32, api_key: &str) -> bool {
        let mut stmt = self
            .conn
            .prepare(
                "
            SELECT
                api_key
            FROM
                api_keys
            WHERE
                api_key = :api_key
            AND
                uid = :uid
                ",
            )
            .unwrap();
        // some value exists
        return match stmt
            .query_map(
                &[
                    (":uid", &uid.to_string()),
                    (":api_key", &api_key.to_string()),
                ],
                |row| Ok(row.get(0).unwrap()),
            )
            .unwrap()
            .map(|x: Result<String>| x.unwrap())
            .next()
        {
            Some(_) => true,
            _ => false,
        };
    }

    pub fn has_api_key(&mut self, uid: i32) -> bool {
        let result: Result<i32> = self.conn.query_row(
            "
            SELECT
                COUNT(*)
            FROM
                api_keys
            WHERE
                uid = :uid
                ",
            &[(":uid", &uid.to_string())],
            |row| row.get(0),
        );
        match result {
            Ok(val) => val > 0,
            _ => false,
        }
    }
}
