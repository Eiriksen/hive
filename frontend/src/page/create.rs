use seed::{prelude::*, *};
use shared::{model::http::*, ObjectId};

use crate::Msg::CreateGame;

pub fn init() -> Model {
    Model::default()
}

pub enum Msg {
    Submit,
    Fetched(fetch::Result<String>),
}

#[derive(Default)]
pub struct Model {
    text: Option<Status>,
}

enum Status {
    Success(String),
    Error(String),
}

async fn send_message(creator: ObjectId) -> fetch::Result<String> {
    Request::new("http://0.0.0.0:5000/create-game")
        .method(Method::Post)
        .json(&creator)?
        .fetch()
        .await?
        .check_status()?
        .text()
        .await
}

pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::Submit => match LocalStorage::get("id") {
            Ok(id) => {
                orders
                    .skip()
                    .perform_cmd(async move { Msg::Fetched(send_message(id).await) });
            }
            Err(_) => {
                model.text = Some(Status::Error(format!("User not logged in")));
            }
        },
        Msg::Fetched(Ok(text)) => match serde_json::from_str::<ResponseBody>(&text) {
            Ok(resp) => match resp.status {
                201 => {
                    model.text = Some(Status::Success(format!("Game successfully created!")));
                }

                e => {
                    model.text = Some(Status::Error(format!("Error ({e})")));
                }
            },
            Err(e) => {
                model.text = Some(Status::Error(format!("deserialize error, {e:?}")));
            }
        },

        Msg::Fetched(Err(text)) => {
            model.text = Some(Status::Error(format!("{text:?}")));
        }
    }
}

pub fn view<Ms: 'static>(model: &Model) -> Node<Ms> {
    let body = || {
        form![
            ev(Ev::Submit, |event| {
                event.prevent_default();
                CreateGame(Msg::Submit)
            }),
            div![C!("center-button"), button![C!["button"], "Create"]],
        ]
    };
    div![
        C!("container center"),
        body(),
        IF!(model.text.is_some() => match model.text {
            Some(Status::Success(ref s)) => h2! [C!("success"), s],
            Some(Status::Error(ref s)) => h2! [C!("error"), s],
            _ => unreachable!()
        })
    ]
}