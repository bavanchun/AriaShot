use camerashot_editor::timeline_ctrl::VideoTimeline;

fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("Starting Camerashot Tier 2 Standalone Editor...");

    let timeline = VideoTimeline::new(10.0);
    tracing::info!(
        "Initialized timeline with duration {}s, kept ranges: {:?}",
        timeline.composition_duration(),
        timeline.kept_ranges()
    );

    println!("Camerashot Tier 2 Editor runtime ready.");
}
