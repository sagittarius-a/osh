use log::LevelFilter;
use log4rs::append::file::FileAppender;
use log4rs::config::Config as LogConfig;
use log4rs::config::{Appender, Root};
use log4rs::encode::pattern::PatternEncoder;

pub fn setup_logging() {
    // https://docs.rs/log4rs/1.0.0/log4rs/encode/pattern/index.html
    let logfile = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{d(%Y-%m-%d %H:%M:%S)} :: {l} - {m}\n",
        )))
        .build("/tmp/shell.log")
        .unwrap();

    let config = LogConfig::builder()
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .build(
            Root::builder()
                .appender("logfile")
                .build(LevelFilter::Debug),
        )
        .unwrap();

    log4rs::init_config(config).unwrap();
}
