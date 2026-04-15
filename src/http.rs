use esp_idf_svc::http::{Method, server::{EspHttpConnection, EspHttpServer}};
use log::info;

/*
I have made extensive efforts to refactor this into a hardware agnostic set of traits and an ESP32 specific back end
without success. I wanted an abstraction which is zero cost so did not go for boxed trait objects (which would require
memory allocations on each request) and instead was looking for a solution based on generic associated types (GAT). 

My conclusion is that this is not possible between the way the ESP Http server works and the limitations of the rust
type system.

The code for the attempted rewrite is in http_server.rs and esp_http_server.rs in the http_api branch.
*/

pub struct HttpServerManager<'a>{
    server: EspHttpServer<'a>,
}

impl HttpServerManager<'_> {
    pub fn new() -> anyhow::Result<Self> {
        let server = EspHttpServer::new(&Default::default())?;
        Ok(Self {
            server,
        })
    }

    pub fn on<F>(
        &mut self,
        uri: &str,
        method: Method,
        f: F,
    ) -> anyhow::Result<()>
    where
        F: for<'r> Fn(esp_idf_svc::http::server::Request<&mut EspHttpConnection>) -> anyhow::Result<()> + Send + 'static,
    {
        self.server.fn_handler(uri, method, f)?;
        Ok(())
    }

    pub fn init_common_pages(&mut self) -> anyhow::Result<()> {
        self.on("/main.css", Method::Get, |req| {
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

