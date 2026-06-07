use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::{Error, HttpResponse};
use futures::future::{ready, LocalBoxFuture, Ready};
use regex::Regex;
use std::env;
use std::rc::Rc;
use actix_web::body::BoxBody;

pub struct ApiGuard {
    allowed_ips: Vec<String>,
    api_keys: Vec<String>,
}

impl ApiGuard {
    pub fn new() -> Self {
        let allowed_ips = env::var("ALLOWED_IPS")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();

        let api_keys = env::var("UPSTASH_FLOW_CONTROL_KEY")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();

        ApiGuard { allowed_ips, api_keys }
    }

    fn is_ip_allowed(&self, ip: &str) -> bool {
        let ip_regex = Regex::new(r"^\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}$").unwrap();
        ip_regex.is_match(ip) && self.allowed_ips.contains(&ip.to_string())
    }

    fn is_key_valid(&self, key: &str) -> bool {
        self.api_keys.contains(&key.to_string())
    }
}

impl<S> Transform<S, ServiceRequest> for ApiGuard
where
    S: Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error> + 'static,
    S::Future: 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Transform = ApiGuardMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(ApiGuardMiddleware {
            service: Rc::new(service),
            allowed_ips: self.allowed_ips.clone(),
            api_keys: self.api_keys.clone(),
        }))
    }
}

pub struct ApiGuardMiddleware<S> {
    service: Rc<S>,
    allowed_ips: Vec<String>,
    api_keys: Vec<String>,
}

impl<S> Service<ServiceRequest> for ApiGuardMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error> + 'static,
    S::Future: 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    actix_web::dev::forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = Rc::clone(&self.service);
        let allowed_ips = self.allowed_ips.clone();
        let api_keys = self.api_keys.clone();

        Box::pin(async move {
            let ip = req.connection_info().realip_remote_addr().unwrap_or("").to_string();
            let api_key = req.headers()
                .get("Upstash-Flow-Control-Key")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");

            let guard = ApiGuard { allowed_ips, api_keys };

            if !guard.is_ip_allowed(&ip) || !guard.is_key_valid(api_key) {
                let res = HttpResponse::Forbidden().body("Access denied");
                return Ok(req.into_response(res));
            }

            service.call(req).await
        })
    }
}