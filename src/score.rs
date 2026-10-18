use serde::Deserialize;
use std::sync::mpsc::{Receiver, channel};
use std::thread;

const API_URL: &str = "https://rustgamescoreendpoint.ilampozhilvarman.workers.dev";

#[derive(Debug, Clone, Deserialize)]
pub struct ScoreEntry {
    pub name: String,
    pub score: u32,
}

pub fn submit_score(name: String, score: u32) {
    thread::spawn(move || {
        let res = ureq::post(API_URL).send_json(ureq::json!({ "name": name, "score": score }));
        if let Err(e) = res {
            eprintln!("score submit failed: {e}");
        }
    });
}

pub fn fetch_top_scores() -> Receiver<Vec<ScoreEntry>> {
    let (tx, rx) = channel();
    thread::spawn(move || match ureq::get(API_URL).call() {
        Ok(resp) => match resp.into_json::<Vec<ScoreEntry>>() {
            Ok(list) => {
                let _ = tx.send(list);
            }
            Err(e) => eprintln!("bad score json: {e}"),
        },
        Err(e) => eprintln!("score fetch failed: {e}"),
    });
    rx
}
