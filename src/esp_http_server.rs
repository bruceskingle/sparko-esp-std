use std::sync::{Arc, Mutex};

use esp_idf_svc::http::{Method, server::{EspHttpConnection, EspHttpServer}};
use log::info;

use crate::http::{HttpServer, Request, Response};

pub struct EspResponse<'r> {
    esp_response: esp_idf_svc::http::server::Response<&'r mut EspHttpConnection<'r>>,
}


impl EspResponse<'_> {
    fn anyhow<T>(result: Result<T, esp_idf_hal::io::EspIOError>) -> std::io::Result<T> {
        match result {
            Ok(cnt) => Ok(cnt),
            Err(error) => Err(std::io::Error::new(std::io::ErrorKind::Other, error)),
        }
    }
}

impl<'a> std::io::Write for EspResponse<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Self::anyhow(self.esp_response.write(buf))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Self::anyhow(self.esp_response.flush())
    }
}

impl<'r> Response<'r> for EspResponse<'r> {

}

pub struct EspRequest<'r>
{
    esp_request: esp_idf_svc::http::server::Request<&'r mut EspHttpConnection<'r>>
}

impl<'r> Request<'r> for EspRequest<'r> {
    type Response = EspResponse<'r>;

    fn into_ok_response(self) -> anyhow::Result<Self::Response> {

        let esp_response: esp_idf_svc::http::server::Response<&'r mut EspHttpConnection<'r>> = self.esp_request.into_ok_response()?;
        let response: EspResponse<'r> = EspResponse {
            esp_response
        };

        Ok(response)
    }

    fn into_response(
        self,
        status: u16,
        message: Option<&'r str>,
        headers: &'r [(&'r str, &'r str)],
    ) -> anyhow::Result<Self::Response> {

        let esp_response: esp_idf_svc::http::server::Response<&'r mut EspHttpConnection<'r>> = self.esp_request.into_response()?;
        let response: EspResponse<'r> = EspResponse {
            esp_response
        };

        Ok(response)
    }
    
    fn method(&self) -> anyhow::Result<crate::http::Method> {
        let m = self.esp_request.method();
        Ok(match m {
            Method::Delete => crate::http::Method::Delete,
            Method::Get => crate::http::Method::Get,
            Method::Head => crate::http::Method::Head,
            Method::Post => crate::http::Method::Post,
            _ => anyhow::bail!("Unsupported method {:?}", m),
        })
    }
    
    fn uri(&'r self) -> &'r str {
        self.esp_request.uri()
    }
}

impl<'r> EspRequest<'r> {
    // type Response = EspResponse<'r>;

    
}

pub struct EspHttpServerManager<'a>{
    server: EspHttpServer<'a>,
}

impl EspHttpServerManager<'_> {
    pub fn new() -> anyhow::Result<Self> {
        let server = EspHttpServer::new(&Default::default())?;
        Ok(Self {
            server,
        })
    }

}

fn to_esp_method(method: crate::http::Method) -> Method {
    match method {
        crate::http::Method::Delete =>Method::Delete,
        crate::http::Method::Get => Method::Get,
        crate::http::Method::Head => Method::Head,
        crate::http::Method::Post => Method::Post,
    }
}

impl<'a> HttpServer for EspHttpServerManager<'a>
where 
    R : 
{
    type Request = EspRequest<'_>;

    fn fn_handler<F>(
        &mut self,
        uri: &str,
        method: crate::http::Method,
        f: F,
    ) -> anyhow::Result<()>
    where
        // F: for<'r> Fn(esp_idf_svc::http::server::Request<&mut EspHttpConnection>) -> anyhow::Result<()> + Send + 'static,
        F: for<'r> Fn(Self::Request) -> anyhow::Result<()> + Send + 'static
    {
        self.server.fn_handler(uri, to_esp_method(method), f)?;
        Ok(())
    }
    
}