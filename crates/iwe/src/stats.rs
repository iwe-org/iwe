pub use liwe::stats::{BrokenLink, GraphStatistics, KeyStatistics};

use minijinja::Environment;

const STATS_TEMPLATE: &str = include_str!("../templates/stats.md.jinja");

pub fn render_stats(stats: &GraphStatistics) -> String {
    let mut env = Environment::new();
    env.add_template("stats", STATS_TEMPLATE)
        .expect("Failed to add template");

    let template = env.get_template("stats").expect("Failed to get template");
    template.render(stats).expect("Failed to render template")
}
