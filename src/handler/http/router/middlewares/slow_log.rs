// Copyright 2024 OpenObserve Inc.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use std::{
    future::{ready, Ready},
    time::{Duration, Instant},
};

use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error,
};
use futures_util::future::LocalBoxFuture;

pub struct SlowLog {
    threshold_secs: u64,
}

impl SlowLog {
    pub fn new(threshold_secs: u64) -> Self {
        SlowLog { threshold_secs }
    }
}

impl<S, B> Transform<S, ServiceRequest> for SlowLog
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = SlowLogMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(SlowLogMiddleware {
            service,
            threshold_secs: self.threshold_secs,
        }))
    }
}

pub struct SlowLogMiddleware<S> {
    service: S,
    threshold_secs: u64,
}

impl<S, B> Service<ServiceRequest> for SlowLogMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let start = Instant::now();
        let remote_addr = match req.headers().contains_key("X-Forwarded-For")
            || req.headers().contains_key("Forwarded")
        {
            true => req
                .connection_info()
                .realip_remote_addr()
                .unwrap_or("-")
                .to_string(),
            false => req.connection_info().peer_addr().unwrap_or("-").to_string(),
        };
        let path = req
            .uri()
            .path_and_query()
            .map(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let method = req.method().to_string();
        let threshold = Duration::from_secs(self.threshold_secs);

        let fut = self.service.call(req);

        Box::pin(async move {
            let res = fut.await?;
            let duration = start.elapsed();

            if duration > threshold {
                log::warn!(
                    "Slow request detected - remote_addr: {}, method: {}, path: {}, took: {:.6}",
                    remote_addr,
                    method,
                    path,
                    duration.as_secs_f64()
                );
            }

            Ok(res)
        })
    }
}
