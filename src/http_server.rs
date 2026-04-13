
// use std::io::Write;

// use log::info;

// pub trait Response: std::io::Write {}

// pub trait Request<'r> {
//     type Response: Response;

//     fn method(&self) -> anyhow::Result<TMethod>;
//     fn uri(&self) -> &str;

//     fn into_response(
//         self,
//         status: u16,
//         msg: Option<&'r str>,
//         headers: &'r [(&'r str, &'r str)],
//     ) -> anyhow::Result<Self::Response>;

//     fn into_ok_response(
//         self,
//     ) -> anyhow::Result<Self::Response>;
// }

// // pub trait Request {
// //     type Response<'r>: Response
// //     where
// //         Self: 'r;

// //     fn method(&self) -> anyhow::Result<TMethod>;
// //     fn uri(&self) -> &str;

// //     // fn into_ok_response<'r>(self: Self<'r>) -> anyhow::Result<Self::Response<'r>> 
    
// //     // where
// //     //     Self: 'r
// //     // {
// //     //     self.into_response(200, Some("OK"), &[])
// //     // }

// //     fn into_response<'r>(
// //         self,
// //         status: u16,
// //         msg: Option<&'r str>,
// //         headers: &'r [(&'r str, &'r str)],
// //     ) -> anyhow::Result<Self::Response<'r>>
// //     where
// //         Self: 'r;
    
// //     fn into_ok_response<'r>(
// //         self
// //     ) -> anyhow::Result<Self::Response<'r>>
// //     where
// //         Self: 'r;
    
// //     // fn into_ok_response<'r>(self) -> anyhow::Result<Self::Response<'r>>
// //     // where
// //     //     Self: 'r,
// //     // {
// //     //     self.into_response(200, Some("OK"), &[])
// //     // }
// // }


// #[derive(Debug)]
// pub enum TMethod {
//     Delete,
//     Get,
//     Head,
//     Post,
// }

// pub trait HttpServer {
//     type Req<'r>: Request<'r>;

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
//             + 'static;
// // }
// // pub trait HttpServer {
// //     type Req<'r>: Request
// //     where
// //         Self: 'r;

// //     fn on<F>(&mut self, uri: &str, method: TMethod, handler: F)
// //         -> anyhow::Result<()>
// //     where
// //         F: for<'r> Fn(Self::Req<'r>) -> anyhow::Result<()>
// //             + Send
// //             + Sync
// //             + 'static;
// // // }
// // // pub trait HttpServer {
// // //     fn on<F>(
// // //         &mut self,
// // //         uri: &str,
// // //         method: TMethod,
// // //         handler: F,
// // //     ) -> anyhow::Result<()>
// // //     where
// // //         F: Fn(Request) -> anyhow::Result<()>
// // //             + Send
// // //             + Sync
// // //             + 'static;

    

//     fn init_common_pages(&mut self) -> anyhow::Result<()> {
//         self.on("/main.css", TMethod::Get, |req| {
//             info!("Received {:?} request for {}", req.method(), req.uri());

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

