use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub bind_addr: String,
    pub lex_db_path: String,
    pub term_db_path: String,
}

impl Config {
    pub fn from_env() -> Self {
        let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:5005".to_string());
        let lex_db_path = env::var("LEX_DB_PATH").unwrap_or_else(|_| "./data/lexicon.sqlite".to_string());
        let term_db_path = env::var("TERM_DB_PATH").unwrap_or_else(|_| "./data/terminology.sqlite".to_string());

        Self { bind_addr, lex_db_path, term_db_path }
    }

    pub fn lex_db_url(&self) -> String {
        format!("sqlite:{}?mode=ro", self.lex_db_path)
    }

    pub fn term_db_url(&self) -> String {
        format!("sqlite:{}?mode=ro", self.term_db_path)
    }
}


