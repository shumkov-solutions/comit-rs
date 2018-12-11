use serde_json;
#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate log;

use memsocket;
use pretty_env_logger;

#[macro_use]
pub mod common;

use crate::common::{setup::start_server_with, *};
use bam::{json::*, *};
use futures::*;
use std::{collections::HashMap, time::Duration};

#[test]
fn do_something_on_response() {
    let (mut _runtime, alice, mut _bob) = start_server_with(say_hello::config());

    let future = _bob._alice.send_request(Request::new(
        "PING".into(),
        HashMap::new(),
        serde_json::Value::Null,
    ));

    // Wait for the request to get dispatched
    ::std::thread::sleep(Duration::from_millis(100));

    // Send the response on the socket
    alice
        .send_with_newline(include_json_line!("empty_response_id-0_ok00.json"))
        .wait()
        .unwrap();

    assert_eq!(future.wait(), Ok(Response::new(Status::OK(0))));
}
