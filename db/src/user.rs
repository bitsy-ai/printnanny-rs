use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::schema::user;

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, Queryable, Identifiable)]
#[diesel(table_name = user)]
pub struct User {
    pub email: String,
    pub id: i32,
    #[serde(rename = "first_name", skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,
    #[serde(rename = "last_name", skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,
}

impl From<printnanny_api_client::models::User> for User {
    fn from(obj: printnanny_api_client::models::User) -> User {
        User {
            id: obj.id,
            email: obj.email,
            first_name: obj.first_name,
            last_name: obj.last_name,
        }
    }
}
