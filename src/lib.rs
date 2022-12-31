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
    log_request(&req);
    utils::set_panic_hook();

    // do middlware stuff here, then if the request is valid we can pass it to the router
    let req_is_valid: bool = true;
    if !req_is_valid {
        return Response::error("Invalid Request", 400);
    }

    let router = Router::new();
    router
        .get("/", |_, _| Response::ok("Hello from Workers!"))
        .get_async("/hello", |mut req, _ctx| async move {
            let data: serde_json::Value = match req.json().await {
                Ok(d) => d,
                Err(_) => {
                    return Response::error("Bad Request: No json", 400);
                }
            };
            let name = match data.get("name") {
                Some(n) => n,
                None => {
                    return Response::error("Bad Request: Missing name", 400);
                },
            };
            return Response::from_json(&json!({ "message": format!("hello, {}!", name) }));
        })
        .get("/worker-version", |_, ctx| {
            let version = ctx.var("WORKERS_RS_VERSION")?.to_string();
            Response::ok(version)
        })
        .post_async("/add", |req, ctx| async move {
            return add_value(req, ctx).await;
        })
        .get_async("/get", |req, ctx| async move {
            return get_value(req, ctx).await;
        })
        .run(req, env)
        .await
}
