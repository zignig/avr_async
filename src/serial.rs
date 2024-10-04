//! Serial port function.

use arduino_hal::hal::usart::Event;
use avr_device::interrupt::Mutex;

use core::{
    cell::RefCell,
    future::poll_fn,
    task::Poll,
};

use crate::{
    executor::ExtWaker,
    isrqueue::{self, ISRQueue},
    queue
};

// Global stuff
pub type Usart = arduino_hal::hal::usart::Usart0<arduino_hal::DefaultClock>;
pub static GLOBAL_SERIAL: Mutex<RefCell<Option<Usart>>> = Mutex::new(RefCell::new(None));

// Serial for print
pub fn serial_init(mut serial: Usart) {
    // Enable the interrupt to uart rcx
    serial.listen(Event::RxComplete);
    // Bind  the serial port for print! macro
    // TODO this should be task; don't delay the works
    avr_device::interrupt::free(|cs| {
        GLOBAL_SERIAL.borrow(cs).replace(Some(serial));
    });
}

// Print macro for outputting stuff.
#[macro_export]
macro_rules! print{
        ($($arg:tt)*) => {
            ::avr_device::interrupt::free(|cs| {
                if let Some(serial) = &mut *crate::serial::GLOBAL_SERIAL.borrow(cs).borrow_mut() {
                    let _ = ::ufmt::uwriteln!(serial, $($arg)*);
                }
            })
        }
    }


// Serial incoming stuff
// Don't know what i'm doing with this , making it up
// This queue is mutexed so it can be used inside a interrupt

pub static INCOMING_QUEUE: ISRQueue<u8, 16> = ISRQueue::new();

// Async needs internal state.
enum SerialState {
    Init,
    Wait,
}

pub struct SerialIncoming<'a> {
    state: SerialState,
    task_id: usize,
    incoming: isrqueue::Receiver<'a, u8, 16>,
}

impl<'a> SerialIncoming<'a> {

    pub fn new() -> Self {
        Self {
            state: SerialState::Init,
            task_id: 0xFFFF, // get the task id later.
            incoming: INCOMING_QUEUE.get_receiver(),
        }
    }

    // This is mostly to bind the incoming queue
    // to the task.
    pub async fn setup(&mut self){
        poll_fn(|cx| {
            match self.state {
                SerialState::Init => {
                    print!("Setup serial incoming");
                    // Set own task id
                    self.task_id = cx.waker().task_id();
                    self.state = SerialState::Wait;
                    print!("Finished Setup");
                    Poll::Ready(())
                }
                SerialState::Wait => {
                    Poll::Ready(())
                }
            }
        })
        .await
    }

    // Bridge the interrupt into an interal queue.
    pub async fn task(&mut self,mut outgoing: queue::Sender<'_,u8,16>) {
        self.setup().await;
        loop {
            let val = self.incoming.receive().await;
            outgoing.send(val);
        }
    }
}


// Whenever there is a char on the serial port run this.
// it will put a byte into the queue and notifty the sender.
#[avr_device::interrupt(atmega328p)]
fn USART_RX() {
    avr_device::interrupt::free(|_cs| {
        // This is a bit cheeky , but this is the only place that uses rx
        // it is in a critical section , could borrow but don't want to
        // waste cycles in a interrupt.
        let serial_port = unsafe { &*arduino_hal::pac::USART0::ptr() };
        let ch = serial_port.udr0.read().bits();
        INCOMING_QUEUE.send(ch);
    });
}
