use esp_idf_svc::http::{Method, server::{EspHttpConnection, EspHttpServer}};

// use crate::http_server::{THttpServer, TRequest, TResponse};

use std::io::Write;

use log::info;



/*
✅ Practical pattern (safe abstraction boundary)
You can safely erase 'c if you enforce usage constraints:
pub struct Request<'r> {
    inner: EspRequest<'r, 'static>,
}

impl<'r> Request<'r> {
    pub fn new<'c>(req: EspRequest<'r, 'c>) -> Self {
        Self {
            inner: unsafe { std::mem::transmute(req) },
        }
    }
}
Then enforce API discipline:
Only expose methods like:
impl<'r> Request<'r> {
    pub fn read(&mut self) -> &[u8] { ... }
    pub fn write(&mut self, ...) { ... }
}
❌ Do NOT expose:
&EspHttpConnection
anything carrying 'c
*/

pub trait TResponse: std::io::Write {}

pub trait TRequest {
    type Resp : TResponse;

    fn method(&self) -> anyhow::Result<TMethod>;
    fn uri(&self) -> &str;

    fn into_response(
        self,
        status: u16,
        msg: Option<&str>,
        headers: &[(&str, &str)],
    ) -> anyhow::Result<Self::Resp>;

    fn into_ok_response(
        self,
    ) -> anyhow::Result<Self::Resp>;
}


pub trait HttpServer {
    // type Req<'r, 'c>: TRequest
    // // where
    // //     Self: 'r,
    // //     Self: 'c
    //     ;
    fn on<F>(
        &mut self,
        uri: &str,
        method: TMethod,
        handler: F,
    ) -> anyhow::Result<()>
    where
        // F: for<'r, 'c> Fn(Self::Req<'r, 'c>) -> anyhow::Result<()>
        F: for<'r, 'c> Fn(Request<'r, 'c>) -> anyhow::Result<()>
            + Send
            + Sync
            + 'static;

    

    fn init_common_pages(&mut self) -> anyhow::Result<()> {
        self.on("/main.css", TMethod::Get, |req| {
            info!("Received {:?} request for {}", req.method(), req.uri());

            let mut resp = req.into_response(
                200,
                Some("OK"),
                &[("Content-Type", "text/css")],
            )?;
            resp.write(r#"
body { font-family: system-ui, -apple-system, BlinkMacSystemFont, sans-serif; margin: 0; padding: 0; background: #f7f7f7; }
.page { max-width: 480px; margin: 0 auto; padding: 18px; }
h1 { font-size: 1.5rem; margin-bottom: 1rem; }
label { display: block; margin: 12px 0 6px; font-weight: 600; }
input, select { width: 100%; padding: 10px 10px; border: 1px solid #ccc; border-radius: 8px; box-sizing: border-box; }
button { margin-top: 18px; width: 100%; padding: 12px; font-size: 1rem; border-radius: 10px; border: none; background: #007aff; color: #fff; }
button:active { background: #005bb5; }
                        "#.as_bytes())?;
            Ok(())
        })?;
        Ok(())
    }
}


// End Public

pub struct EspRequest<'r, 'c> {
    inner: esp_idf_svc::http::server::Request<
        &'r mut EspHttpConnection<'c>,
    >,
}

pub struct Request<'r, 'c> {
    inner: EspRequest<'r, 'c>,
}

impl<'r, 'c> Request<'r, 'c> {
    pub fn read(&mut self, buf: &mut [u8]) -> anyhow::Result<usize>{

        Ok(self.inner.inner.read(buf)?)
    }
}

impl<'r, 'c> TRequest for Request<'r, 'c> {
    type Resp = Response<'r, 'c>;

    fn method(&self) -> anyhow::Result<TMethod> {
        let m = self.inner.inner.method();
        Ok(match m {
            Method::Delete => TMethod::Delete,
            Method::Get => TMethod::Get,
            Method::Head => TMethod::Head,
            Method::Post => TMethod::Post,
            _ => anyhow::bail!("Unsupported method {:?}", m),
        })
    }

    fn uri(&self) -> &str {
        self.inner.inner.uri()
    }

    fn into_ok_response(self) -> anyhow::Result<Response<'r, 'c>> {
        self.into_response(200, Some("OK"), &[])
    }

    fn into_response(
        self,
        status: u16,
        msg: Option<&str>,
        headers: &[(&str, &str)],
    ) -> anyhow::Result<Response<'r, 'c>> {

        let x = self.inner.inner.into_response(status, msg, headers)?;
        let y = EspResponse {
            inner: x,
        };
        let z = Response {
            inner: y,
        };

        Ok(z)
        // Ok(Response {
        //     inner: self.inner.inner.into_response(status, msg, headers)?,
        // })
    }
}

pub struct EspResponse<'r, 'c> {
    inner: esp_idf_svc::http::server::Response<
        &'r mut EspHttpConnection<'c>,
    >,
}

pub struct Response<'r, 'c> {
    inner: EspResponse<'r, 'c>,
}

impl TResponse for Response<'_, '_> {}

impl Response<'_, '_> {

    fn anyhow<T>(result: Result<T, esp_idf_hal::io::EspIOError>) -> std::io::Result<T> {
        match result {
            Ok(cnt) => Ok(cnt),
            Err(error) => Err(std::io::Error::new(std::io::ErrorKind::Other, error)),
        }
    }
}

impl std::io::Write for Response<'_, '_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Self::anyhow(self.inner.inner.write(buf))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Self::anyhow(self.inner.inner.flush())
    }
}

#[derive(Debug)]
pub enum TMethod {
    Delete,
    Get,
    Head,
    Post,
}

   



pub struct EspHttpServerManager<'s>{
    server: EspHttpServer<'s>,
}

impl<'s, 'r> EspHttpServerManager<'s> {
    pub fn new() -> anyhow::Result<Self> {
        // let server = EspHttpServer::new(&Default::default())?;
        Ok(Self {
            server: EspHttpServer::new(&Default::default())?,
        })
    }
}

fn to_esp_method(method: TMethod) -> Method {
    match method {
        TMethod::Delete =>Method::Delete,
        TMethod::Get => Method::Get,
        TMethod::Head => Method::Head,
        TMethod::Post => Method::Post,
    }
}

fn adapt<'c, F>(
    f: F,
) -> impl for<'r> Fn(
    esp_idf_svc::http::server::Request<
        &'r mut EspHttpConnection<'c>
    >
) -> anyhow::Result<()>
where
    F: for<'r> Fn(Request<'r, 'c>) -> anyhow::Result<()>
        + Send
        + Sync
        + 'static,
{
    move |raw_req| {
        let req = Request {
            inner: EspRequest { inner: raw_req },
        };

        f(req)
    }
}

impl<'s> HttpServer for EspHttpServerManager<'s> {
    // type Req<'r, 'c> = 
    // Request<'r, 'c>;
    // where
    //     'c: 'r;
    fn on<F>(
        &mut self,
        uri: &str,
        method: TMethod,
        handler: F,
    ) -> anyhow::Result<()>
    where
        // F: for<'r, 'c> Fn(Self::Req<'r, 'c>) -> anyhow::Result<()>
        F: for<'r, 'c> Fn(Request<'r, 'c>) -> anyhow::Result<()>
            + Send
            + Sync
            + 'static,
    {
        self.server.fn_handler(
            uri,
            to_esp_method(method),
            // adapt(handler),
            move |raw_req| {
                let req = Request {
                    inner: EspRequest { inner: raw_req },
                };

                handler(req)
    }
        )?;

        Ok(())
    }
}

