
use std::io::Write;

use log::info;

pub trait Response: std::io::Write {}

pub trait Request {
    type Response : Response;

    fn method(&self) -> anyhow::Result<TMethod>;
    fn uri(&self) -> &str;

    fn into_response(
        self,
        status: u16,
        msg: Option<&str>,
        headers: &[(&str, &str)],
    ) -> anyhow::Result<Self::Response>;

    fn into_ok_response(
        self,
    ) -> anyhow::Result<Self::Response>;
}



#[derive(Debug)]
pub enum TMethod {
    Delete,
    Get,
    Head,
    Post,
}

pub trait HttpServer {
    type Req:  Request;

    fn on<F>(
        &mut self,
        uri: &str,
        method: TMethod,
        handler: F,
    ) -> anyhow::Result<()>
    where
        F: Fn(Self::Req) -> anyhow::Result<()>
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






// PRIVATE STARTS HERE

use esp_idf_svc::http::{Method, server::{EspHttpConnection, EspHttpServer}};

// use crate::http_server::{Request,Response,HttpServer, TMethod};



pub struct EspResponse<'s> {
    inner: esp_idf_svc::http::server::Response<
        &'s mut EspHttpConnection<'s>,
    >,
}

impl Response  for EspResponse<'_> {}

impl EspResponse<'_> {
    fn anyhow<T>(result: Result<T, esp_idf_hal::io::EspIOError>) -> std::io::Result<T> {
        match result {
            Ok(cnt) => Ok(cnt),
            Err(error) => Err(std::io::Error::new(std::io::ErrorKind::Other, error)),
        }
    }
}

impl std::io::Write for EspResponse<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Self::anyhow(self.inner.write(buf))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Self::anyhow(self.inner.flush())
    }
}

// pub struct EspRequest<'r, 'c> {
//     inner: esp_idf_svc::http::server::Request<
//         &'r mut EspHttpConnection<'c>,
//     >,
// }

// pub struct EspRequest<'r> {
//     inner: esp_idf_svc::http::server::Request<
//         &'r mut EspHttpConnection<'static> // ← key trick
//     >,
// }

pub struct EspRequest<'s> {
    inner: esp_idf_svc::http::server::Request<&'s mut EspHttpConnection<'s>>,
}

impl EspRequest<'_> {
    fn read(&mut self, buf: &mut [u8]) -> anyhow::Result<usize>{

        Ok(self.inner.read(buf)?)
    }
}

impl<'s> Request for EspRequest<'s> {
    type Response = EspResponse<'s>;
    // where
    //     Self: 'r;

    fn method(&self) -> anyhow::Result<TMethod> {
        let m = self.inner.method();
        Ok(match m {
            Method::Delete => TMethod::Delete,
            Method::Get => TMethod::Get,
            Method::Head => TMethod::Head,
            Method::Post => TMethod::Post,
            _ => anyhow::bail!("Unsupported method {:?}", m),
        })
    }

    fn uri(&self) -> &str {
        self.inner.uri()
    }

    fn into_response(
        self,
        status: u16,
        msg: Option<&str>,
        headers: &[(&str, &str)],
    ) -> anyhow::Result<Self::Response>
    {
        Ok(Self::Response {
            inner: self.inner.into_response(status, msg, headers)?,
        })
    }
    
    fn into_ok_response(
        self
    ) -> anyhow::Result<Self::Response> {
        self.into_response(200, Some("OK"), &[])
    }
}


// pub struct EspHttpServerManager<'s>{
//     server: EspHttpServer<'s>,
// }

// impl<'s, 'r> EspHttpServerManager<'s> {
//     pub fn new() -> anyhow::Result<Self> {
//         // let server = EspHttpServer::new(&Default::default())?;
//         Ok(Self {
//             server: EspHttpServer::new(&Default::default())?,
//         })
//     }
// }

fn to_esp_method(method: TMethod) -> Method {
    match method {
        TMethod::Delete =>Method::Delete,
        TMethod::Get => Method::Get,
        TMethod::Head => Method::Head,
        TMethod::Post => Method::Post,
    }
}

