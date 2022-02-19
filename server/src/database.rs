use mongodb::{
    bson::doc,
    error::Error,
    options::{ClientOptions, Credential},
    Client, Database,
};
use shared::model::UserCredentials;

use crate::user::User;
type Uuid = String;

pub const LIVE: &'static str = "live";

#[derive(Debug, PartialEq)]
pub enum DatabaseError
{
    UserAlreadyExist,
    UserDontExist,
    DbError,
}
pub type DatabaseResult<T> = Result<T, DatabaseError>;


pub async fn connect() -> Result<Client, Error>
{
    let mut client_options = ClientOptions::parse("mongodb://localhost:27017").await?;
    client_options.app_name = Some("My App".to_string());

    client_options.credential = Some(
        Credential::builder()
            .username(Some("root".to_string()))
            .password(Some("rootpassword".to_string()))
            .build(),
    );

    Client::with_options(client_options)
}


pub async fn register_user(db: Database, cred: UserCredentials) -> DatabaseResult<Uuid>
{
    let col = db.collection::<User>("users");
    let user = User::from_cred(cred);

    // Check if user with same name exists
    let filter = doc! { "name": user.name.as_str() };
    match col.find_one(filter, None).await
    {
        Ok(Some(_)) => Err(DatabaseError::UserAlreadyExist),
        Ok(None) =>
        {
            if col.insert_one(&user, None).await.is_err()
            {
                Err(DatabaseError::DbError)
            }
            else
            {
                Ok(user.uuid)
            }
        },
        _ => Err(DatabaseError::DbError),
    }
}

pub async fn find_user_by_uuid(db: Database, uuid: Uuid) -> DatabaseResult<User>
{
    let col = db.collection::<User>("users");

    let filter = doc! { "uuid": uuid };
    match col.find_one(filter, None).await
    {
        Ok(Some(user)) => Ok(user),
        Ok(None) => Err(DatabaseError::UserDontExist),
        Err(_) => Err(DatabaseError::DbError),
    }
}

#[cfg(test)]
mod test
{
    use mongodb::{error::Error, Collection, Database};

    use super::*;
    use crate::user::User;


    struct Guard<T>
    {
        database:   Database,
        collection: Collection<T>,
    }

    impl<T> Drop for Guard<T>
    {
        fn drop(&mut self)
        {
            use tokio::{runtime::Handle, task};

            task::block_in_place(move || {
                Handle::current().block_on(async move {
                    self.database.drop(None).await.unwrap();
                });
            });
        }
    }

    async fn get_collection<T>() -> Result<Guard<T>, Error>
    {
        let client = connect().await?;

        let name = format!("{}", uuid::Uuid::new_v4());
        let database = client.database(&name);
        let collection = database.collection::<T>(&name);

        Ok(Guard {
            database,
            collection,
        })
    }


    #[tokio::test(flavor = "multi_thread")]
    async fn test_can_register_and_find_user() -> Result<(), Error>
    {
        let guard = get_collection::<User>().await?;

        let cred = UserCredentials {
            name: "sivert".into(), password: "password".into()
        };

        let res = register_user(guard.database.clone(), cred).await;
        assert!(res.is_ok());

        let res = find_user_by_uuid(guard.database.clone(), res.unwrap()).await;
        assert!(res.is_ok());

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_register_user_errors() -> Result<(), Error>
    {
        let guard = get_collection::<User>().await?;

        let cred = UserCredentials {
            name: "sivert".into(), password: "password".into()
        };

        let res = find_user_by_uuid(guard.database.clone(), "totaly-a-uuid".to_string()).await;
        assert_eq!(res.unwrap_err(), DatabaseError::UserDontExist);

        let res = register_user(guard.database.clone(), cred.clone()).await;
        assert!(res.is_ok());

        let res = register_user(guard.database.clone(), cred).await;
        assert_eq!(res.unwrap_err(), DatabaseError::UserAlreadyExist);

        Ok(())
    }
}
