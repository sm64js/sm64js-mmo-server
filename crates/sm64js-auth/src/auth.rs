use crate::{AuthInfo, Identity};
use actix_service::{Service, Transform};
use actix_session::UserSession;
use actix_web::{
    dev::{RequestHead, ServiceRequest, ServiceResponse},
    http::header,
    web, Error,
};
use futures::future::{ok, Future, Ready};
use sm64js_db::DbPool;
use std::{
    cell::RefCell,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll},
};

pub struct Auth;

impl<S: 'static, B> Transform<S> for Auth
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AuthMiddleware {
            service: Rc::new(RefCell::new(service)),
        })
    }
}

pub struct AuthMiddleware<S> {
    service: Rc<RefCell<S>>,
}

impl<S, B> Service for AuthMiddleware<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    #[allow(clippy::type_complexity)]
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, mut req: ServiceRequest) -> Self::Future {
        let mut svc = self.service.clone();

        Box::pin(async move {
            if req.path().starts_with("/api/") || req.path().starts_with("/ws/") {
                let session = req.get_session();
                let pool: Option<&web::Data<DbPool>> = req.app_data();
                if let Some(pool) = pool {
                    let conn = pool.get().expect("couldn't get db connection from pool");
                    match sm64js_db::get_auth_info(&conn, &session) {
                        Ok(Some(account)) => {
                            Identity::set_identity(AuthInfo(account), &mut req);
                        }
                        Ok(None) => {}
                        Err(err) => {
                            eprintln!("{:?}", err);
                            session.purge();
                        }
                    }
                    // TODO apikey auth
                    // if let Some(apikey) = get_apikey_from_head(req.head()) {
                    //     if let Some(token) = Token::find(data, apikey) {
                    //         Identity::set_identity(token, &mut req);
                    //     }
                    // }
                }
            }
            let res = svc.call(req).await?;
            Ok(res)
        })
    }
}

fn _get_apikey_from_head(header: &RequestHead) -> Option<String> {
    if let Some(authorization) = header.headers().get(header::AUTHORIZATION) {
        if let Ok(authorization) = authorization.to_str() {
            let s: Vec<&str> = authorization.split(' ').collect();
            if let (Some("APIKEY"), Some(apikey)) = (s.get(0).copied(), s.get(1)) {
                return Some((*apikey).to_string());
            }
        }
    }
    None
}
