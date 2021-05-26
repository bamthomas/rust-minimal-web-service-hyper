use tokio_postgres::{Client, NoTls, Error as PgError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
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
    async fn save(&self, contact: &Contact) -> Result<u64, Error>;
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
        let row = self.client.query_one("SELECT id, firstname, lastname, phone, email FROM contact WHERE id=$1", &[&id]).await?;
        let row_firstname : Option<String> =  row.get(1);
        let row_phone : Option<String> =  row.get(3);
        let row_email : Option<String> =  row.get(4);
        Ok(Contact {
            id: row.get(0),
            firstname: row_firstname.unwrap_or(String::from("")),
            lastname: row.get(2),
            phone: row_phone.unwrap_or(String::from("")),
            email: row_email.unwrap_or(String::from("")),
        })
        
    }

    async fn save(&self, contact: &Contact) -> Result<u64, Error> {
        Ok(self.client.execute("INSERT INTO contact (id, firstname, lastname, phone, email) VALUES ($1, $2, $3, $4, $5)",
                            &[&contact.id, &contact.firstname, &contact.lastname, &contact.phone, &contact.email]).await?)
    }
}

#[cfg(test)]
mod tests {
    use crate::repository::{PgsqlRepository, Repository, Contact};
    use test_context::{test_context, AsyncTestContext};
    use tokio_postgres::{NoTls};
    use async_trait::async_trait;

    struct PgContext { repository: PgsqlRepository }

    #[async_trait]
    impl AsyncTestContext for PgContext {
        async fn setup() -> PgContext {
            let (client, connection) = tokio_postgres::connect("host=postgresql user=test password=test dbname=test", NoTls).await.unwrap();

            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    eprintln!("connection error: {}", e);
                }
            });
            PgContext {  repository: PgsqlRepository{ client } }
        }

        async fn teardown(self) {
            self.repository.client.execute("DELETE FROM contact", &[]).await.unwrap();
        }
    }

    #[test_context(PgContext)]
    #[tokio::test]
    async fn get_contact_no_contact(ctx: &PgContext) {
        assert!(ctx.repository.get(12).await.is_err(), "no results should be found")
    }

    #[test_context(PgContext)]
    #[tokio::test]
    async fn save_get_contact(ctx: &PgContext) {
        let contact = Contact {
            id: 13,
            firstname: "first".to_string(),
            lastname: "second".to_string(),
            phone: "0123456789".to_string(),
            email: "e@mail.com".to_string()
        };
        assert!(ctx.repository.save(&contact).await.is_ok(), "save should succeed");
        assert!(ctx.repository.get(13).await.is_ok(), "contact should be found")
    }

    #[test_context(PgContext)]
    #[tokio::test]
    async fn save_get_contact_with_empty_fields(ctx: &PgContext) {
        ctx.repository.client.execute("INSERT INTO contact (id, lastname) VALUES ($1,$2)", &[&14, &"foo"],).await.unwrap();
        let contact = match ctx.repository.get(14).await {
            Ok(contact) => contact,
            Err(error) => {panic!("error : {:?}", error)},
        };
        assert_eq!(contact.id, 14);
        assert_eq!(contact.firstname, String::from(""));
        assert_eq!(contact.lastname, "foo");
        assert_eq!(contact.phone, String::from(""));
        assert_eq!(contact.email, String::from(""))
    }
}