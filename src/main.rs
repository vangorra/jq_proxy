use std::collections::HashMap;
use std::fs;
use std::process::exit;
use std::str;

use actix_web::{App, HttpRequest, HttpResponse, HttpServer, web};
use clap;
use clap::{AppSettings, Clap};
use curl::easy::Easy;
use jq_rs;
use serde::Deserialize;
use serde::Serialize;
use serde_yaml;

#[derive(Serialize)]
struct ErrorResponse {
    is_error: bool,
    message: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
struct PathConfig {
    source_url: String,
    jq_filter: String,
}

fn default_listen() -> String {
    "0.0.0.0:8080".to_string()
}

fn default_paths() -> HashMap<String, PathConfig> {
    HashMap::new()
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
struct AppConfig {
    #[serde(default = "default_listen")]
    listen: String,

    #[serde(default = "default_paths")]
    paths: HashMap<String, PathConfig>,
}

#[derive(Clap)]
#[clap(setting = AppSettings::ColoredHelp)]
struct CliArgs {
    #[clap(short, long)]
    config_file_path: String,
}

async fn proxy(req: HttpRequest, data: web::Data<AppConfig>) -> HttpResponse {
    if !data.paths.contains_key(req.path()) {
        return HttpResponse::NotFound()
            .json(ErrorResponse {
                is_error: true,
                message: format!("Path not configured")
            });
    }

    let path_config = data.paths.get(req.path()).unwrap().clone();
    let mut easy = Easy::new();

    let url_result = easy.url(&path_config.source_url);
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

    let jq_result = jq_rs::run(&path_config.jq_filter, &body_string);
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

    let cli_args: CliArgs = CliArgs::parse();

    let config_file_contents_result = fs::read_to_string(cli_args.config_file_path);
    if config_file_contents_result.is_err() {
        eprintln!("Config read error: {}", config_file_contents_result.unwrap_err());
        exit(1);
    }

    let config_result: Result<AppConfig, serde_yaml::Error> = serde_yaml::from_str(config_file_contents_result.unwrap().as_str());
    if config_result.is_err() {
        eprintln!("Config parse error: {}", config_result.unwrap_err());
        exit(1);
    }

    let config: AppConfig = config_result.unwrap();
    let listen = config.listen.clone();

    if config.paths.len() == 0 {
        eprintln!("No paths configured.");
        exit(1)
    }

    println!("Listening on address {}", listen);
    (&config.paths).into_iter().for_each(|item| {
        println!("  {} -> {}", item.0, item.1.source_url);
    });

    HttpServer
        ::new(move || {
            let config_clone = config.clone();
            return App::new()
                .app_data(web::Data::new(config.clone()))
                .configure(|cfg: &mut web::ServiceConfig| {
                    config_clone.paths.into_iter().for_each(|item| {
                        let path = item.0;

                        cfg.service(
                            web::resource(path).route(web::get().to(proxy))
                        );
                    });
                });
        })
        .bind(listen)?
        .run()
        .await
}
