use tokio_postgres::{Client, NoTls, Error as PgError};
use async_trait::async_trait;

pub struct Contact {
    pub id: i32,
    pub firstname: String,
    pub lastname: String,
    pub phone: String,
    pub email: String
}

#[async_trait]
pub trait Repository {
    async fn new(dsl: &str) -> Self;
    async fn get(&self, id: i32) -> Result<Contact, Error>;
}

pub struct PgsqlRepository {
    client: Client
}

#[derive(Debug)]
pub enum Error {
    Db(PgError),
    Intern(String),
}

impl From<PgError> for Error {
    fn from(err: PgError) -> Error {
        Error::Db(err)
    }
}

impl From<String> for Error {
    fn from(err: String) -> Error {
        Error::Intern(err)
    }
}

#[async_trait]
impl Repository for PgsqlRepository {
    async fn new(dsn: &str) -> Self {
        let (client, connection) = tokio_postgres::connect(dsn, NoTls).await.unwrap();

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        Self { client }
    }

    async fn get(&self, id: i32) -> Result<Contact, Error> {
        let rows = self.client.query("SELECT id, firstname, lastname, phone, email FROM contact WHERE id=$1", &[&id]).await?;
        if rows.len() == 0 {
            Err(Error::Intern(format!("no record with id {}", id)))
        } else {
            let id = rows[0].get(0);
            let firstname = rows[0].get(1);
            let lastname = rows[0].get(2);
            let phone = rows[0].get(3);
            let email = rows[0].get(4);

            Ok(Contact {
                id,
                firstname,
                lastname,
                phone,
                email
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::repository::{PgsqlRepository, Repository};

    #[tokio::test]
    async fn get_contact_no_contact() {
        let repository = PgsqlRepository::new("host=postgresql user=test password=test dbname=test").await;
        assert!(repository.get(12).await.is_err(), "no results should be found")
    }
}