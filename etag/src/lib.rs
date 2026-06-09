use std::future::Ready;
use actix_service::{forward_ready, Service};
use actix_web::body::{BoxBody, EitherBody, MessageBody};
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::http::header::{ETag, EntityTag, Header, IfNoneMatch, TryIntoHeaderPair};
use actix_web::http::{Method, StatusCode};
use actix_web::{HttpMessage, HttpResponse};
use actix_web::web::Bytes;
use futures::TryFutureExt;

#[non_exhaustive]
pub struct Etag<const HASH_SIZE:usize, B>
{
    pub hash_func: fn(&[u8], &ServiceResponse) -> [u8; HASH_SIZE],
    pub method_picker_req: fn(&ServiceRequest) -> Option<bool>,
    pub method_picker_res: fn(&ServiceResponse<B>) -> bool,
    pub force_strong_etag: fn(&ServiceResponse) -> bool,
}

impl<B> Etag<{blake3::OUT_LEN},B> {
    pub const DEFAULT:Self = Etag{
        hash_func: |v, _|blake3::hash(v).into(),
        method_picker_req: |_| None,
        method_picker_res: |res| matches!(*res.request().method(), Method::GET|Method::HEAD),
        force_strong_etag: |_| false,
    };
}
impl<B> core::default::Default for Etag<{blake3::OUT_LEN}, B> {
    fn default() -> Self {
        Self::DEFAULT
    }
}
impl<const HASH_SIZE:usize, B> Clone for Etag<HASH_SIZE, B> {
    fn clone(&self) -> Self {
        Self{
            hash_func: self.hash_func,
            method_picker_req: self.method_picker_req,
            method_picker_res: self.method_picker_res,
            force_strong_etag: self.force_strong_etag,
        }
    }
}
pub struct EtagMiddleware<const HASH_SIZE:usize, S, B> {
    data: Etag<HASH_SIZE, B>,
    service: S,
}

impl<S, B, const HASH_SIZE:usize> actix_service::Transform<S, ServiceRequest> for Etag<HASH_SIZE, B>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<EitherBody<BoxBody>>;
    type Error = actix_web::Error;
    type Transform = EtagMiddleware<HASH_SIZE, S, B>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        core::future::ready(Ok(EtagMiddleware {
            service,
            data: self.clone(),
        }))
    }
}


impl<S, B, const HASH_SIZE:usize> Service<ServiceRequest> for EtagMiddleware<HASH_SIZE, S, B>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<EitherBody<BoxBody>>;
    type Error = actix_web::Error;
    #[allow(clippy::type_complexity)]
    type Future = futures::future::MapOk<S::Future, Box<dyn FnOnce(ServiceResponse<B>) -> Self::Response>>;
    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let request_etag_header: Option<IfNoneMatch> = req.get_header();
        let data = self.data.clone();
        let handle = (data.method_picker_req)(&req);
        let fut = self.service.call(req);
        fut.map_ok(Box::new(move |res| {
            if handle.unwrap_or_else(||(data.method_picker_res)(&res)) {
                let mut modified = true;
                let mut payload: Option<Bytes> = None;
                let mut res = res.map_body(|_h, body| match body.try_into_bytes() {
                    Ok(v) => {
                        payload = Some(v.clone());
                        v.boxed()
                    },
                    Err(slf) => slf.boxed()
                });
                if let Some(bytes) = payload {
                    let custom_etag = res.response().headers().get(ETag::name());
                    let tag = match custom_etag.and_then(|etag| etag.to_str().ok()) {
                        Some(custom_etag) => EntityTag::new_strong(custom_etag.to_owned()),
                        None => {
                            let response_hash = (data.hash_func)(&bytes, &res);
                            let base64 = data_encoding::BASE64.encode(&response_hash);
                            let buff = format!("{:x}-{}", bytes.len(), base64);
                            if (data.force_strong_etag)(&res) {
                                EntityTag::new_strong(buff)
                            } else {
                                EntityTag::new_weak(buff)
                            }
                        }
                    };

                    if let Some(request_etag_header) = request_etag_header {
                        if request_etag_header == IfNoneMatch::Any
                            || request_etag_header.to_string() == tag.to_string()
                        {
                            modified = false
                        }
                    }
                    if modified {
                        if let Ok((name, value)) = ETag(tag.clone()).try_into_pair() {
                            res.headers_mut().insert(name, value);
                        }
                    }
                }

                match modified {
                    false => res
                        .into_response(HttpResponse::new(StatusCode::NOT_MODIFIED))
                        .map_into_right_body(),
                    true => res.map_into_left_body(),
                }
            } else {
                res.map_into_boxed_body().map_into_left_body()
            }
        }))
    }
}