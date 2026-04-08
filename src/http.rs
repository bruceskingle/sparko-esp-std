use std::sync::{Arc, Mutex};

use esp_idf_svc::http::{Method, server::{EspHttpConnection, EspHttpServer}};
use log::info;


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

    pub fn fn_handler<F>(
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
        self.fn_handler("/main.css", Method::Get, |req| {
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

    // I don't think we need this.
    // pub fn init_ap_pages(&mut self) -> anyhow::Result<()> {
    //     // *self.configured.lock().unwrap() = false;

        
        

    //     self.fn_handler("/", Method::Get, |req| {

    //         // info!("Received request for / from {}", req.connection().remote_addr());

    //         info!("Received {:?} request for {}", req.method(), req.uri());

    //         // let html = r#"
    //         //     <!DOCTYPE html>
    //         //     <html lang="en">
    //         //     <head>
    //         //         <meta charset="utf-8" />
    //         //         <meta name="viewport" content="width=device-width, initial-scale=1" />
    //         //         <title>ESP32 Setup</title>
    //         //         <style>
    //         //             body { font-family: system-ui, -apple-system, BlinkMacSystemFont, sans-serif; margin: 0; padding: 0; background: #f7f7f7; }
    //         //             .page { max-width: 480px; margin: 0 auto; padding: 18px; }
    //         //             h1 { font-size: 1.5rem; margin-bottom: 1rem; }
    //         //             label { display: block; margin: 12px 0 6px; font-weight: 600; }
    //         //             input { width: 100%; padding: 10px 10px; border: 1px solid #ccc; border-radius: 8px; box-sizing: border-box; }
    //         //             button { margin-top: 18px; width: 100%; padding: 12px; font-size: 1rem; border-radius: 10px; border: none; background: #007aff; color: #fff; }
    //         //             button:active { background: #005bb5; }
    //         //         </style>
    //         //     </head>
    //         //     <body>
    //         //         <div class="page">
    //         //             <h1>ESP32 Setup</h1>
    //         //             <form method="POST" action="/connect">
    //         //                 <label for="ssid">WiFi SSID</label>
    //         //                 <input id="ssid" name="ssid" autocomplete="off" required />

    //         //                 <label for="pass">WiFi Password</label>
    //         //                 <input id="pass" name="pass" type="password" autocomplete="off" required />

    //         //                 <button type="submit">Save</button>
    //         //             </form>
    //         //         </div>
    //         //     </body>
    //         //     </html>
    //         //     "#;

    //                     let html = r#"
    //             <!DOCTYPE html>
    //             <html lang="en">
    //             <head>
    //             </head>
    //             <body>
    //                 Try /config
    //             </body>
    //             </html>
    //             "#;
    //         let mut resp = req.into_ok_response()?;
    //         resp.write(html.as_bytes())?;
    //         Ok(())
    //     })?;

    //     // let configured_clone2 = self.configured.clone();
    //     // self.fn_handler("/connect", Method::Post, move |mut req| {

    //     //     // info!("Received request for /connect from {}", req.connection().remote_addr());

    //     //     info!("Received {:?} request for {}", req.method(), req.uri());
            

    //     //     let mut buf = [0;512];
    //     //     let len = req.read(&mut buf)?;

    //     //     let body = core::str::from_utf8(&buf[..len]).unwrap();

    //     //     let ssid = parse(body,"ssid");
    //     //     let pass = parse(body,"pass");

    //     //     info!("Received WiFi credentials: ssid={}, pass={}", ssid, pass);
    //     //     // tx_clone.send(WifiCommand::Connect{ssid,pass}).ok();


    //     //     *configured_clone2.lock().unwrap() = true;

    //     //     let mut resp = req.into_ok_response()?;
    //     //     resp.write(b"Saved!")?;
    //     //     Ok(())
    //     // })?;

    //     // fn parse(body:&str,key:&str)->String{
    //     //     body.split('&')
    //     //         .find(|p|p.starts_with(key))
    //     //         .and_then(|v|v.split('=').nth(1))
    //     //         .unwrap_or("")
    //     //         .to_string()
    //     // }

    //     Ok(())
    // }
}

