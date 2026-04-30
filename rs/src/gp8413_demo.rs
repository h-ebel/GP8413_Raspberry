#![warn(clippy::all)]
#![warn(clippy::cargo)]
#![warn(clippy::pedantic)]
#![deny(unsafe_op_in_unsafe_fn)]
#![allow(clippy::cargo_common_metadata)]

// Minimal example for DFR1073 with GP8413 chip (two-channel DAC with
// configurable output range of 5 V or 10 V and 15-bit resolution)
// Henrik Ebel, Finland, 2026
//
// Disclaimer: This is merely a small study whether it makes
// sense to drive the DAC from a Raspberry Pi 500+ in Rust directly
// using standard write commands instead of some library. It is not
// meant to be a usage-ready piece of code and merely a quickly put
// together feasibility study. Be careful when interfacing with hardware,
// no guarantees that you will not fry anything.

use std::fs::OpenOptions;
use std::io::{self, Write};
use std::os::fd::AsRawFd;

// From linux/i2c-dev.h - a bit ugly that we cannot use I2C_SLAVE directly
// as the Linux i2c-dev header includes something like #define I2C_SLAVE 0x0703
const I2C_SLAVE: libc::c_ulong = 0x0703;

fn main() {
    ////////////////////////////////////////////////////////////////////
    // Remember to enable i2c in the Raspberry Pi's settings.
    // Typical setup: SDA (D on DAC chip) to Raspberry Pi's first hardware
    // i2c port (I2C1) on physical GPIO pin 3, correspondingly SCL (C
    // on DAC chip) to pin 5. Then, minus to ground on pin 6, + to 5 V
    // DC on hardware pin 4; for the latter, i.e., the supplied voltage,
    // also 3.3V should work.
    //
    // User-changeable settings:
    const I2C_DEV: &str = "/dev/i2c-1"; // specify correct device here
    const I2C_ADDR: u16 = 0x58; // standard address if on-device switches A0-A2 are all 0, , can also try to check via i2cdetect -y 1
    const DAC_CHANNEL: u8 = 0; // 0 or 1
    const USE_EXTENDED_VOLTAGE_RANGE: bool = true; // false => 0..5V, true => 0..10V
    const V_OUT: f64 = 3.0; // target voltage in V
    // Remark: the set voltage will be kept, so if you want to have
    // zero voltage, you need to set V_OUT to zero, recompile, and run
    // again. Obviously, this would be different in a productive program.
    ////////////////////////////////////////////////////////////////////

    let voltage_max = if USE_EXTENDED_VOLTAGE_RANGE {
        10.0
    } else {
        5.0
    };

    if (DAC_CHANNEL != 0) && (DAC_CHANNEL != 1) {
        eprintln!(
            "The channel DAC_CHANNEL must be 0 or 1, following the physical numbering on the DAC board."
        );
        return;
    }

    if (V_OUT < 0.0) || (V_OUT > voltage_max) {
        eprintln!("V_OUT out of range, it must be between 0 and {voltage_max} V.");
        return;
    }

    // Open I2C device
    let mut file = match OpenOptions::new().read(true).write(true).open(I2C_DEV) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("open({I2C_DEV}) failed: {e}");
            return;
        }
    };

    // Set I2C slave address
    let fd = file.as_raw_fd();
    let ioctl_result = unsafe { libc::ioctl(fd, I2C_SLAVE, libc::c_ulong::from(I2C_ADDR)) };
    if ioctl_result < 0 {
        eprintln!("ioctl(I2C_SLAVE) failed: {}", io::Error::last_os_error());
        return;
    }

    // Can find out register numbers etc. from https://github.com/DFRobot/DFRobot_GP8XXX
    // GP8413 config register is 0x01 (uint8_t)
    // When writing to the config register, the written byte 0x00 (uint8_t)
    // corresponds to 5V, whereas 0x11 corresponds to 10V.

    // Now, assemble data to write, first byte is the register, here the
    // config register, second byte signifies the chosen output range
    let config: [u8; 2] = [
        0x01,
        if USE_EXTENDED_VOLTAGE_RANGE {
            0x11
        } else {
            0x00
        },
    ];
    // error handling
    if let Err(e) = file.write_all(&config) {
        eprintln!("write(config) failed: {e}");
        return;
    }

    // The DAC has a resolution of 15 bit, meaning we can pick integer
    // values between 0 and 32767, linearly mapping to the output range
    // of 0 to V_max.
    // Based on this, we calculate the integer best fitting to our
    // desired voltage V_out. We first save this to a 16-bit
    // unsigned integer that can certainly hold the value.
    let value_coded: u16 = ((V_OUT / voltage_max) * 32767.0).round() as u16;

    // We learn from the documentation and published code of the GP8413
    // that we do need to send two bytes (16 bit), but that we have to
    // put the 15 bit value to the left of the 16 bit window, so we
    // shift it left by one bit. It seems the GP8413 disregards the last
    // bit.
    let value_coded_shifted: u16 = value_coded << 1;

    // Encode the correct channel number
    let register_to_write: u8 = if DAC_CHANNEL == 0 { 0x02 } else { 0x04 };

    // Line up three bytes: first, the register, then the two bytes
    // containing the shifted 15-bit data. The latter we have to
    // add as two individual bytes (8-bit unsigned integers), where the
    // DAC chip first expects the least significant byte (LSB, i.e.,
    // the right one, corresponding to the smaller numbers), then the
    // other one (MSB).
    // The LSB can be sent by masking off the MSB via & 0xFF,
    // and the MSB can be sent by shifting 8-bit to the right and then
    // masking off similarly. (Remember: 0xFF is 255, the maximal value
    // for 8 bit, i.e., one byte, in unsigned int terms, so the logical
    // and works as a mask).
    let output: [u8; 3] = [
        register_to_write,
        (value_coded_shifted & 0x00FF) as u8,        // LSB first
        ((value_coded_shifted >> 8) & 0x00FF) as u8, // then MSB
    ];
    // Print what we will do
    println!(
        "GP8413 set: DAC_CHANNEL {} = {} V ({} range), I2C address 0x{:X}", // :X formats in upper-case hex
        DAC_CHANNEL,
        V_OUT,
        if USE_EXTENDED_VOLTAGE_RANGE {
            "0-10V"
        } else {
            "0-5V"
        },
        I2C_ADDR
    );
    // Do what we want to do: Write to the i2c device
    if let Err(e) = file.write_all(&output) {
        eprintln!("write(output) failed: {e}");
        return;
    }

    if let Err(e) = file.flush() {
        eprintln!("flush() failed: {e}");
        //return; // very last return not needed in rust
    }
    // File is closed automatically when it goes out of scope.
}
