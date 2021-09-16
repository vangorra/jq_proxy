use std::str;

use actix_web::{App, get, HttpResponse, HttpServer, web};
use curl::easy::Easy;
use jq_rs;
use serde::Deserialize;
use serde::Serialize;

#[derive(Deserialize)]
struct ProxyArgs {
    url: String,
    query: String,
}

#[derive(Serialize)]
struct ErrorResponse {
    is_error: bool,
    message: String,
}

#[get("/")]
async fn proxy(proxy_args: web::Query<ProxyArgs>) -> HttpResponse {
    let mut easy = Easy::new();
    let url_result = easy.url(&proxy_args.url);
    if url_result.is_err() {
        return HttpResponse::InternalServerError()
            .json(ErrorResponse {
                is_error: true,
                message: format!("Failed to retrieve from URL: {}", url_result.unwrap_err()),
            })
    }

    let mut body_string = String::new();
    {
        let mut transfer = easy.transfer();
        transfer.write_function(|data| {
            body_string.push_str(str::from_utf8(data).unwrap());
            Ok(data.len())
        }).unwrap();

        let transfer_perform_result = transfer.perform();
        if transfer_perform_result.is_err() {
            return HttpResponse::BadRequest()
                .json(ErrorResponse {
                    is_error: true,
                    message: format!("{}", transfer_perform_result.unwrap_err())
                })
        }
    }

    let jq_result = jq_rs::run(&proxy_args.query, &body_string);
    if jq_result.is_err() {
        return HttpResponse::InternalServerError()
            .json(ErrorResponse {
                is_error: true,
                message: format!("Failed to run JQ: {}", jq_result.unwrap()),
            })
    }

    return HttpResponse::Ok()
        .content_type("application/json")
        .body(jq_result.unwrap())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let listen_addr = "0.0.0.0:8080";
    println!("Starting server on http://{}", listen_addr);
    HttpServer::new(|| App::new().service(proxy))
        .bind(listen_addr)?
        .run()
        .await
}