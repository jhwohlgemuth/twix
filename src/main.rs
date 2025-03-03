use clap::{Parser, Subcommand, ValueHint};
use indicatif::{ProgressBar, ProgressStyle};
use playwright::api::Page;
use playwright::Playwright;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;
use uriparse::URI;

const PROGRESS_BAR_TEMPLATE: &str = "{spinner:.green} {pos}/{len} [{bar:.green}] {msg}";

#[derive(Debug, Serialize, Deserialize)]
struct Following {
    following: FollowingData,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FollowingData {
    account_id: String,
    user_link: String,
}
#[derive(Debug, Parser)]
#[command(author, version, about, verbatim_doc_comment)]
struct Arguments {
    #[command(subcommand)]
    pub command: Option<Commands>,
}
#[derive(Debug, Subcommand)]
#[command(long_about = None)]
enum Commands {
    #[clap(verbatim_doc_comment)]
    Process {
        #[arg(short, long, value_name = "PATH", value_hint = ValueHint::FilePath)]
        following: Option<PathBuf>,
        #[arg(short, long, value_name = "LIST", value_delimiter = ',')]
        ids: Option<Vec<String>>,
    },
}
fn get_handle(page: Page) -> String {
    let url = page.url().unwrap();
    match URI::try_from(url.as_str()) {
        Ok(uri) => uri
            .query()
            .unwrap()
            .split("=")
            .collect::<Vec<&str>>()
            .last()
            .unwrap()
            .to_string(),
        Err(_) => "".to_string(),
    }
}
fn import_following(path: PathBuf) -> Vec<String> {
    let content = match read_file(path.clone()) {
        Ok(value) if !value.is_empty() => value.replace("window.YTD.following.part0 = ", ""),
        Ok(_) | Err(_) => "{}".to_owned(),
    };
    let parsed: serde_json::Result<Vec<Following>> = serde_json::from_str(&content);
    match parsed {
        Ok(values) => {
            let results: Vec<String> = values
                .into_iter()
                .map(|value| value.following.account_id)
                .collect();
            results.clone()
        }
        Err(_) => unimplemented!(),
    }
}
fn login(page: Page) {
    unimplemented!()
}
async fn process(values: Vec<String>) -> Vec<String> {
    let playwright = Playwright::initialize().await.unwrap();
    playwright.install_chromium().unwrap(); // Install browsers
    let chromium = playwright.chromium();
    let browser = chromium.launcher().headless(true).launch().await.unwrap();
    let context = browser.context_builder().build().await.unwrap();
    let page = context.new_page().await.unwrap();
    let mut handles = Vec::new();
    let progress = ProgressBar::new(values.len() as u64);
    progress.set_style(ProgressStyle::with_template(PROGRESS_BAR_TEMPLATE).unwrap());
    for value in values {
        progress.set_message(format!("Processing {}...", value));
        let url = format!("https://twitter.com/intent/user?user_id={}", value);
        page.goto_builder(&url).goto().await.unwrap();
        page.wait_for_selector_builder("div[data-testid = \"IntentLoginSheet_Login_Sheet\"]")
            .wait_for_selector()
            .await
            .unwrap();
        handles.push(get_handle(page.clone()));
        progress.inc(1);
    }
    progress.finish_with_message("All identifiers have been processed");
    let _close = page.close(Some(false)).await;
    handles.sort();
    handles
}
fn read_file(path: PathBuf) -> Result<String, Box<dyn Error>> {
    let mut content = String::new();
    let _ = match File::open(path.clone()) {
        Ok(mut file) => file.read_to_string(&mut content),
        Err(err) => Err(err),
    };
    Ok(content)
}

#[tokio::main]
async fn main() {
    let args = Arguments::parse();
    match &args.command {
        Some(Commands::Process { ids, following }) => {
            let input = match ids {
                Some(values) => values,
                None => match following {
                    Some(path) => &import_following(path.clone()),
                    None => unreachable!(),
                },
            };
            let handles = process(input.clone()).await;
            let mut file =
                File::create("following.txt").expect("Failed to create or open the file");
            file.write_all(handles.join("\n").as_bytes())
                .expect("Failed to write to the file");
        }
        None => (),
    };
}
