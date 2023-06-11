
use std::convert::Infallible;

use rocket::{request::{FromRequest, self, Outcome}, Request, http::Status};

#[derive(Debug)]
pub struct GitHubEvent(pub String);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for GitHubEvent {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let event = request.headers().get_one("X-GitHub-Event");
        match event {
          Some(event) => {
            // check validity
            Outcome::Success(GitHubEvent(event.to_string()))
          },
          // token does not exist
          None => Outcome::Failure((Status::Unauthorized, ()))
        }
    }
}