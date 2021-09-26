use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::process::exit;
use std::str;

use actix_web::{App, HttpRequest, HttpResponse, HttpServer, middleware::Logger, web};
use chrono::Local;
use clap;
use clap::{AppSettings, Clap};
use curl::easy::Easy;
use env_logger::Builder;
use jq_rs;
use log::LevelFilter;
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

fn do_curl_and_jq(path_config: PathConfig) -> Result<String, String> {
    let mut easy = Easy::new();

    // Set the URL.
    easy.url(&path_config.source_url).unwrap();
    easy.fail_on_error(true).unwrap();

    // Curl for the data.
    let mut body_string = String::new();
    {
        let mut transfer = easy.transfer();
        transfer
            .write_function(|data| {
                body_string.push_str(str::from_utf8(data).unwrap());
                Ok(data.len())
            })
            .unwrap(); // Always returns Ok()

        transfer.perform()
            .map_err(|e| format!("Failed to run curl: {}", e))?;
    }

    // Query the data.
    return Ok(jq_rs::run(&path_config.jq_filter, &body_string)
        .map_err(|e| format!("Failed to run jq: {}", e))?
    );
}

fn proxy(req: HttpRequest, data: web::Data<AppConfig>) -> HttpResponse {
    // Path will exist as the only paths configured are based on the app context data.
    let path_config = data.paths.get(req.path())
        .unwrap()
        .clone();

    // HttpResponse::new()
    return match do_curl_and_jq(path_config) {
        Ok(data) => HttpResponse::Ok()
            .content_type("application/json")
            .body(data),
        Err(message) => HttpResponse::InternalServerError()
            .json(ErrorResponse {
                is_error: true,
                message,
            })
    }
}

fn parse_config(config_file_path: String) -> Result<AppConfig, String> {
    log::debug!("Reading config file {}.", config_file_path);
    let config_file_contents = fs::read_to_string(config_file_path)
        .map_err(|e| format!("Config read error: {}", e))?;

    log::debug!("Parse config file.");
    let config: AppConfig = serde_yaml::from_str(&config_file_contents)
        .map_err(|e| format!("Config parse error: {}", e))?;

    return match config.paths.is_empty() {
        true => Err("No paths configured in config file.".to_string()),
        false => Ok(config),
    };
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Configure logging output.
    Builder::new()
        .format(|buf, record| {
            writeln!(buf,
                     "{} [{}] - {}",
                     Local::now().format("%Y-%m-%dT%H:%M:%S"),
                     record.level(),
                     record.args()
            )
        })
        .filter(None, LevelFilter::Info)
        .init();

    log::debug!("Parsing CLI args.");
    let cli_args: CliArgs = CliArgs::parse();

    let config_result = parse_config(cli_args.config_file_path);
    if config_result.is_err() {
        eprintln!("{}", config_result.unwrap_err());
        exit(1);
    }

    let config = config_result.unwrap();

    println!("Configured paths:");
    (&config.paths).into_iter().for_each(|item| {
        println!("  {} -> {}", item.0, item.1.source_url);
    });

    let moved_config = config.clone();
    HttpServer
        ::new(move || {
            let mut app = App::new()
                .wrap(Logger::new("%r"))
                .app_data(web::Data::new(moved_config.clone()));

            // Configure the paths.
            for (path, _path_config) in &(moved_config.paths) {
                app = app.service(web::resource(path).route(
                    web::get().to(proxy)
                ));
            }

            return app;
        })
        .bind(config.listen)?
        .run()
        .await
}
