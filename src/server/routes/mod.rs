use rocket::{get, State};
use rocket::http::Status;
use rocket::response::status;

use fss::state::StateLock;

pub mod get_server_info;
pub mod main_page;
pub mod util;

#[get("/")]
pub fn index(state_lock: State<StateLock>) -> Result<&'static str, status::Custom<&'static str>> {
    if state_lock.is_some() {
        Ok("api works!")
    } else {
        Err(status::Custom(Status::InternalServerError, "State not loaded yet"))
    }
}
