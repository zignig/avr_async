/// Async executor for avr , timer queue
/// Stolen from https://github.com/therustybits/zero-to-async
/// chapter 6 and converted.
// TODO Should convert to timer1 (16bit)
// and use the overflow and value interrupt.
// This is a 64 bit counter , should overflow in 5162 , should be enough.

use core::{
    cell::RefCell,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use arduino_hal::pac::TC0;
use avr_device::interrupt::Mutex;
use fugit::{Duration, Instant};
use heapless::{binary_heap::Min, BinaryHeap};
use portable_atomic::{AtomicU64, Ordering};
use crate::executor::{wake_task, ExtWaker};

// fugit does ticks -> millis in compile time
// for prescale_64

pub type TickInstant = Instant<u64, 1, 984>;
pub type TickDuration = Duration<u64, 1, 984>;

// Make a heap for incoming timers
// If you run out of timers increase this number.
const MAX_DEADLINES: usize = 8;

static WAKE_DEADLINES: Mutex<RefCell<BinaryHeap<(u64, usize), Min, MAX_DEADLINES>>> =
    Mutex::new(RefCell::new(BinaryHeap::new()));


// Timers are used to do stuff in the future.

// With async you need internal state ...
enum TimerState {
    Init,
    Wait,
}

// A timer
pub struct Timer {
    end_time: TickInstant,
    state: TimerState,
}

impl Timer {
    // Make a new timer
    // these are 64 bit , excessive but it will _never_ run out.
    pub fn new(duration: TickDuration) -> Self {
        Self {
            end_time: Ticker::now() + duration,
            state: TimerState::Init,
        }
    }

    // Insert the timer into the heap
    pub fn register(&self, task_id: usize) {
        let new_deadline = self.end_time.ticks();
        // crack open the deadlines
        avr_device::interrupt::free(|cs| {
            let deadlines = &mut *WAKE_DEADLINES.borrow(cs).borrow_mut();
            if deadlines.push((new_deadline, task_id)).is_err() {
                // if you get here add more timers.
                panic!("Deadline dropped for task {}!", task_id);
            }
        });
    }
}

// Asyncy works for the time
// build a timer that will wakeup later.
impl Future for Timer {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.state {
            TimerState::Init => {
                self.register(cx.waker().task_id());
                self.state = TimerState::Wait;
                Poll::Pending
            }
            TimerState::Wait => {
                if Ticker::now() >= self.end_time {
                    Poll::Ready(())
                } else {
                    Poll::Pending
                }
            }
        }
    }
}

// Wait for later
pub async fn delay(duration: TickDuration) {
    Timer::new(duration).await;
}

// The timer exists as a static
// Static , so it stays for everyone.
static TICKER: Ticker = Ticker {
    ovf_count: AtomicU64::new(0),
};

// Attached to 8 bit timer, increments on overflow
pub struct Ticker {
    ovf_count: AtomicU64,
}

// Functions for the static ticker.
// Borrow the timer and hand it back
impl Ticker {
    pub fn init(timer: &TC0) {
        // overflow interrupt enable
        timer.timsk0.write(|w| w.toie0().set_bit());
        // prescaler ,  prescale_64 gives to 1 millisecond resolution
        // if you need more ... do it.0
        //timer.tccr0b.write(|w| w.cs0().direct());
        timer.tccr0b.write(|w| w.cs0().prescale_64());
    }

    // Get the current time, in milliseconds (thanks fugit)
    pub fn now() -> TickInstant {
        let ticks = TICKER.ovf_count.load(Ordering::SeqCst);
        TickInstant::from_ticks(ticks)
    }

    // Raw ticks
    pub fn ticks() -> u64 {
        TICKER.ovf_count.load(Ordering::SeqCst)
    }

    // Print out a list of the current timers.
    pub fn show_timers() {
        avr_device::interrupt::free(|cs| {
            let ticks = Ticker::ticks();
            let deadlines = &mut *WAKE_DEADLINES.borrow(cs).borrow_mut();
            for i in deadlines.iter() {
                // this might just be negative ( panic causing)
                let until: i32 = (i.0 as i32) - (ticks as i32);
                crate::print!("task {} in {} ticks ", i.1, until);
            }
        });
    }
}

// Do this every millisecond or so...
// for this application 1 millsecond resolution is ok
// if you want more change the prescaler.

// timer 2  is 16 bit , putting events in is a little more complicated
// with a horizon on next overflow; the logic is complicated.

#[avr_device::interrupt(atmega328p)]
fn TIMER0_OVF() {
    let ticks = TICKER.ovf_count.fetch_add(1, Ordering::SeqCst);
    avr_device::interrupt::free(|cs| {
        let deadlines = &mut *WAKE_DEADLINES.borrow(cs).borrow_mut();
        if let Some((next_deadline, task_id)) = deadlines.peek() {
            if ticks > *next_deadline {
                // There is a timer expired, run it
                // this does not account for lag.
                wake_task(*task_id);
                deadlines.pop();
            }
        }
    });
}
