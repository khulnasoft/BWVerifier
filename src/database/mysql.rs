use crate::database::DatabaseInterface;
use crate::verification::Messages;
use mysql::params;
use mysql::prelude::Queryable;
use mysql::{Params, Pool, PooledConn};
use std::collections::HashMap;
use std::thread::sleep;
use std::time::Duration;

#[derive(Debug)]
pub struct Mysql {}
impl Mysql {
    fn get_client(&self) -> Option<PooledConn> {
        if let Ok(pool) =
            Pool::new("mysql://benchmarkdbuser:benchmarkdbpass@bw-database/hello_world")
        {
            if let Ok(conn) = pool.get_conn() {
                Some(conn)
            } else {
                None
            }
        } else {
            None
        }
    }

    fn run_counting_query(&self, query: &str) -> u32 {
        if let Some(mut client) = self.get_client() {
            if let Ok(rows) = client.query_map(query, |(_var_name, value): (String, u32)| {
                (_var_name, value)
            }) {
                let mut sum = 0;
                for row in rows {
                    sum += row.1;
                }
                return sum;
            }
        }

        0
    }

    fn get_rows_updated(&self) -> u32 {
        self.run_counting_query(r"SELECT variable_name, variable_value from PERFORMANCE_SCHEMA.SESSION_STATUS where Variable_name = 'Innodb_rows_updated'")
    }
}
impl DatabaseInterface for Mysql {
    fn wait_for_database_to_be_available(&self) {
        let mut messages = Messages::default();
        let max = 60;
        let mut slept = 0;
        while slept < max {
            if self.get_client().is_some() {
                return;
            }

            sleep(Duration::from_secs(1));
            slept += 1;
        }
        messages.error(
            format!(
                "Database connection could not be established after {} seconds.",
                max
            ),
            "Database unavailable",
        );
    }

    fn get_all_from_world_table(&self) -> HashMap<i32, i32> {
        let mut to_ret = HashMap::new();
        if let Some(mut client) = self.get_client() {
            if let Ok(rows) = client
                .query_map("SELECT * FROM world", |(id, randomnumber): (i32, i32)| {
                    (id, randomnumber)
                })
            {
                for row in rows {
                    to_ret.insert(row.0, row.1);
                }
            }
        }

        to_ret
    }

    fn insert_one_thousand_fortunes(&self) {
        if let Some(mut client) = self.get_client() {
            let params = vec![Params::Empty; 1000];
            let mut index = 12;
            let func = |_| {
                index += 1;
                params! {
                    "id" => index,
                    "fortune" => "フレームワークのベンチマーク",
                }
            };
            if client
                .exec_batch(
                    r"INSERT INTO fortune(id,message) VALUES (:id,:fortune)",
                    params.iter().map(func),
                )
                .is_ok()
            {
                // Deliberately left empty. In the case of success, then great
                // we can exit this function and the assumption is that the DB
                // has the correct number of rows. If not, then the verifier
                // will fail because the *correct* number of rows is never
                // returned to the application, which will never correctly
                // verify with the verifier.
            }
        }
    }

    fn get_count_of_all_queries_for_table(&self, _table_name: &str) -> u32 {
        let selects =
            self.run_counting_query(r"Show global status where Variable_name = 'Com_select'");
        let updates =
            self.run_counting_query(r"Show global status where Variable_name = 'Com_update'");

        // Note: this is given the 1.5% margin just as in
        // `get_count_of_rows_updated_for_table`.
        (updates as f64 * 1.015) as u32 + selects
    }

    fn get_count_of_rows_selected_for_table(
        &self,
        _table_name: &str,
        _expected_rows_per_query: u32,
    ) -> u32 {
        let rows_read = self.run_counting_query(r"SELECT variable_name, variable_value from PERFORMANCE_SCHEMA.SESSION_STATUS where Variable_name = 'Innodb_rows_read'");
        // Note: we explicitly do not call `get_count_of_rows_updated_for_table`
        // here because we are going to subtract the rows updated from the rows
        // read. The first value is both accurate and precise; the second is
        // known to by *lower* for MySQL (see the documentation for said
        // function) which means that this *should* guarantee "enough" rows
        // were selected.
        let rows_updated = self.get_rows_updated();

        rows_read - rows_updated
    }

    /// Note: This function is given a margin of 1.5% for MySQL for rows
    /// updated because when MySQL issues an `update` statement that
    /// *would* have done an identity update, it is treated as a no-op and
    /// the query is discarded and therefore the rows updated count that
    /// MySQL reports is **NOT** incremented.
    ///
    /// Example:
    /// ```sql
    /// UPDATE world SET randomnumber = 15 WHERE id = 100;
    /// UPDATE world SET randomnumber = 15 WHERE id = 100;
    /// ```
    /// What is interesting is that even though the `update` is dropped,
    /// **A** query is still run as a part of the check, so
    /// `get_count_of_all_queries_for_table` still returns the correct
    /// number even when several of these no-op `updates` are dropped.
    fn get_count_of_rows_updated_for_table(
        &self,
        _table_name: &str,
        _expected_rows_per_query: u32,
    ) -> u32 {
        let count = self.get_rows_updated();

        (count as f64 * 1.015) as u32
    }
}
