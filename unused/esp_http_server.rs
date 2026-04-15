use esp_idf_svc::http::{Method, server::{EspHttpConnection, EspHttpServer}};

// use crate::http_server::{THttpServer, TRequest, TResponse};

use std::io::Write;

use log::info;

pub struct EspRequest<'r, 'c> {
    inner: esp_idf_svc::http::server::Request<
        &'r mut EspHttpConnection<'c>,
    >,
}

pub struct EspResponse<'r, 'c> {
    inner: esp_idf_svc::http::server::Response<
        &'r mut EspHttpConnection<'c>,
    >,
}

pub struct Request<'r, 'c> {
    inner: EspRequest<'r, 'c>,
}

pub struct Response<'r, 'c> {
    inner: EspResponse<'r, 'c>,
}

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

   

impl<'r, 'c> Request<'r, 'c> {
    pub fn read(&mut self, buf: &mut [u8]) -> anyhow::Result<usize>{

        Ok(self.inner.inner.read(buf)?)
    }

    pub fn method(&self) -> anyhow::Result<TMethod> {
        let m = self.inner.inner.method();
        Ok(match m {
            Method::Delete => TMethod::Delete,
            Method::Get => TMethod::Get,
            Method::Head => TMethod::Head,
            Method::Post => TMethod::Post,
            _ => anyhow::bail!("Unsupported method {:?}", m),
        })
    }

    pub fn uri(&self) -> &str {
        self.inner.inner.uri()
    }

    pub fn into_ok_response(self) -> anyhow::Result<Response<'r, 'c>> {
        self.into_response(200, Some("OK"), &[])
    }

