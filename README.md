# AVR async executor and reactor

Did you ever want an arduino to do more than one thing at the same time?

Now you can...

This provides an executor and reactor (timer queue) using the excellent [avr-hal](https://github.com/Rahix/avr-hal) ; thanks @rahix

# Futures

Programming in async is a little different. Please read [this](https://cliffle.com/blog/async-inversion/) at least twice and you may _or_ may _not_ understand.

The important thing is that the tasking is _cooperative_ so all task __must__ have at least one .await so it can yeild control to the next task.

It does take a little while to get intothe groove, but there are two points to remember. 

1. State needs to be setup __inside__ the task.
2. Communications between tasks is done with channels and queues.
3. The borrow checker is not your friend.

# Inspiration

Been writing rust for Arduino for a while and [this](https://www.youtube.com/watch?v=wni5h5vIPhU) video appeared in the feed and got me started. 

I had been looking at  [lilos](https://github.com/cbiffle/lilos) for a while but it is targeted at ARM and did not translate to 8bit that well. So a rewrite was in order.

# Hazards

If you include a panic and attempt to get symbols and a full panic drop , you will run out of ram on the atmega32p , weird crashes will happen, don't do that.

# Changes
 
You may need to change the serial port in .cargo/config.toml and console.py to talk to your device.