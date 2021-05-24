use bytes::Bytes;
use hyper::{
    body::to_bytes,
    service::{make_service_fn, service_fn},
    Body, Request, Server
};
use route_recognizer::Params;
use router::Router;
use std::sync::Arc;
use crate::repository::{PgsqlRepository, Repository};
use futures::executor::block_on;

mod handler;
mod router;
mod repository;

type Response = hyper::Response<hyper::Body>;
type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

pub struct AppState {
    pub repository: Arc<PgsqlRepository>
}

#[tokio::main]
async fn main() {
    let mut router: Router = Router::new();
    router.get("/contacts/:id", Box::new(handler::get_contact));

    let shared_router = Arc::new(router);
    let new_service = make_service_fn(move |_| {
        let app_state = Arc::new(AppState {
            repository: Arc::new(block_on(PgsqlRepository::new("host=postgresql user=classe password=classe dbname=classe")))
        });

        let router_capture = shared_router.clone();
        async {
            Ok::<_, Error>(service_fn(move |req| {
                route(router_capture.clone(), req, Arc::clone(&app_state))
            }))
        }
    });

    let addr = "0.0.0.0:8080".parse().expect("address creation works");
    let server = Server::bind(&addr).serve(new_service);
    println!("Listening on http://{}", addr);
    let _ = server.await;
}

async fn route(
    router: Arc<Router>,
    req: Request<hyper::Body>,
    app_state: Arc<AppState>,
) -> Result<Response, Error> {
    let found_handler = router.route(req.uri().path(), req.method());
    let resp = found_handler
        .handler
        .invoke(Context::new(app_state, req, found_handler.params))
        .await;
    Ok(resp)
}

pub struct Context {
    pub state: Arc<AppState>,
    pub req: Request<Body>,
    pub params: Params,
    body_bytes: Option<Bytes>,
}

impl Context {
    pub fn new(state: Arc<AppState>, req: Request<Body>, params: Params) -> Context {
        Context {
            state,
            req,
            params,
            body_bytes: None,
        }
    }

    pub async fn body_json<T: serde::de::DeserializeOwned>(&mut self) -> Result<T, Error> {
        let body_bytes = match self.body_bytes {
            Some(ref v) => v,
            _ => {
                let body = to_bytes(self.req.body_mut()).await?;
                self.body_bytes = Some(body);
                self.body_bytes.as_ref().expect("body_bytes was set above")
            }
        };
        Ok(serde_json::from_slice(&body_bytes)?)
    }
}
