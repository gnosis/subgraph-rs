use subgraph::log;

#[export_name = "greatOnTurning"]
pub extern "C" fn great_on_turning(event: *const ()) {
    log::info!("[greatOnTurning] Hello from Rust 🦀!");
    todo!("event pointer: {:?}", event);
}

#[export_name = "dayOfTheAnswer"]
pub extern "C" fn day_of_the_answer(_: *const ()) {
    todo!();
}