// fn adapt<'s, F>(
//     f: F,
// ) -> impl Fn(
//     esp_idf_svc::http::server::Request<
//         &'s mut EspHttpConnection<'s>
//     >
// ) -> anyhow::Result<()>
// where
//     F: Fn(EspRequest<'s>) -> anyhow::Result<()>
//         + Send
//         + Sync
//         + 'static,
// {
//     move |raw_req| {
//         let req = EspRequest { inner: raw_req };
//         f(req)
//     }
// }

fn adapt<F>(f: F) -> impl for<'r> Fn(esp_idf_svc::http::server::Request<&mut EspHttpConnection<'r>>) -> anyhow::Result<()>
where
    F: for<'r> Fn(EspRequest<'r>) -> anyhow::Result<()>
        + Send
        + Sync
        + 'static
{
    move |raw_req| {
        let req = EspRequest { inner: raw_req };
        f(req)
    }
}

pub struct EspHttpServerManager<'s> {
    server: EspHttpServer<'s>,
}

impl<'s> EspHttpServerManager<'s> {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            server: EspHttpServer::new(&Default::default())?,
        })
    }
}

impl<'s> HttpServer for EspHttpServerManager<'s> {
    type Req = EspRequest<'s>;

    fn on<F>(
        &mut self,
        uri: &str,
        method: TMethod,
        handler: F,
    ) -> anyhow::Result<()>
    where
        F: Fn(Self::Req) -> anyhow::Result<()>
            + Send
            + Sync
            + 'static,
    {
        self.server
            .fn_handler(uri, to_esp_method(method), 
            // adapt(handler)
            move |raw_req| {
                let req = EspRequest { inner: raw_req };
                handler(req)
            }
        
        )?;
        Ok(())
    }
}

// impl<'s> HttpServer for EspHttpServerManager<'s> {
//     type Req = EspRequest<'s>;
//     fn on<F>(
//         &mut self,
//         uri: &str,
//         method: TMethod,
//         handler: F,
//     ) -> anyhow::Result<()>
//     where
//         F: for<'r> Fn(Self::Req) -> anyhow::Result<()>
//             + Send
//             + Sync
//             + 'static,
//     {
//         self.server.fn_handler(
//             uri,
//             to_esp_method(method),
//             adapt(handler)
//             // move |raw_req| {
//             //     let req = EspRequest { inner: raw_req };
//             //     handler(req)
//             // },
//         )?;

//         Ok(())
//     }
// }

// impl<'s> HttpServer for EspHttpServerManager<'s> {
//     type Req<'r> = EspRequest<'r, 's>;

//     fn on<F>(
//         &mut self,
//         uri: &str,
//         method: TMethod,
//         handler: F,
//     ) -> anyhow::Result<()>
//     where
//         F: for<'r> Fn(Self::Req<'r>) -> anyhow::Result<()>
//             + Send
//             + Sync
//             + 'static,
//     {
//         self.server.fn_handler(
//             uri,
//             to_esp_method(method),
//             move |raw_req| {
//                 let req = EspRequest { inner: raw_req };
//                 handler(req)
//             },
//         )?;

//         Ok(())
//     }
// }

// impl<'s> HttpServer for EspHttpServerManager<'s> {
//     type Req<'r> = EspRequest<'r, 's>;

//     fn on<F>(&mut self, uri: &str, method: TMethod, handler: F)
//         -> anyhow::Result<()>
//     where
//         F: for<'r> Fn(Self::Req<'r>) -> anyhow::Result<()>
//             + Send
//             + Sync
//             + 'static,
//     {
//         self.server.fn_handler(
//             uri,
//             to_esp_method(method),
//             move |raw_req| {
//                 let req = EspRequest { inner: raw_req };
//                 handler(req)
//             },
//         )?;

//         Ok(())
//     }
// }

// impl<'s> HttpServer for EspHttpServerManager<'s> {
//     fn on<F>(
//         &mut self,
//         uri: &str,
//         method: TMethod,
//         handler: F,
//     ) -> anyhow::Result<()>
//     where
//         F: Fn(Request) -> anyhow::Result<()>
//             + Send
//             + Sync
//             + 'static,
//     {
//         self.server.fn_handler(
//             uri,
//             to_esp_method(method),
//             adapt(handler),
//         )?;

//         Ok(())
//     }
// }
