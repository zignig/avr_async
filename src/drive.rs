/// Mockup for the drive system
/// An example of a task that will run itself until it is finished
/// and then just wait for events.
///
///
use core::{future::poll_fn, task::Poll};

use crate::{
    channel,
    time::{TickDuration, Ticker},
};

use fugit::ExtU64;
use futures::{select_biased, FutureExt};

use crate::time;

// Overstate of the drive
#[derive(PartialEq)]
pub enum DriveState {
    Init,
    Running,
    SoftStop,
    Idle,
    Error,
}

// Driver commands
pub enum DriveCommands {
    Forward,
    Backwards,
    Left,
    Right,
    Stop,
}

// Represention of the dual drive
pub struct Drive {
    state: DriveState,
    timeout: TickDuration,
    next_timeout: u64,
    throttle: i16,
    default_throttle: i16,
    current: i16,
    rate: i16,
    stop_rate: i16,
}

impl Drive {
    // TODO this need to be handed to an eeprom config
    pub fn new() -> Self {
        Self {
            state: DriveState::Init,
            timeout: 500.millis(),
            next_timeout: 0,
            throttle: 0,
            default_throttle: 255,
            current: 0,
            rate: 2,
            stop_rate: 10,
        }
    }

    // On a state change or a command the select_biased! will
    // do the thing
    pub async fn run_if(&mut self) {
        poll_fn(|_cx| match self.state {
            DriveState::Init => {
                crate::print!("Initialize the drive");
                // Move to idle
                self.state = DriveState::Idle;
                Poll::Pending
            }
            DriveState::Running => {
                self.update();
                Poll::Ready(())
            }
            DriveState::SoftStop => {
                // If the drive times out, put the throttle to zero
                if self.adjust_throttle(self.stop_rate) {
                    self.state = DriveState::Idle
                }
                Poll::Ready(())
            }
            DriveState::Idle => Poll::Pending,
            DriveState::Error => Poll::Pending,
        })
        .await
    }

    // When the drive is running update the internal variables
    // Once it is finished it will wait for commands in the task.
    fn update(&mut self) {
        if Ticker::ticks() > self.next_timeout {
            crate::print!("drive timeout");
            self.throttle = 0;
            self.state = DriveState::SoftStop;
            return;
        }
        // adjust the throttle
        let _ = self.adjust_throttle(self.rate);
    }

    // get the current closer to the throttle setting
    fn adjust_throttle(&mut self, rate: i16) -> bool {
        if self.current != self.throttle {
            if self.current < self.throttle {
                self.current += rate;
                // to far ?
                if self.current > self.throttle {
                    self.current = self.throttle;
                }
            }
            if self.current > self.throttle {
                self.current -= rate;
                // to far ?
                if self.current < self.throttle {
                    self.current = self.throttle
                }
            }
        }
        crate::print!("current {}", self.current);
        // if we are on target return true
        self.current == self.throttle
    }

    fn enable(&mut self) {
        self.next_timeout = Ticker::ticks() + self.timeout.ticks();
        self.state = DriveState::Running;
    }

    fn disable(&mut self) {
        self.state = DriveState::Idle;
    }

    fn set_command(&mut self, command: DriveCommands) {
        // update as the update is lost in the select_biased
        self.update();
        // only run in running and idle
        if self.state != DriveState::Error {
            match command {
                DriveCommands::Forward => {
                    self.enable();
                    self.throttle = self.default_throttle;
                    // crate::print!("forward")
                }
                DriveCommands::Backwards => {
                    self.enable();
                    self.throttle = -self.default_throttle;
                    // crate::print!("backwards")
                }
                DriveCommands::Left => {
                    self.enable();
                    // crate::print!("left")
                }
                DriveCommands::Right => {
                    self.enable();
                    // crate::print!("right")
                }
                DriveCommands::Stop => {
                    self.disable();
                    self.throttle = 0;
                    // crate::print!("stop");
                }
            }
        }
    }

    // this will wait for commands or run the drive if
    // it needs to.
    pub async fn task(
        &mut self,
        mut commands: channel::Receiver<'_, DriveCommands>,
    ) {
        loop {
            select_biased! {
                // Wait for a command
                command = commands.receive().fuse()=>{
                    self.set_command(command);
                }
                // If it is in process run again
                _ = self.run_if().fuse() => {}
                // Clippy was complaining 
                complete => break
            }
            // Insert a timer to do it again.
            time::delay(10.millis()).await;
        }
    }
}
