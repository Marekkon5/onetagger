//! Admission and response handling for providers without a published lower quota.
use crate::cooldown::ProviderCooldown;
use crate::request_pacer::RequestPacer;
use anyhow::Error;
use reqwest::blocking::{RequestBuilder, Response};
use std::time::Duration;

pub(crate) struct ProviderRequests {
    pacer: RequestPacer,
    cooldown: &'static ProviderCooldown,
}

impl ProviderRequests {
    pub(crate) const fn new(cooldown: &'static ProviderCooldown) -> Self {
        Self { pacer: RequestPacer::new(), cooldown }
    }
}

pub(crate) trait PacedRequest {
    fn send_paced(self, budget: &ProviderRequests, slowdown: u16) -> Result<Response, Error>;
}

impl PacedRequest for RequestBuilder {
    fn send_paced(self, budget: &ProviderRequests, slowdown: u16) -> Result<Response, Error> {
        budget.cooldown.check()?;
        // A local safety budget, not a claim about an unpublished service quota.
        budget.pacer.wait_scaled(Duration::from_millis(250), slowdown);
        budget.cooldown.check()?;
        let response = self.send()?;
        budget.cooldown.check_response(&response, Duration::from_secs(15))?;
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cooldown::ProviderRateLimited;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    #[test]
    fn a_retry_after_response_prevents_the_next_http_request() {
        static COOLDOWN: ProviderCooldown = ProviderCooldown::new();
        let budget = ProviderRequests::new(&COOLDOWN);
        let server = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/", server.local_addr().unwrap());
        let client = reqwest::blocking::Client::builder().no_proxy().build().unwrap();
        std::thread::scope(|scope| {
            scope.spawn(|| {
                let (mut socket, _) = server.accept().unwrap();
                let mut bytes = [0; 2048];
                socket.read(&mut bytes).unwrap();
                socket.write_all(b"HTTP/1.1 429 Too Many Requests\r\nRetry-After: 120\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
            });
            let error = client.get(&url).send_paced(&budget, 2).unwrap_err();
            let limited = error.downcast_ref::<ProviderRateLimited>().unwrap();
            assert!(!limited.deferred());
            assert_eq!(limited.retry_after_seconds(), 120);
        });
        let error = client.get(&url).send_paced(&budget, 2).unwrap_err();
        assert!(error.downcast_ref::<ProviderRateLimited>().unwrap().deferred());
        server.set_nonblocking(true).unwrap();
        assert_eq!(server.accept().unwrap_err().kind(), std::io::ErrorKind::WouldBlock);
    }
}
