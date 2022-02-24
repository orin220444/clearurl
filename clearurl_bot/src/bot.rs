use clearurl::UrlCleaner;
use log::error;
use std::sync::Arc;
use teloxide::{dispatching2::UpdateFilterExt, prelude2::*, types::Update, RequestError};

#[derive(Clone)]
struct Config {
    enable_groups: Arc<Vec<i64>>,
}

impl Config {
    pub fn is_enabled_group(&self, g: i64) -> bool {
        for group in self.enable_groups.iter() {
            if g == *group {
                return true;
            }
        }

        false
    }
}

async fn handle_link_message(
    msg: Message,
    bot: AutoSend<Bot>,
    cleaner: Arc<UrlCleaner>,
    regex: Arc<regex::Regex>,
) -> Result<(), RequestError> {
    let caps = regex.captures_iter(msg.text().unwrap_or(""));
    let mut buffer = String::new();
    for cap in caps {
        let url = &cap[1];
        let url = match cleaner.clear(url).await {
            Ok(u) => u,
            Err(e) => {
                error!("{}", e);
                return respond(());
            }
        };
        buffer.push_str(url.as_str());
        buffer.push('\n');
    }

    if !buffer.is_empty() {
        let resp = bot.send_message(msg.chat_id(), buffer).await;
        if let Err(e) = resp {
            error!("{}", e);
        }
    }
    respond(())
}

fn build_runtime() -> (AutoSend<Bot>, Arc<UrlCleaner>, Arc<regex::Regex>) {
    let bot = Bot::from_env().auto_send();
    let cleaner = Arc::new(UrlCleaner::from_file("../rules.toml").unwrap());
    let http_regex_rule = Arc::new(
        regex::Regex::new(
            r"(http[s]?://(?:[a-zA-Z]|[0-9]|[$-_@.&+]|[!*\(\),]|(?:%[0-9a-fA-F][0-9a-fA-F]))+)",
        )
        .expect("Illegal http regex rule"),
    );

    (bot, cleaner, http_regex_rule)
}

fn build_handler() -> Handler<'static, DependencyMap, Result<(), RequestError>> {
    Update::filter_message().branch(
        dptree::filter(|msg: Message, cfg: Config| {
            msg.text().is_some() && cfg.is_enabled_group(msg.chat_id())
        })
        .endpoint(handle_link_message),
    )
}

pub async fn run() {
    teloxide::enable_logging!();
    dotenv::dotenv().ok();
    log::info!("Starting bot...");

    let bot_config = Config {
        enable_groups: Arc::new(vec![]),
    };

    let (bot, cleaner, http_regex_rule) = build_runtime();

    Dispatcher::builder(bot, build_handler())
        .dependencies(dptree::deps![bot_config, http_regex_rule, cleaner])
        .default_handler(|_| async move {})
        .error_handler(LoggingErrorHandler::with_custom_text("Fail to handle"))
        .build()
        .setup_ctrlc_handler()
        .dispatch()
        .await;
}
