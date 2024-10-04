// This is an Async rewrite of the newdrive
// https://github.com/zignig/steves_brain/tree/main/rover/newdrive
// the executor is based on chapter 6 of
// https://github.com/therustybits/zero-to-async

// Robot rewrite in async

#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

use arduino_hal::{
    hal::port::PB5,
    port::{mode::Output, Pin},
};
use core::pin::pin;
use fugit::ExtU64;
use panic_halt as _;

mod channel;
mod drive;
mod executor;
mod isrqueue;
mod queue;
mod serial;
mod time;

use channel::Channel;
use drive::{Drive, DriveCommands};
use executor::run_tasks;
use queue::Queue;
use time::{delay, TickDuration, Ticker};
use crate::serial::SerialIncoming;

#[arduino_hal::entry]
fn main() -> ! {
    // grab the stuff from the micro
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    // Serial Port setup

    let serial_port = arduino_hal::default_serial!(dp, pins, 115200);
    // bind the serial port to the macro in utils so it can be used anywhere
    serial::serial_init(serial_port);

    print!("Activate the awsome!");

    // Set up timer 0 for system clock
    Ticker::init(&dp.TC0);

    // Led (blinky!)
    let led = pins.d13.into_output();

    // Some Test tasks

    // Just blink the LED to show that it's running
    // SPI pins takes this ! WATCH OUT !
    let blink = pin!(blinker(led, 50.millis()));

    // See that it's running on the serial console
    let t1 = pin!(show_name(2.secs(), "boop!"));

    // Longer task , perhaps shut everything down and go to low power mode
    let t3 = pin!(show_name(5.secs(), "check idle"));

    // Really long taks 
    let t4 = pin!(show_name(24.hours(),"hello"));

    // Show the current timer queue for debug (for now)
    let show = pin!(show_time());

    // Make a new Drive task
    // Make a comms channel to the motor
    let drive_commands: Channel<DriveCommands> = Channel::new();

    // Create  the drive
    // TODO get the config from eeprom
    let mut drive = Drive::new(drive::DriveConfig::new());
    // extract the task
    let drive_task = pin!(drive.task(drive_commands.get_receiver()));

     // Serial command system.
    let serial_chars: Queue<u8, 16> = Queue::new();
    let mut serial_incoming = SerialIncoming::new();
    let serial_task = pin!(serial_incoming.task(serial_chars.get_sender()));

    // Push The commands into another task
    let command_out = pin!(make_commands(
        serial_chars.get_receiver(),
        drive_commands.get_sender()
    ));

   // DRAGONS! beware , unsafe code.
    unsafe { avr_device::interrupt::enable() };

    // Main Executor (asyncy goodness)
    // Run all the defined tasks
    loop {
        run_tasks(&mut [
            t1,
            t3,
            t4,
            blink,
            drive_task,
            serial_task,
            command_out,
            show,
        ]);
    }
}

// Serial input to control the micro
// react to incoming serial events
// mainly used for testing , interactivty.
async fn make_commands(
    mut rec: queue::Receiver<'_, u8, 16>,
    drive_commands: channel::Sender<'_, DriveCommands>,
) {
    loop {
        let val = rec.receive().await;
        match val {
            b'w' => drive_commands.send(DriveCommands::Forward),
            b's' => drive_commands.send(DriveCommands::Backwards),
            b'a' => drive_commands.send(DriveCommands::Left),
            b'd' => drive_commands.send(DriveCommands::Right),
            b' ' => drive_commands.send(DriveCommands::Stop),
            _ => print!("{}", val.to_ascii_lowercase()),
        }
    }
}

// Just blink the led
async fn blinker(mut led: Pin<Output, PB5>, interval: TickDuration) {
    loop {
        delay(interval).await;
        led.toggle();
    }
}

// Largely for testing boop a name onto the serial console.
async fn show_name(interval: TickDuration, blurb: &str) {
    loop {
        delay(interval).await;
        print!("{}", blurb);
    }
}


fn divider() {
    print!("-----------");
}

// Show the current timers in stack
async fn show_time() {
    loop {
        delay(5.secs()).await;
        divider();
        print!("time: {}", Ticker::now().duration_since_epoch().to_secs());
        Ticker::show_timers();
        divider();
    }
}