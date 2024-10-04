/// Async executor for avr
/// Stolen from https://github.com/therustybits/zero-to-async
/// and converted.
/// excellent video https://www.youtube.com/watch?v=wni5h5vIPhU
///

/// Converting to bitmask waker (like lilos)
/// Has the advantage that multiple wakes in the loop don't fill up
/// a queue, the just set the bit that is already 1 to 1 ... ;)

use core::{
    future::Future,
    pin::Pin,
    task::{Context, RawWaker, RawWakerVTable, Waker},
};

use portable_atomic::{AtomicUsize, Ordering};

/// An alternative to storing the waker: just extract the task information
/// you're looking for via an extension trait that you can implement for `Waker`
/// Not a great general solution if you want to be compatible with other
/// executors, but for this project it's fine.
pub trait ExtWaker {
    fn task_id(&self) -> usize;
}

impl ExtWaker for Waker {
    fn task_id(&self) -> usize {
        // When "waker-getters" is stabilized, do this instead:
        // self.as_raw().data() as usize
        for task_id in 0..NUM_TASKS.load(Ordering::Relaxed) {
            if get_waker(task_id).will_wake(self) {
                return task_id;
            }
        }
        panic!("Unknown waker/executor!");
    }
}

fn get_waker(task_id: usize) -> Waker {
    // SAFETY:
    // Data argument interpreted as an integer, not dereferenced
    unsafe { Waker::from_raw(RawWaker::new(task_id as *const (), &VTABLE)) }
}

// Vector table

static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);

unsafe fn clone(p: *const ()) -> RawWaker {
    RawWaker::new(p, &VTABLE)
}

unsafe fn drop(_p: *const ()) {}

unsafe fn wake(p: *const ()) {
    wake_task(p as usize);
}

unsafe fn wake_by_ref(p: *const ()) {
    wake_task(p as usize);
}

pub fn wake_task(task_id: usize) {
    let _ = TASK_MASK.bit_set(task_id as u32, Ordering::SeqCst);
}

const fn mask_for_index(index: usize) -> usize {
    1_usize.rotate_left(index as u32)
}

// End Vector Table 


// Bit mask for the tasks. (max 16 tasks for AVR)
static TASK_MASK: AtomicUsize = AtomicUsize::new(0);

// Number of tasks in application
static NUM_TASKS: AtomicUsize = AtomicUsize::new(0);


// Run the things, start all to register wakers.
pub fn run_tasks(tasks: &mut [Pin<&mut dyn Future<Output = ()>>]) -> ! {
    // Max 16 task for AVR
    NUM_TASKS.store(tasks.len(), Ordering::Relaxed);

    // everybody gets one run to start...
    // Set all the bits to 1 (run everything first time)
    // all the tasks initialize. (async needs to make wakers)
    // this will build wakers for all the tasks.
    crate::print!("Starting Executor");
    for task_id in 0..tasks.len() {
        crate::print!("task {} starting",task_id);
        let _ = TASK_MASK.bit_set(task_id as u32, Ordering::SeqCst);
    }
    
    crate::print!("running");
    
    // run all the tasks
    // using a bit field means that for each cycle all the flagged tasks get run
    // in order... so the declared order of tasks matters.
    // most of the time it will just get on with business.
    // TODO get some cycle stats.

    loop {
        let mask = TASK_MASK.load(Ordering::SeqCst);
        // Is there anything to do ? 
        if mask != 0 {
            for (task_id, task) in tasks.iter_mut().enumerate() {
                if mask & mask_for_index(task_id) != 0 {
                    // Clear the wake bit for the task
                    // Task may wake them selves up , do this before running.
                    let _ = TASK_MASK.bit_clear(task_id as u32, Ordering::SeqCst);
                    let _ = task
                        .as_mut()
                        .poll(&mut Context::from_waker(&get_waker(task_id)));
                }
            }
        }
        // go to sleep, wait for next interrupt.
        avr_device::asm::sleep();
    }
}
