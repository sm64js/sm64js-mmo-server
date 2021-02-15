// use crate::Token;

use crate::{Identity, Token, Tokens};
use actix_service::{Service, Transform};
use actix_web::{
    dev::{RequestHead, ServiceRequest, ServiceResponse},
    http::header,
    Error,
};
use futures::future::{ok, Future, Ready};
use std::{
    convert::TryFrom,
    pin::Pin,
    task::{Context, Poll},
};

pub struct Auth;

impl<S, B> Transform<S> for Auth
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
        ok(AuthMiddleware { service })
    }
}

pub struct AuthMiddleware<S> {
    service: S,
}

impl<S, B> Service for AuthMiddleware<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
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
        let data: Option<&Tokens> = req.app_data();
        if let (Some(data), Ok(auth_req)) = (data, AuthReq::try_from(req.head())) {
            if let Some(token) = Token::find(data, auth_req.apikey) {
                Identity::set_identity(token, &mut req);
            }
        }

        let fut = self.service.call(req);
        Box::pin(async move {
            let res = fut.await?;
            Ok(res)
        })
    }
}

#[derive(Debug)]
pub struct AuthReq {
    apikey: String,
}

impl TryFrom<&RequestHead> for AuthReq {
    type Error = ();

    fn try_from(header: &RequestHead) -> Result<Self, Self::Error> {
        if let Some(authorization) = header.headers().get(header::AUTHORIZATION) {
            if let Ok(authorization) = authorization.to_str() {
                let s: Vec<&str> = authorization.split(' ').collect();
                if let (Some("APIKEY"), Some(apikey)) = (s.get(0).copied(), s.get(1)) {
                    return Ok(AuthReq {
                        apikey: (*apikey).to_string(),
                    });
                }
            }
        }
        Err(())
    }
}
