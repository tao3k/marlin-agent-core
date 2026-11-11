//! Opt-in measurements; emitted after the measured work and VM shutdown.
#[derive(Default)]
pub(super) struct VmStages {
    pub input_us: u128,
    pub submit_us: u128,
    pub receive_us: u128,
    pub output_us: u128,
}
pub(super) struct Turn {
    pub elapsed_us: u128,
    pub submitted: usize,
    pub completed: Vec<(u64, u128)>,
    pub gc_count: f64,
    pub gc_wall_us: f64,
    pub heap_live_bytes: f64,
    pub stages: VmStages,
}
impl Turn {
    pub fn emit(&self) {
        eprintln!(
            "CONTEXT-TURN elapsed_us={} submitted={} completed={:?} gc_count={} gc_wall_us={:.3} heap_live_bytes={} input_us={} submit_us={} receive_us={} output_us={}",
            self.elapsed_us,
            self.submitted,
            self.completed,
            self.gc_count,
            self.gc_wall_us,
            self.heap_live_bytes,
            self.stages.input_us,
            self.stages.submit_us,
            self.stages.receive_us,
            self.stages.output_us
        );
    }
}
