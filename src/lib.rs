use madsim::rand;
use rand::prelude::*;
use rand::Rng;
use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::sync::{Arc, Mutex};
use tokio::io::driver::Handle as ReactorHandle;
use tokio::runtime::Builder;
use tokio::task::JoinHandle;
use tokio::time::driver::Driver as TimeDriver;
pub struct CustomRuntime {
    scheduler: Arc<Mutex<dyn LotteryScheduler>>,
    reactor_handle: ReactorHandle,
    time_driver: TimeDriver,
}
pub trait LotteryScheduler {
    fn add_task(&mut self, task: Task, tickets: u32);
    fn remove_task(&mut self, task_id: TaskId) -> Option<Task>;
    fn select_next_task(&mut self) -> Option<Task>;
}

struct LotterySchedulerImpl {
    tasks: HashMap<TaskId, (Task, u32)>,
}

impl LotteryScheduler for LotterySchedulerImpl {
    fn select_next_task(&mut self) -> Option<Task> {
        let total_tickets: u32 = self.tasks.values().map(|(_, tickets)| *tickets).sum();

        if total_tickets == 0 {
            return None;
        }

        let ticket = rand::thread_rng().gen_range(0..total_tickets);
        let mut current_ticket: u32 = 0;

        for (task_id, (task, tickets)) in &self.tasks {
            current_ticket += tickets;

            if ticket < current_ticket {
                let task = self.remove_task(*task_id).unwrap();
                return Some(task);
            }
        }

        None
    }
}

impl CustomRuntime {
    // ...
    pub fn run(&self) {
        loop {
            // Run the reactor to poll I/O events
            self.reactor_handle.turn(None);

            // Run the time driver to manage timers
            self.time_driver.turn(None);

            // Select the next task from the scheduler
            let mut scheduler = self.scheduler.lock().unwrap();
            if let Some(task) = scheduler.select_next_task() {
                // Run the task
                let _ = task.run();
            }
        }
    }

    pub fn spawn<F>(&self, future: F, tickets: u32) -> TaskId
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let task = Task::new(future);
        let task_id = task.id();
        let mut scheduler = self.scheduler.lock().unwrap();
        scheduler.add_task(task, tickets);
        task_id
    }
}
