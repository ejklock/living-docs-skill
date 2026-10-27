//! The JSON shape of the effective view: `{"withheld":N,"records":[...]}`.

use super::{Options, View};
use serde::Serialize;

#[derive(Serialize)]
struct RecordJson {
    #[serde(rename = "type")]
    doc_type: String,
    number: Option<i32>,
    title: String,
    description: String,
    lineage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    distance: Option<usize>,
}

#[derive(Serialize)]
struct EffectiveJson {
    withheld: usize,
    records: Vec<RecordJson>,
}

pub(super) fn json_of(withheld: usize, views: Vec<View>, options: &Options) -> String {
    let records = views
        .into_iter()
        .map(|view| RecordJson {
            doc_type: view.doc_type,
            number: view.number,
            title: view.title,
            description: view.description,
            lineage: view.lineage,
            body: (options.full || options.contract).then_some(view.body),
            distance: view.distance,
        })
        .collect();
    serde_json::to_string(&EffectiveJson { withheld, records })
        .unwrap_or_else(|err| format!("{{\"error\":\"failed to serialize JSON: {err}\"}}"))
}