    pub fn into_response(
        self,
        status: u16,
        msg: Option<&'r str>,
        headers: &'r [(&'r str, &'r str)],
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

pub struct ResponseHandle<'r, 'c> {
    inner: Option<Response<'r, 'c>>,
}

pub trait HttpServer {
    fn on<F>(
        &mut self,
        uri: &str,
        method: TMethod,
        handler: F,
    ) -> anyhow::Result<()>
    where
        F: Fn(Request) -> anyhow::Result<()>
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

// fn adapt<F>(
//     f: F,
// ) -> impl for<'r, 'c> Fn(
//     esp_idf_svc::http::server::Request<
//         &'r mut EspHttpConnection<'c>
//     >
// ) -> anyhow::Result<()>
// where
//     F: Fn(Request<'r, 'c>) -> anyhow::Result<()>
//         + Send
//         + Sync
//         + 'static,
// {
//     move |raw_req| {
//         let req = Request {
//             inner: EspRequest { inner: raw_req },
//         };

//         let resp = ResponseHandle { inner: None };

//         f(req, resp)
//     }
// }

fn adapt<F>(
    f: F,
) -> impl for<'r, 'c> Fn(
    esp_idf_svc::http::server::Request<
        &'r mut EspHttpConnection<'c>
    >
) -> anyhow::Result<()>
where
    F: for<'r, 'c> Fn(Request<'r, 'c>) -> anyhow::Result<()>
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

impl<'s> HttpServer for EspHttpServerManager<'s> {
    fn on<F>(
        &mut self,
        uri: &str,
        method: TMethod,
        handler: F,
    ) -> anyhow::Result<()>
    where
        F: Fn(Request) -> anyhow::Result<()>
            + Send
            + Sync
            + 'static,
    {
        self.server.fn_handler(
            uri,
            to_esp_method(method),
            adapt(handler),
        )?;

        Ok(())
    }
}

// pub trait TResponse: std::io::Write
// {}

// pub trait TRequest<'r, 'c>
// where 'c:'r 
// {
//     type Response: TResponse;

//     fn method(&self) -> anyhow::Result<TMethod>;
//     fn uri(&'r self) -> &'r str;
//     fn into_ok_response(self) -> anyhow::Result<Self::Response>;
//     fn into_response(
//         self,
//         status: u16,
//         message: Option<&'r str>,
//         headers: &'r [(&'r str, &'r str)],
//     ) -> anyhow::Result<Self::Response>;
// }

// pub trait THttpServer
// {
//     type Request<'r, 'c>: TRequest<'r, 'c>
//     where 'c:'r ;

//     fn fn_handler<F>(
//         &mut self,
//         uri: &str,
//         method: TMethod,
//         f: F,
//     ) -> anyhow::Result<()>
//     where
//         F: for<'r, 'c> Fn(Self::Request<'r, 'c>) -> anyhow::Result<()> + Send + Sync + 'static;


//     fn init_common_pages(&mut self) -> anyhow::Result<()> {
//         self.fn_handler("/main.css", TMethod::Get, |req| {
//             // info!("Received {:?} request for {}", req.method(), &req.uri().to_string());

//             let mut resp = req.into_response(
//                 200,
//                 Some("OK"),
//                 &[("Content-Type", "text/css")],
//             )?;
//             resp.write(r#"
// body { font-family: system-ui, -apple-system, BlinkMacSystemFont, sans-serif; margin: 0; padding: 0; background: #f7f7f7; }
// .page { max-width: 480px; margin: 0 auto; padding: 18px; }
// h1 { font-size: 1.5rem; margin-bottom: 1rem; }
// label { display: block; margin: 12px 0 6px; font-weight: 600; }
// input, select { width: 100%; padding: 10px 10px; border: 1px solid #ccc; border-radius: 8px; box-sizing: border-box; }
// button { margin-top: 18px; width: 100%; padding: 12px; font-size: 1rem; border-radius: 10px; border: none; background: #007aff; color: #fff; }
// button:active { background: #005bb5; }
//                         "#.as_bytes())?;
//             Ok(())
//         })?;
//         Ok(())
//     }
// }





// pub struct EspResponse<'r,'c>
// where 'c: 'r
// {
//     esp_response: esp_idf_svc::http::server::Response<&'r mut EspHttpConnection<'c>>,
// }


// impl EspResponse<'_,'_> {
//     fn anyhow<T>(result: Result<T, esp_idf_hal::io::EspIOError>) -> std::io::Result<T> {
//         match result {
//             Ok(cnt) => Ok(cnt),
//             Err(error) => Err(std::io::Error::new(std::io::ErrorKind::Other, error)),
//         }
//     }
// }

// impl<'a> std::io::Write for EspResponse<'_,'_> {
//     fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
//         Self::anyhow(self.esp_response.write(buf))
//     }

//     fn flush(&mut self) -> std::io::Result<()> {
//         Self::anyhow(self.esp_response.flush())
//     }
// }

// impl TResponse for EspResponse<'_,'_> 
// {

// }

// pub struct EspRequest<'r,'c>
// {
//     esp_request: esp_idf_svc::http::server::Request<&'r mut EspHttpConnection<'c>>
// }

// impl<'r,'c> TRequest<'r, 'c> for EspRequest<'r,'c> 
// where 'c: 'r
// {
//     type Response = EspResponse<'r,'c>;

//     fn method(&self) -> anyhow::Result<TMethod> {
//         let m = self.esp_request.method();
//         Ok(match m {
//             Method::Delete => TMethod::Delete,
//             Method::Get => TMethod::Get,
//             Method::Head => TMethod::Head,
//             Method::Post => TMethod::Post,
//             _ => anyhow::bail!("Unsupported method {:?}", m),
//         })
//     }

//     fn uri(&'r self) -> &'r str {
//         self.esp_request.uri()
//     }
    
//     fn into_ok_response(self) -> anyhow::Result<Self::Response> {

//         let esp_response: esp_idf_svc::http::server::Response<&'r mut EspHttpConnection<'c>> = self.esp_request.into_ok_response()?;
//         let response: EspResponse<'r,'c> = EspResponse {
//             esp_response
//         };

//         Ok(response)
//     }
    
//     fn into_response(
//         self,
//         status: u16,
//         message: Option<&'r str>,
//         headers: &'r [(&'r str, &'r str)],
//     ) -> anyhow::Result<Self::Response> {

//         let esp_response: esp_idf_svc::http::server::Response<&'r mut EspHttpConnection<'c>> = self.esp_request.into_response(status, message, headers)?;
//         let response: EspResponse<'r,'c> = EspResponse {
//             esp_response
//         };

//         Ok(response)
//     }
// }

// impl<'r, 'c> EspRequest<'r, 'c> {
//     // type Response = EspResponse<'r>;

    
// }

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



// // i

// fn adapt<F>(f: F) -> impl for<'r,'c> Fn(esp_idf_svc::http::server::Request<&'r mut EspHttpConnection<'c>>) -> anyhow::Result<()>
// where
//     F: for<'r, 'c> Fn(EspRequest<'r, 'c>) -> anyhow::Result<()> + Send + Sync + 'static,
// {
//     move |raw_req| {
//         let esp_req = EspRequest { esp_request: raw_req };
//         f(esp_req)
//     }
// }

// impl<'s> THttpServer for EspHttpServerManager<'s>
// {
//     type Request<'r, 'c> = EspRequest<'r, 'c>
//     where 'c: 'r;

//     fn fn_handler<F>(
//         &mut self,
//         uri: &str,
//         method: TMethod,
//         f: F,
//     ) -> anyhow::Result<()>
//     where
//         F: for<'r, 'c> Fn(Self::Request<'r, 'c>) -> anyhow::Result<()> + Send + Sync + 'static,
//     {
//         // let wrapped_f =
//         //     move |raw_req| {
//         //         let esp_req = EspRequest { esp_request: raw_req };
//         //         f(esp_req)
//         //     };

//         // self.server.fn_handler(uri, to_esp_method(method), wrapped_f)?;

//         self.server.fn_handler(uri, to_esp_method(method), adapt(f))?;
//         Ok(())
//     }
// }

