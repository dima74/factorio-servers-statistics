use rocket::get;

pub mod get_server_info;
pub mod main_page;
pub mod util;

#[get("/")]
pub fn index() -> &'static str {
    "api works!"
}
