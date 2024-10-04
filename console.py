#!/usr/bin/python3
" Serial interface for uploading boneless firmware"


from serial.tools.miniterm import Miniterm
import serial
import time

the_port = "/dev/serial/by-id/usb-FTDI_FT232R_USB_UART_A700eCzo-if00-port0"

class Console:
    def __init__(self, port=the_port,baud=115200):
        self.port = port
        self.baud = baud
        self.ser = serial.serial_for_url(
            port, baud
        )
        # self.ser.dtr = 0

    def attach(self):
        import argparse

        parser = argparse.ArgumentParser()
        parser.add_argument("-l", "--list", action="store_true")
        parser.add_argument("-v", "--verbose", action="store_true")

        args = parser.parse_args()
        term = Miniterm(self.ser)
        term.set_rx_encoding("utf-8")
        term.set_tx_encoding("utf-8")
        term.exit_character = "\x1d"
        print("Attach console")
        term.start()
        term.join(True)

if __name__ == "__main__":
    c = Console()
    c.attach()
