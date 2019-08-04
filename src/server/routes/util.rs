use std::io::Cursor;
use std::sync::Arc;

use rocket::{Request, Response, response};
use rocket::response::Responder;

/// https://github.com/SergioBenitez/Rocket/issues/893#issuecomment-456151567
pub struct ArcResponder<T>(pub Arc<T>);

impl<T> AsRef<[u8]> for ArcResponder<T> where T: AsRef<[u8]> {
    fn as_ref(&self) -> &[u8] {
        (*self.0).as_ref()
    }
}

impl<'r, T: 'r> Responder<'r> for ArcResponder<T> where T: AsRef<[u8]> {
    fn respond_to(self, _request: &Request) -> response::Result<'r> {
        Response::build()
            .sized_body(Cursor::new(self))
            .ok()
    }
}
