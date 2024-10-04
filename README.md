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

# Information

```
cargo install cargo-binutils
```


Some commands to get info 

```
cargo size --release -- -A

.data                         1362  0x800100
.text                        11282       0x0
.bss                           111  0x800652
.note.gnu.avr.deviceinfo        64       0x0
.debug_info                   1524       0x0
.debug_abbrev                 1442       0x0
.debug_line                     26       0x0
.debug_str                     520       0x0
Total                        16331
```

Look like lots of static strings taking up ram. 

``` cargo install cargo-bloat ```

cargo bloat

```
File  .text    Size           Crate Name
0.3%   8.9%  1.2KiB  newdrive_async newdrive_async::__avr_device_rt_main
0.3%   8.7%  1.2KiB       [Unknown] __vector_16
0.3%   8.6%  1.2KiB  newdrive_async newdrive_async::show_time::{{closure}}
0.3%   7.7%  1.0KiB  newdrive_async newdrive_async::serial::SerialIncoming::task::{{closure}}
0.2%   5.8%    794B  newdrive_async newdrive_async::drive::Drive::task::{{closure}}
0.2%   5.8%    792B  newdrive_async newdrive_async::make_commands::{{closure}}
0.2%   5.3%    728B  newdrive_async newdrive_async::executor::run_tasks
0.1%   4.1%    568B  newdrive_async newdrive_async::drive::Drive::adjust_throttle
```

look at the code size, because it is harvard architecture this matters less.
