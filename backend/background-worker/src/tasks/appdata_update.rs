use appstream_worker::AppstreamWorker;
use fang::{
    serde::{Deserialize, Serialize},
    typetag, FangError, Queueable, Runnable, Scheduled,
};

#[derive(Serialize, Deserialize)]
pub struct AppdataUpdate {}

impl AppdataUpdate {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for AppdataUpdate {
    fn default() -> Self {
        Self::new()
    }
}

#[typetag::serde]
impl Runnable for AppdataUpdate {
    fn run(&self, _queue: &dyn Queueable) -> Result<(), FangError> {
        AppstreamWorker::new().run_appstream_update();

        Ok(())
    }

    fn uniq(&self) -> bool {
        true
    }

    fn cron(&self) -> Option<Scheduled> {
        Some(Scheduled::CronPattern("30 0/5 * * * *".to_string()))
    }
}
