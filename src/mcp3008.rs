use esp_idf_hal::spi;
use esp_idf_hal::gpio;
use esp_idf_hal::prelude::*;
use esp_idf_hal::spi::{Master, Spi, SPI3};
use esp_idf_sys::EspError;
use esp_idf_hal::gpio::{OutputPin, InputPin};
use esp_idf_hal::spi::SpiError;
use embedded_hal::prelude::_embedded_hal_blocking_spi_Transfer;
use byteorder::{ByteOrder, BigEndian};

pub struct Reading
{
    pub raw: u32,
    pub voltage: f32,
}

pub struct MCP3008<
    SCLK:OutputPin,
    SDO:OutputPin,
    SDI:InputPin + OutputPin,
    CS:OutputPin>
{
    spi: Master<SPI3, SCLK, SDO, SDI, CS>,
    v_ref: f32,
}


impl<SCLK:OutputPin, SDO:OutputPin, SDI:InputPin + OutputPin, CS:OutputPin> MCP3008<SCLK, SDO, SDI, CS> {
    pub fn new(spi: spi::SPI3,
               clk: SCLK,
               si: SDO,
               so: SDI,
               cs: CS,
               v_ref: f32
    ) -> Result<Self, EspError>
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
