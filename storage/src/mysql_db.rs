use std::error::Error;

use mysql::Pool;

#[derive(Debug, Clone)]
pub struct MysqlDB {
    pool: Pool,
}

impl MysqlDB {
    pub fn new(url: &str) -> Self {
        let pool = Pool::new(url).expect("Connect mysql error!");
        Self { pool }
    }

    pub fn pool(&self) -> &Pool {
        &self.pool
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Payment {
    customer_id: i32,
    amount: i32,
    account_name: Option<String>,
}

#[cfg(test)]
pub mod mysql_db_tests {
    use mysql::{params, prelude::Queryable};

    use super::{MysqlDB, Payment};

    #[test]
    pub fn read_and_write_test() {
        let db = MysqlDB::new("mysql://root:root@localhost:3306/bft_diagnosis");
        let mut conn = db.pool.get_conn().expect("Get Conn Error!");

        conn.query_drop(
            r"CREATE TABLE testtable (
                id int auto_increment primary key,
                customer_id int not null,
                amount int not null,
                account_name text
            )",
        )
        .expect("Create table error!");

        

        let payments = vec![
            Payment {
                customer_id: 1,
                amount: 2,
                account_name: None,
            },
            Payment {
                customer_id: 3,
                amount: 4,
                account_name: Some("foo".into()), 
            },
            Payment {
                customer_id: 5,
                amount: 6,
                account_name: None,
            },
            Payment {
                customer_id: 7,
                amount: 8,
                account_name: None,
            },
            Payment {
                customer_id: 9,
                amount: 10,
                account_name: Some("bar".into()),
            }, 
        ];

        conn.exec_batch(
            r"INSERT INTO testtable (customer_id, amount, account_name)
        VALUES (:customer_id, :amount, :account_name)",
            payments.iter().map(|p| {
                params! {
                    "customer_id" => p.customer_id,
                    "amount" => p.amount,
                    "account_name" => &p.account_name,
                }
            }),
        )
        .expect("Insert value error!");

        let query_result = conn.query_map(
            "SELECT customer_id, amount, account_name from testtable limit 0,8",
            |(customer_id, amount, account_name)| Payment {
                customer_id,
                amount,
                account_name,
            },
        )
        .expect("Query error!");

        println!("{:?}", query_result);

        conn.exec_drop("DROP TABLE testtable", ()).expect("DROP Table Error!");

        let sql = format!(
            "SELECT round, peer_id, throughput FROM scalability_throughput_results WHERE item = 'Scalability(2, 3, 1)' LIMIT {},{}",
            0, 10
        );

        let results: Vec<(u16, String, u64)> = conn
            .query_map(sql, |(round, peer_id, throughput)| {
                (round, peer_id, throughput)
            })
            .unwrap();
        
            println!("{:?}", results);

    }
}
