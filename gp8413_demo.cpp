// Minimal example for DFR1073 with GP8413 chip (two-channel DAC with 
// configurable output range of 5V or 10V and 15bit resolution)
// Henrik Ebel, Finland, 2026

// Disclaimer: This is merely a small study whether it makes 
// sense to drive the DAC from a Raspberry Pi 500+ in C++ directly 
// using standard write commands instead of some library. It is not 
// meant to be a usage-ready piece of code and merely a quickly put 
// together feasibility study. Be careful when interfacing with hardware,
// no guarantees that you will not fry anything.

#include <cmath>
#include <cerrno>
#include <cstring>
#include <fcntl.h>
#include <iostream>
#include <linux/i2c-dev.h>
#include <sys/ioctl.h>
#include <unistd.h>


int main()
{
    ////////////////////////////////////////////////////////////////////
    // Remember to enable i2c in the Raspberry's settings.
    // Typical setup: SDA (D on DAC chip) to Raspberry's first hardware 
    // i2c port (I2C1) on physical GPIO pin 3, correspondlingly SCL (C 
    // on DAC chip) to pin 5. Then, minus to ground on pin 6, + to 5 V 
    // DC on hardware pin 4; for the latter, i.e., the supplied voltage,
    // also 3.3V should work)
    // User-changeable settings:
    constexpr const char* i2c_dev = "/dev/i2c-1"; // specify correct device here
    constexpr int i2c_addr = 0x58; // 0x58 is the standard address (all on-device switches A0-A2 on 0), can also try to check via i2cdetect -y 1
    constexpr int dac_channel  = 0; // 0 or 1
    constexpr bool use_extended_voltage_range = true; // false => 0..5V, true => 0..10V
    constexpr double V_out = 3.000;   // target voltage in V
    // Remark: the set voltage will be kept, so if you want to have 
    // zero voltage, you need to set V_out to zero, recompile, and run
    // again. Obviously, this would be different in a productive program.
    ////////////////////////////////////////////////////////////////////

    constexpr double V_max = use_extended_voltage_range ? 10.0 : 5.0;

    if ((dac_channel != 0) && (dac_channel != 1)) {
        std::cerr << "The channel dac_channel must be 0 or 1, following the physical numbering on the DAC board.\n";
        return 1;
    }

    if (V_out < 0.0 || V_out > V_max) {
        std::cerr << "V_out out of range, it must be between 0 and "
                  << V_max << " V.\n";
        return 1;
    }

    // open i2c device
    int fd = open(i2c_dev, O_RDWR);
    if (fd < 0) {
        std::cerr << "open(" << i2c_dev << ") failed: "
                  << std::strerror(errno) << "\n";
        return 1;
    }

    if (ioctl(fd, I2C_SLAVE, i2c_addr) < 0) {
        std::cerr << "ioctl(I2C_SLAVE) failed: "
                  << std::strerror(errno) << "\n";
        close(fd);
        return 1;
    }
    
    // Can find out register numbers etc. from https://github.com/DFRobot/DFRobot_GP8XXX
    // GP8413 config register is 0x01 (uint8_t)
    // When writing to the config register, the written byte 0x00 (uint8_t) 
    // corresponds to 5V, whereas 0x11 corresponds to 10V.
    
    // Now, assemble data to write, first byte is the register, here the 
    // config register, second byte signifies the chosen output range
    const uint8_t config[2] = {
        0x01,
        static_cast<uint8_t>(use_extended_voltage_range ? 0x11 : 0x00)
    };
    // error handling
    if (write(fd, config, sizeof(config)) != static_cast<ssize_t>(sizeof(config))) {
        std::cerr << "write(config) failed: " << std::strerror(errno) << "\n";
        close(fd);
        return 1;
    }

    // The DAC has a resolution of 15 bit, meaning we can pick integer 
    // values between 0 and 32767, linearly mapping to the output range 
    // of 0 to V_max. 
    // Based on this, we calculate the integer best fitting to our 
    // desired voltage V_out. We first save this to a 16-bit 
    // unsigned integer that can certainly hold the value.
    const uint16_t value_coded =
        static_cast<uint16_t>(std::lround((V_out / V_max) * 32767.0));

    // We learn from the documentation and published code of the GP8413
    // that we do need to send two bytes (16 bit), but that we have to 
    // put the 15 bit value to the left of the 16 bit window, so we
    // shift it left by one bit. It seems the GP8413 disregards the last
    // bit. 
    const uint16_t value_coded_shifted = static_cast<uint16_t>(value_coded << 1);
    // encode the correct channel number
    const uint8_t register_to_write = (dac_channel == 0) ? 0x02 : 0x04; 
    
    // line up three bytes: first, the register, then the two bytes 
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
    const uint8_t output[3] = {
        register_to_write,
        static_cast<uint8_t>(value_coded_shifted & 0xFF),         // LSB first
        static_cast<uint8_t>((value_coded_shifted >> 8) & 0xFF)   // then MSB
    };
    // Print what we will do
    std::cout << "GP8413 set: dac_channel " << dac_channel
              << " = " << V_out << " V"
              << " (" << (use_extended_voltage_range ? "0-10V" : "0-5V") << " range)"
              << ", I2C address 0x" << std::hex << i2c_addr << std::dec << "\n";
    // Do what we want to do: Write to the i2c device
    if (write(fd, output, sizeof(output)) != static_cast<ssize_t>(sizeof(output))) {
        std::cerr << "write(output) failed: " << std::strerror(errno) << "\n";
        close(fd);
        return 1;
    }

    close(fd);
              
    return 0;
}
