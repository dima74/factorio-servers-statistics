use rocket::routes;

use fss::state::StateLock;

mod routes;
mod cors;

pub fn init(state_lock: StateLock) {
    let routes = routes![
        routes::index,
        routes::servers_search_index,
        routes::get_server_info::get_server_info,
    ];
    rocket::ignite()
        .attach(cors::CORS())
        .manage(state_lock)
        .mount("/", routes)
        .launch();
}
