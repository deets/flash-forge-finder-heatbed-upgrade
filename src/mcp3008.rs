use esp_idf_hal::spi;
use esp_idf_hal::gpio;
use esp_idf_hal::prelude::*;
use esp_idf_sys::EspError;
use esp_idf_hal::spi::SpiError;
use embedded_hal::prelude::_embedded_hal_blocking_spi_Transfer;
use byteorder::{ByteOrder, BigEndian};

pub struct Reading
{
    pub raw: u32,
    pub voltage: f32,
}

pub struct MCP3008
{
    spi: spi::Master<
            spi::SPI3,
        gpio::Gpio12<gpio::Unknown>,
        gpio::Gpio11<gpio::Unknown>,
        gpio::Gpio13<gpio::Unknown>,
        gpio::Gpio15<gpio::Unknown>>,
    v_ref: f32,
}


impl MCP3008 {
    pub fn new(spi: spi::SPI3,
               clk: gpio::Gpio12<gpio::Unknown>,
               si: gpio::Gpio11<gpio::Unknown>,
               so: gpio::Gpio13<gpio::Unknown>,
               cs: gpio::Gpio15<gpio::Unknown>,
               v_ref: f32
    ) -> Result<MCP3008, EspError>
    {
        let config = <spi::config::Config as Default>::default().baudrate(5.MHz().into());
        let pins = spi::Pins {
                sclk: clk,
                sdo: si,
                sdi: Some(so),
                cs: Some(cs)
            };

        let di = spi::Master::<spi::SPI3, _, _, _, _>::new(
            spi,
            pins,
            config
            )?;
        Ok(MCP3008{spi: di, v_ref: v_ref})
    }

    pub fn read(&mut self, channel: u8) -> Result<Reading, SpiError> {
        // Start bit, Single ended, channel number
        let command = ((0b11000 | channel) as u16) << 11;
        // We need 17 bits in one transfer
        // but to ease the bit shifting, I'll pump out
        // 32
        let mut buf: [u8;4] = [0, 0, 0, 0];
        BigEndian::write_u16(&mut buf, command);
        self.spi.transfer(&mut buf)?;
        let raw = BigEndian::read_u32(&buf) >> 15  & 0x03ff;
        let voltage = raw as f32 / 1023.0 * self.v_ref;
        Ok(Reading{raw, voltage})
    }
}
