use rocket::{get, State};
use rocket::response::content;

use fss::cacher::CacherStateLock;

use crate::server::routes::util::ArcResponder;

#[get("/main-page")]
pub fn main_page(cacher_state_lock: State<CacherStateLock>) -> content::Json<ArcResponder<String>> {
    let cacher_state = cacher_state_lock.read();
    content::Json(ArcResponder(cacher_state.main_page_serialized.clone()))
}
