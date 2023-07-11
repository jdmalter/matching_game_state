use crate::runtime::{Config, State};

mod name;
mod runtime;

fn t() {
    let config = Config::default(4);
    let (state, max_matches) = State::new(config);
}
