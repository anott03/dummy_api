use serde_json::json;
use worker::*;

mod utils;

fn log_request(req: &Request) {
    console_log!(
        "{} - [{}], located at: {:?}, within: {}",
        Date::now().to_string(),
        req.path(),
        req.cf().coordinates().unwrap_or_default(),
        req.cf().region().unwrap_or("unknown region".into())
    );
}

fn add_headers(res: std::result::Result<worker::Response, worker::Error>) -> std::result::Result<worker::Response, worker::Error> {
    return match res {
        Ok(mut res) => {
            let headers = res.headers_mut();
            if let Err(_) = headers.set("Access-Control-Allow-Origin", "*") {
                console_log!("error setting response headers");
            }
            Ok(res)
        },
        Err(res) => Err(res),
    };
}

async fn get_value(mut req: Request, ctx: RouteContext<()>) -> std::result::Result<worker::Response, worker::Error> {
    let data: serde_json::Value = match req.json().await {
        Ok(d) => d,
        Err(_) => { return Response::error("Bad request", 400); }
    };
    let key = match data.get("key") {
        Some(n) => n.to_string(),
        None => {
            return Response::error("Bad Request: Missing key", 400);
        },
    };
    let kv = ctx.kv("DUMMY_API")?;
    let value = match kv.get(&key).json::<String>().await {
        Ok(v) => v,
        Err(_err) => Some(String::new())
    };

    return match value {
        Some(val) => Response::from_json(&json!({ "success": true, "value": val.as_str() })),
        None => Response::from_json(&json!({ "success": false, "message": format!("no value fouund for key {}", key) }))
    };
}

async fn add_value(mut req: Request, ctx: RouteContext<()>) -> std::result::Result<worker::Response, worker::Error> {
    let data: serde_json::Value = match req.json().await {
        Ok(d) => d,
        Err(_) => { return Response::error("Bad request", 400); }
    };
    let key = match data.get("key") {
        Some(n) => n,
        None => {
            return Response::error("Bad Request: Missing key", 400);
        },
    };
    let value = match data.get("value") {
        Some(n) => n,
        None => {
            return Response::error("Bad Request: Missing value", 400);
        },
    };

    let kv = ctx.kv("DUMMY_API")?;
    kv.put(&key.to_string(), &value.to_string())?
        .execute()
        .await?;

    return Response::from_json(&json!({ "success": true }));
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    let _allowed_origins = vec![
        "http://localhost:3000",
        "http://localhost:3001",
        "https://localhost:3000",
        "https://localhost:3001",
    ];
 
    log_request(&req);
    utils::set_panic_hook();

    let headers = req.headers();
    console_log!("HEADERS {:?}", headers);

    // middleware-esque things
    // crashes the worker when failed (insomnia)
    // if let Ok(o) = req.headers().get("origin") {
        // let origin = o.unwrap();
        // if !allowed_origins.contains(&origin.as_str()) {
            // return add_headers(Response::error("invalid origin", 400));
        // }
    // } else {
        // return add_headers(Response::error("invalid origin", 400));
    // }

    let router = Router::new();
    router
        .get("/", |_, _| Response::ok("Hello from Workers!"))
        .get("/worker-version", |_, ctx| {
            let version = ctx.var("WORKERS_RS_VERSION")?.to_string();
            add_headers(Response::ok(version))
        })
        .post_async("/add", |req, ctx| async move {
            return add_headers(add_value(req, ctx).await);
        })
        .post_async("/get", |req, ctx| async move {
            return add_headers(get_value(req, ctx).await);
        })
        .get("/names", |_req, _ctx| {
            return add_headers(Response::from_json(&json!({
                "names": [{"first": "John", "last": "Doe"}, {"first": "Jane", "last": "Smith"}]
            })));
        })
        .run(req, env)
        .await
}
