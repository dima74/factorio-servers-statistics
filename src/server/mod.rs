use rocket::routes;

use fss::cacher::CacherStateLock;
use fss::state::StateLock;

mod routes;
mod cors;

pub fn init(state_lock: StateLock, cacher_state_lock: CacherStateLock) {
    let routes = routes![
        routes::index,
        routes::servers_search_index,
        routes::get_server_info::get_server_info,
        routes::main_page::main_page,
    ];
    rocket::ignite()
        .attach(cors::CORS())
        .manage(state_lock)
        .manage(cacher_state_lock)
        .mount("/", routes)
        .launch();
}
