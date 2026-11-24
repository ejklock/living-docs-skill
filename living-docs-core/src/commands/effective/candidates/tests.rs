use super::*;

fn numbers(found: &[Candidate]) -> Vec<i32> {
    found.iter().map(|c| c.number).collect()
}

/// `total` ADRs numbered from 1; those in `with` carry `text` as their body,
/// the others carry a body no query word reaches.
fn corpus(total: i32, with: &[i32], text: &'static str) -> Vec<AdrText<'static>> {
    (1..=total)
        .map(|number| AdrText {
            number,
            title: "Filler",
            description: "d",
            body: if with.contains(&number) {
                text
            } else {
                "unrelated"
            },
        })
        .collect()
}

#[test]
#[rustfmt::skip]
fn the_stop_list_is_exactly_the_contracted_words() {
    let contracted = [
        "about", "after", "again", "against", "also", "always", "among", "because", "been",
        "before", "being", "between", "both", "cannot", "could", "does", "done", "down",
        "during", "each", "either", "else", "even", "ever", "every", "from", "further", "have",
        "having", "here", "hers", "herself", "himself", "into", "itself", "just", "keep", "like",
        "made", "make", "many", "more", "most", "much", "must", "never", "none", "once", "only",
        "other", "others", "otherwise", "ours", "over", "same", "shall", "should", "since",
        "some", "such", "than", "that", "their", "theirs", "them", "then", "there", "these",
        "they", "this", "those", "though", "through", "thus", "under", "until", "upon", "used",
        "uses", "using", "very", "want", "wants", "were", "what", "when", "whenever", "where",
        "whether", "which", "while", "whom", "whose", "will", "with", "within", "without",
        "would", "your", "yours",
    ];

    let actual: Vec<&str> = STOP_WORDS.split_whitespace().collect();

    assert_eq!(actual, contracted);
}

#[test]
fn a_word_in_exactly_three_percent_of_a_hundred_is_rare_and_one_more_is_not() {
    let at = corpus(100, &[1, 2, 3], "zephyr quartz");
    let over = corpus(100, &[1, 2, 3, 4], "zephyr quartz");

    assert_eq!(numbers(&candidates("zephyr quartz", &at)), [1, 2, 3]);
    assert!(candidates("zephyr quartz", &over).is_empty());
}

#[test]
fn a_small_bundle_keeps_a_floor_of_two_records() {
    let at = corpus(20, &[1, 2], "zephyr quartz");
    let over = corpus(20, &[1, 2, 3], "zephyr quartz");

    assert_eq!(numbers(&candidates("zephyr quartz", &at)), [1, 2]);
    assert!(candidates("zephyr quartz", &over).is_empty());
}

#[test]
fn one_shared_rare_word_lists_nothing_and_two_list_the_record() {
    let adrs = corpus(10, &[4], "zephyr quartz");

    assert!(candidates("zephyr", &adrs).is_empty());
    let found = candidates("zephyr quartz", &adrs);
    assert_eq!(numbers(&found), [4]);
    assert_eq!(found[0].shared, ["quartz", "zephyr"]);
}

#[test]
fn more_shared_words_come_first_then_the_lower_number() {
    let mut adrs = corpus(200, &[], "");
    adrs[8].body = "alpha bravo";
    adrs[4].body = "alpha bravo";
    adrs[6].body = "alpha bravo";
    adrs[20].body = "alpha bravo charlie";

    let found = candidates("alpha bravo charlie", &adrs);

    assert_eq!(numbers(&found), [21, 5, 7]);
}

#[test]
fn at_most_three_records_are_listed() {
    let adrs = corpus(200, &[1, 2, 3, 4, 5], "zephyr quartz");

    assert_eq!(numbers(&candidates("zephyr quartz", &adrs)), [1, 2, 3]);
}

#[test]
fn stop_words_and_short_words_are_ignored() {
    let adrs = corpus(10, &[1, 2], "with this that cat dog");

    assert!(candidates("with this that cat dog", &adrs).is_empty());
}

#[test]
fn a_non_ascii_letter_splits_a_word() {
    let adrs = corpus(10, &[3], "alpha\u{e9}bravo");

    assert_eq!(numbers(&candidates("alpha bravo", &adrs)), [3]);
    assert!(candidates("na\u{ef}ve caf\u{e9}", &adrs).is_empty());
}

#[test]
fn case_is_ignored_and_the_title_and_description_count() {
    let adrs = [AdrText {
        number: 7,
        title: "ZEPHYR routing",
        description: "Quartz",
        body: "x",
    }];

    let found = candidates("zephyr quartz", &adrs);

    assert_eq!(found[0].shared, ["quartz", "zephyr"]);
}

#[test]
fn the_output_is_byte_stable_whatever_the_input_order() {
    let adrs = corpus(40, &[2, 9, 30], "zephyr quartz");
    let mut reversed = corpus(40, &[2, 9, 30], "zephyr quartz");
    reversed.reverse();

    let first = serde_json::to_string(&candidates("zephyr quartz", &adrs)).unwrap();
    let again = serde_json::to_string(&candidates("zephyr quartz", &adrs)).unwrap();
    let flipped = serde_json::to_string(&candidates("zephyr quartz", &reversed)).unwrap();

    assert_eq!(first, again);
    assert_eq!(first, flipped);
}
