use crate::AuthInfo;
use actix_http::{HttpMessage, Payload};
use actix_web::{
    dev::{Extensions, ServiceRequest},
    http::StatusCode,
    web::{HttpRequest, HttpResponse},
    Error, FromRequest,
};
use futures::future::{err, ok, Ready};
use paperclip::actix::Apiv2Security;
use std::{cell::RefCell, rc::Rc};

#[derive(Apiv2Security, Debug)]
#[openapi(apiKey, in = "cookie", name = "sm64js")]
pub struct Identity(Rc<RefCell<Option<AuthInfo>>>);

impl Identity {
    pub fn get_auth_info(&self) -> AuthInfo {
        self.0.borrow().as_ref().unwrap().clone()
    }

    pub fn set_identity(account: AuthInfo, req: &mut ServiceRequest) {
        let identity = Identity::get_identity(&mut *req.extensions_mut());
        let mut inner = identity.0.borrow_mut();
        *inner = Some(account);
    }

    fn get_identity(extensions: &mut Extensions) -> Identity {
        if let Some(s_impl) = extensions.get::<Rc<RefCell<Option<AuthInfo>>>>() {
            return Identity(Rc::clone(s_impl));
        }
        let inner = Rc::new(RefCell::new(None));
        extensions.insert(inner.clone());
        Identity(inner)
    }
}

impl FromRequest for Identity {
    type Error = Error;
    type Future = Ready<Result<Identity, Error>>;
    type Config = ();

    #[inline]
    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let identity = Identity::get_identity(&mut *req.extensions_mut());
        let inner = identity.0.borrow();
        if inner.is_some() {
            drop(inner);
            ok(identity)
        } else if req.path().contains("/api/login") || req.path().contains("/api/logout") {
            err(HttpResponse::new(StatusCode::NO_CONTENT).into())
        } else {
            err(HttpResponse::new(StatusCode::UNAUTHORIZED).into())
        }
    }
}
