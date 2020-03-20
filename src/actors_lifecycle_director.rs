use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::sync::Arc;
use crate::system_director::SystemDirector;
use async_std::task;
use std::time::{Duration, SystemTime};

#[derive(Debug)]
pub(crate) struct ActorsLifecycleDirector {
	system_director: SystemDirector,
	is_running: Arc<AtomicBool>, 
	allowed_innactive_seconds: u64,
}

impl ActorsLifecycleDirector {
	pub fn new(system_director: SystemDirector, allowed_innactive_seconds: u64) -> ActorsLifecycleDirector {

		let is_running = Arc::new(AtomicBool::new(true));

		let director = ActorsLifecycleDirector {
			system_director,
			is_running: is_running.clone(),
			allowed_innactive_seconds
		};

		task::spawn(periodic_clean_loop(director.clone(), is_running));

		director
	}

	pub fn stop(&self) {
		self.is_running.store(false, Relaxed)
	}
}

async fn periodic_clean_loop(director: ActorsLifecycleDirector, is_running: Arc<AtomicBool>) {
	loop {

		let allowed_innactive_seconds = director.allowed_innactive_seconds;
		let now = SystemTime::now();

		director.system_director.end_filtered(Box::new(move |report| {

			let secs = match now.duration_since(report.last_message_on) {
				Ok(duration) => duration.as_secs(),
				// TODO: Check what to do in this case
				Err(_) => 0,
			};

			if secs > allowed_innactive_seconds {
				false
			} else {
				true
			}
		}));

		if !is_running.load(Relaxed) {
			break;
		}

		task::sleep(Duration::from_secs(5)).await;
	}
}

impl Clone for ActorsLifecycleDirector {
	fn clone(&self) -> ActorsLifecycleDirector {
		ActorsLifecycleDirector {
			system_director: self.system_director.clone(),
			is_running: self.is_running.clone(),
			allowed_innactive_seconds: self.allowed_innactive_seconds.clone()
		}
	}
}
